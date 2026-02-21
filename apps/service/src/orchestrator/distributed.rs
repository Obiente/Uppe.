//! Distributed orchestration coordinator for public monitors.
//!
//! This module coordinates monitoring checks across peers using consensus-based
//! scheduling to prevent DDoS and ensure fair distribution.

use anyhow::Result;
use peerup::crypto::KeyPair;
use peerup::distributed::{
    ConsensusManager, OrchestrationSchedule, PublicMonitorGroup,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::database::models::Monitor;
use crate::database::Database;
use crate::monitoring::validation::{validate_check_interval, validate_monitor_target};

/// Distributed orchestration coordinator
pub struct DistributedOrchestrator {
    /// Database for storing monitor groups and votes
    database: Arc<dyn Database>,

    /// Consensus manager for orchestration votes
    consensus: Arc<ConsensusManager>,

    /// Local peer ID
    peer_id: String,

    /// Ed25519 keypair for signing votes
    keypair: Arc<KeyPair>,

    /// Cache of public monitor groups
    groups: Arc<RwLock<HashMap<String, PublicMonitorGroup>>>,

    /// P2P network handle
    p2p_network: Arc<crate::p2p::P2PNetwork>,
}

impl DistributedOrchestrator {
    pub fn new(
        database: Arc<dyn Database>,
        peer_id: String,
        keypair: Arc<KeyPair>,
        p2p_network: Arc<crate::p2p::P2PNetwork>,
    ) -> Self {
        Self {
            database,
            consensus: Arc::new(ConsensusManager::new()),
            peer_id,
            keypair,
            groups: Arc::new(RwLock::new(HashMap::new())),
            p2p_network,
        }
    }

    /// Initialize orchestrator - load existing groups from database
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing distributed orchestrator");

        // Load all enabled public monitors from database
        let all_monitors = self.database.get_enabled_monitors().await?;
        let public_monitors: Vec<_> = all_monitors
            .into_iter()
            .filter(|m| m.is_public())
            .collect();

        if public_monitors.is_empty() {
            info!("No public monitors found during initialization");
            return Ok(());
        }

        info!("Found {} public monitors, organizing into groups", public_monitors.len());

        // Group monitors by domain
        let mut domain_monitors: HashMap<String, Vec<Monitor>> = HashMap::new();
        for monitor in public_monitors {
            if let Some(domain) = &monitor.public_domain {
                domain_monitors
                    .entry(domain.clone())
                    .or_insert_with(Vec::new)
                    .push(monitor);
            }
        }

        // Create or update groups
        for (domain, _monitors) in domain_monitors {
            let display_name = format!("Public Monitors - {}", domain);
            let group = PublicMonitorGroup::new(domain.clone(), display_name, self.peer_id.clone());

            // Try to load existing group from database
            match self.database.get_public_monitor_group(&domain).await {
                Ok(Some(existing_group)) => {
                    info!("Loaded existing public monitor group: {}", domain);
                    self.groups.write().await.insert(domain, existing_group);
                }
                Ok(None) => {
                    info!("Creating new public monitor group: {}", domain);
                    self.groups.write().await.insert(domain, group);
                }
                Err(e) => {
                    warn!("Failed to load group {} from database: {}", domain, e);
                    self.groups.write().await.insert(domain, group);
                }
            }
        }

        Ok(())
    }

    /// Handle a new monitor being created
    pub async fn handle_new_monitor(&self, monitor: &Monitor) -> Result<()> {
        if !monitor.is_public() {
            // Private/Internal monitors handled by PrivateMonitorOrchestrator or owner directly
            return Ok(());
        }

        let domain = monitor
            .public_domain
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Public monitor missing domain"))?;

        info!("Handling new public monitor for domain: {}", domain);

        // **SECURITY**: Validate monitor target before adding to public group
        validate_monitor_target(&monitor.target, &monitor.check_type)
            .map_err(|e| anyhow::anyhow!("Monitor validation failed: {}", e))?;

        // **SECURITY**: Validate check interval
        validate_check_interval(monitor.interval_seconds)
            .map_err(|e| anyhow::anyhow!("Interval validation failed: {}", e))?;

        // Check if group already exists
        let mut groups = self.groups.write().await;

        if let Some(existing_group) = groups.get_mut(domain) {
            // Join existing group
            info!("Joining existing monitor group for {}", domain);
            existing_group.add_peer(self.peer_id.clone());

            // Broadcast join message to network
            self.broadcast_join(domain).await?;

            // Propose updated schedule via consensus
            self.propose_schedule_update(domain, existing_group.schedule.clone())
                .await?;
        } else {
            // Create new group
            info!("Creating new public monitor group for {}", domain);

            let display_name = monitor
                .public_display_name
                .clone()
                .unwrap_or_else(|| domain.clone());

            let group = PublicMonitorGroup::new(domain.clone(), display_name, self.peer_id.clone());

            // Broadcast announcement to network
            self.broadcast_announcement(&group).await?;

            // Store group
            groups.insert(domain.clone(), group.clone());

            // Initialize consensus for this domain
            self.consensus
                .get_or_create(domain, group.schedule.clone())
                .await;
        }

        Ok(())
    }

    /// Check if this peer should perform a check for a monitor
    pub async fn should_check_now(&self, monitor: &Monitor) -> bool {
        if !monitor.is_public() {
            return true; // Private monitors always check normally
        }

        let domain = match &monitor.public_domain {
            Some(d) => d,
            None => return false,
        };

        // Query consensus manager
        self.consensus
            .should_check_now(domain, &self.peer_id)
            .await
    }

    /// Mark that a check was completed
    pub async fn mark_check_completed(&self, monitor: &Monitor) -> Result<()> {
        if !monitor.is_public() {
            return Ok(()); // Private monitors don't need tracking
        }

        let domain = match &monitor.public_domain {
            Some(d) => d,
            None => return Ok(()),
        };

        debug!("Marking check completed for {}", domain);

        // Update consensus state
        self.consensus
            .mark_check_completed(domain, &self.peer_id)
            .await;

        // Update group check count
        if let Some(group) = self.groups.write().await.get_mut(domain) {
            group.mark_check_completed(&self.peer_id);
        }

        Ok(())
    }

    /// Propose a schedule update (triggers consensus vote)
    async fn propose_schedule_update(
        &self,
        domain: &str,
        schedule: OrchestrationSchedule,
    ) -> Result<()> {
        // Create vote payload for signing
        let timestamp = chrono::Utc::now().timestamp();
        let vote_data = format!(
            "{}:{}:{}",
            domain,
            serde_json::to_string(&schedule)?,
            timestamp
        );

        // Sign with Ed25519 using the node's keypair
        let signature_bytes = peerup::crypto::sign_bytes(vote_data.as_bytes(), &self.keypair);
        let signature = hex::encode(&signature_bytes);

        let vote = peerup::distributed::OrchestrationVote {
            domain: domain.to_string(),
            schedule,
            voter_peer_id: self.peer_id.clone(),
            signature,
            public_key: Some(self.keypair.public_key_bytes().to_vec()),
            timestamp,
        };

        // Cast local vote
        self.consensus
            .cast_vote(domain, vote.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to cast vote: {}", e))?;

        // Broadcast vote to network
        self.broadcast_vote(vote).await?;

        Ok(())
    }

    /// Handle incoming vote from another peer
    pub async fn handle_vote(&self, vote: peerup::distributed::OrchestrationVote) -> Result<()> {
        debug!(
            "Received orchestration vote from {} for {}",
            vote.voter_peer_id, vote.domain
        );

        // Cast vote in consensus manager
        self.consensus
            .cast_vote(&vote.domain, vote.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to cast vote: {}", e))?;

        // Check if consensus reached
        let groups = self.groups.read().await;
        if let Some(group) = groups.get(&vote.domain) {
            let total_peers = group.participating_peers.len();

            if let Some(new_schedule) = self
                .consensus
                .check_consensus(&vote.domain, total_peers)
                .await
            {
                info!(
                    "Consensus reached for {} - updating schedule",
                    vote.domain
                );

                // Update local group schedule
                drop(groups); // Release read lock
                if let Some(group) = self.groups.write().await.get_mut(&vote.domain) {
                    group.schedule = new_schedule.clone();
                    group.last_updated = chrono::Utc::now().timestamp();

                    // Persist to database
                    if let Err(e) = self.database.save_public_monitor_group(group).await {
                        warn!("Failed to persist group {} to database: {}", vote.domain, e);
                    } else {
                        debug!("Persisted updated schedule for group {} to database", vote.domain);
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle peer joining a public monitor group
    pub async fn handle_peer_join(&self, domain: String, peer_id: String) -> Result<()> {
        info!("Peer {} joining monitor group {}", peer_id, domain);

        let mut groups = self.groups.write().await;

        if let Some(group) = groups.get_mut(&domain) {
            group.add_peer(peer_id.clone());

            // Trigger schedule rebalance via consensus
            let new_schedule = group.schedule.clone();
            drop(groups); // Release lock

            self.propose_schedule_update(&domain, new_schedule).await?;
        } else {
            warn!(
                "Received join request for unknown monitor group: {}",
                domain
            );
        }

        Ok(())
    }

    /// Handle peer leaving a public monitor group
    pub async fn handle_peer_leave(&self, domain: String, peer_id: String) -> Result<()> {
        info!("Peer {} leaving monitor group {}", peer_id, domain);

        let mut groups = self.groups.write().await;

        if let Some(group) = groups.get_mut(&domain) {
            group.remove_peer(&peer_id);

            if group.participating_peers.is_empty() {
                // Last peer left, remove group
                groups.remove(&domain);
                info!("Monitor group {} removed (no peers left)", domain);
            } else {
                // Trigger schedule rebalance
                let new_schedule = group.schedule.clone();
                drop(groups);

                self.propose_schedule_update(&domain, new_schedule).await?;
            }
        }

        Ok(())
    }

    /// Broadcast announcement of new public monitor group
    async fn broadcast_announcement(&self, group: &PublicMonitorGroup) -> Result<()> {
        let message = peerup::distributed::PublicMonitorMessage::Announce {
            domain: group.domain.clone(),
            display_name: group.display_name.clone(),
            creator_peer_id: self.peer_id.clone(),
        };

        debug!("Broadcasting monitor group announcement for domain: {}", group.domain);

        let topic_name = format!("/uppe/public-monitors/{}", group.domain);
        let data = serde_json::to_vec(&message)?;

        self.p2p_network
            .send_command(crate::p2p::messages::P2PCommand::PublishToTopic {
                topic: topic_name,
                data,
            })
            .await?;

        Ok(())
    }

    /// Broadcast join message
    async fn broadcast_join(&self, domain: &str) -> Result<()> {
        let message = peerup::distributed::PublicMonitorMessage::Join {
            domain: domain.to_string(),
            peer_id: self.peer_id.clone(),
        };

        debug!("Broadcasting join message for domain: {}", domain);

        let topic_name = format!("/uppe/public-monitors/{}", domain);
        let data = serde_json::to_vec(&message)?;

        self.p2p_network
            .send_command(crate::p2p::messages::P2PCommand::PublishToTopic {
                topic: topic_name,
                data,
            })
            .await?;

        Ok(())
    }

    /// Broadcast orchestration vote
    async fn broadcast_vote(&self, vote: peerup::distributed::OrchestrationVote) -> Result<()> {
        let message = peerup::distributed::PublicMonitorMessage::ScheduleUpdate {
            domain: vote.domain.clone(),
            schedule: vote.schedule.clone(),
        };

        debug!("Broadcasting orchestration vote for domain: {}", vote.domain);

        let topic_name = format!("/uppe/public-monitors/{}", vote.domain);
        let data = serde_json::to_vec(&message)?;

        self.p2p_network
            .send_command(crate::p2p::messages::P2PCommand::PublishToTopic {
                topic: topic_name,
                data,
            })
            .await?;

        Ok(())
    }

    /// Get all public monitor groups
    pub async fn get_all_groups(&self) -> Vec<PublicMonitorGroup> {
        self.groups.read().await.values().cloned().collect()
    }

    /// Get group for specific domain
    pub async fn get_group(&self, domain: &str) -> Option<PublicMonitorGroup> {
        self.groups.read().await.get(domain).cloned()
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_public_monitor_grouping() {
        // TODO: Add integration tests
    }
}
