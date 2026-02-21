//! Private monitor orchestration - peer-assisted monitoring with encryption.
//!
//! This module coordinates monitoring of private services by assigning helper peers,
//! encrypting results for the owner, and managing result synchronization.

use anyhow::{anyhow, Result};
use peerup::distributed::metadata::PeerMetadataDHT;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use crate::crypto::{decrypt_result_for_owner, EncryptedResult};
use crate::database::models::Monitor;
use crate::database::Database;

/// Maximum number of assignments a single helper can have
const MAX_ASSIGNMENTS_PER_HELPER: usize = 10;

/// Timeout waiting for helper acceptance (30 seconds)
const ASSIGNMENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Helper health status
#[derive(Debug, Clone)]
struct HelperStatus {
    /// Peer ID
    peer_id: String,
    /// Last result received from this helper
    last_seen: SystemTime,
    /// Number of assignments
    assignment_count: usize,
    /// Whether helper confirmed the assignment
    confirmed: bool,
}

/// Private monitor orchestration state
pub struct PrivateMonitorOrchestrator {
    /// Database for storing encrypted results
    database: Arc<dyn Database>,

    /// Local peer ID
    peer_id: String,

    /// Owner's X25519 public key (for encryption)
    owner_pubkey: [u8; 32],

    /// Helper peer assignments: monitor_uuid -> Vec<helper_peer_ids>
    assignments: Arc<RwLock<HashMap<String, Vec<String>>>>,

    /// Helper status tracking: helper_peer_id -> status
    helper_status: Arc<RwLock<HashMap<String, HelperStatus>>>,

    /// Pending assignments awaiting confirmation: monitor_uuid -> (helper_peer_ids, assigned_at)
    pending_assignments: Arc<RwLock<HashMap<String, (Vec<String>, SystemTime)>>>,

    /// P2P network handle
    p2p_network: Arc<crate::p2p::P2PNetwork>,

    /// Connected peer IDs (for assignment)
    connected_peers: Arc<RwLock<HashSet<String>>>,

    /// Synced DHT keys: tracks which encrypted result batches have been decrypted
    /// Format: "{monitor_uuid}-{batch_index}" -> timestamp of last successful sync
    synced_keys: Arc<RwLock<HashMap<String, i64>>>,

    /// Last time owner sync was performed (for rate limiting)
    last_sync_time: Arc<RwLock<Option<i64>>>,

    /// Peer trust metadata for scoring helper selection
    peer_metadata: Arc<PeerMetadataDHT>,
}

impl PrivateMonitorOrchestrator {
    pub fn new(
        database: Arc<dyn Database>,
        peer_id: String,
        owner_pubkey: [u8; 32],
        p2p_network: Arc<crate::p2p::P2PNetwork>,
    ) -> Self {
        Self {
            database,
            peer_id,
            owner_pubkey,
            assignments: Arc::new(RwLock::new(HashMap::new())),
            helper_status: Arc::new(RwLock::new(HashMap::new())),
            pending_assignments: Arc::new(RwLock::new(HashMap::new())),
            p2p_network,
            connected_peers: Arc::new(RwLock::new(HashSet::new())),
            synced_keys: Arc::new(RwLock::new(HashMap::new())),
            last_sync_time: Arc::new(RwLock::new(None)),
            peer_metadata: Arc::new(PeerMetadataDHT::new("uppe".to_string())),
        }
    }

    /// Initialize orchestrator - assign helpers for existing monitors
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing private monitor orchestrator");

        // Load all private monitors from database
        let all_monitors = self.database.get_enabled_monitors().await?;
        let mut private_monitors: Vec<_> = all_monitors
            .into_iter()
            .filter(|m| m.is_private())
            .collect();

        if private_monitors.is_empty() {
            info!("No private monitors found during initialization");
            return Ok(());
        }

        info!("Found {} private monitors, assigning helper peers", private_monitors.len());

        // Ensure all private monitors have an owner_peer_id (for backward compatibility)
        for monitor in &mut private_monitors {
            if monitor.owner_peer_id.is_none() {
                monitor.owner_peer_id = Some(self.peer_id.clone());
                // Persist the update
                if let Err(e) = self.database.save_monitor(monitor).await {
                    warn!("Failed to save monitor with owner_peer_id: {}", e);
                }
            }
        }

        // Assign helper peers for each private monitor
        for monitor in private_monitors {
            match self.assign_helper_peers(&monitor).await {
                Ok(helpers) => {
                    if !helpers.is_empty() {
                        // Store assignments
                        self.assignments.write().await.insert(
                            monitor.uuid.to_string(),
                            helpers.clone(),
                        );

                        // Notify each helper
                        for helper_id in helpers {
                            if let Err(e) = self.notify_helper_peer(&helper_id, &monitor).await {
                                warn!("Failed to notify helper peer {}: {}", helper_id, e);
                            }
                        }
                    } else {
                        warn!("No helpers available for private monitor {}", monitor.uuid);
                    }
                }
                Err(e) => {
                    warn!("Failed to assign helpers for monitor {}: {}", monitor.uuid, e);
                }
            }
        }

        Ok(())
    }

    /// Check if owner sync should run (rate-limited to every 12 hours)
    pub async fn should_sync_owner_results(&self) -> bool {
        let last_sync = self.last_sync_time.read().await;
        match *last_sync {
            Some(timestamp) => {
                let now = chrono::Utc::now().timestamp();
                // Allow sync if 12+ hours have passed
                now - timestamp > 12 * 3600
            }
            None => true, // Never synced before
        }
    }

    /// Mark that owner sync was just performed
    async fn mark_sync_completed(&self) {
        *self.last_sync_time.write().await = Some(chrono::Utc::now().timestamp());
    }

    /// Handle a new private monitor being created
    pub async fn handle_new_monitor(&self, monitor: &Monitor) -> Result<()> {
        if !monitor.is_private() {
            return Ok(()); // Only handle private monitors
        }

        let monitor_uuid = monitor.uuid.to_string();
        info!("Handling new private monitor: {}", monitor_uuid);

        // Assign helper peers
        let helpers = self.assign_helper_peers(monitor).await?;

        if helpers.is_empty() {
            warn!(
                "No helper peers available for private monitor {}",
                monitor_uuid
            );
            return Ok(());
        }

        // Store as PENDING assignments
        self.pending_assignments
            .write()
            .await
            .insert(monitor_uuid.clone(), (helpers.clone(), SystemTime::now()));

        // Notify helper peers
        for helper_id in &helpers {
            self.notify_helper_peer(helper_id, monitor).await?;
        }

        info!(
            "Assigned {} helper peers to monitor {} (pending confirmation)",
            helpers.len(),
            monitor_uuid
        );

        Ok(())
    }

    /// Assign helper peers to monitor a private service
    ///
    /// Selection criteria (in priority order):
    /// 1. Not the owner
    /// 2. Currently online
    /// 3. Higher trust score preferred (based on successful operations)
    /// 4. Shuffle among equal-trust candidates to prevent gaming
    async fn assign_helper_peers(&self, monitor: &Monitor) -> Result<Vec<String>> {
        let connected = self.connected_peers.read().await;

        // Filter out owner
        let owner_id = monitor
            .owner_peer_id
            .as_ref()
            .ok_or_else(|| anyhow!("Private monitor missing owner_peer_id"))?;

        let mut candidates: Vec<String> = connected
            .iter()
            .filter(|peer_id| *peer_id != owner_id)
            .cloned()
            .collect();

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // Score candidates by trust, then shuffle within equal-trust tiers
        let mut scored: Vec<(String, f64)> = Vec::with_capacity(candidates.len());
        for peer_id in &candidates {
            let trust = self.peer_metadata.get_trust_score(peer_id).await;
            scored.push((peer_id.clone(), trust.score));
        }

        // Sort descending by trust score (highest first)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Shuffle candidates that share the same trust tier (within 0.1 of each other)
        // to prevent deterministic selection among equally-trusted peers
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let mut i = 0;
        while i < scored.len() {
            let tier_score = scored[i].1;
            let mut j = i + 1;
            while j < scored.len() && (scored[j].1 - tier_score).abs() < 0.1 {
                j += 1;
            }
            scored[i..j].shuffle(&mut rng);
            i = j;
        }

        candidates = scored.into_iter().map(|(id, _)| id).collect();

        // Select 3-5 helpers (or fewer if not enough peers)
        let helper_count = std::cmp::min(candidates.len(), 5);
        let helper_count = std::cmp::max(helper_count, std::cmp::min(candidates.len(), 3));

        Ok(candidates[..helper_count].to_vec())
    }

    /// Notify a helper peer to start monitoring
    async fn notify_helper_peer(&self, helper_id: &str, monitor: &Monitor) -> Result<()> {
        debug!("Notifying peer {} to help monitor {}", helper_id, monitor.uuid);

        // Create helper assignment request with correct field names
        let request = crate::p2p::messages::HelperAssignmentRequest {
            monitor_uuid: monitor.uuid.to_string(),
            target: monitor.target.clone(),
            check_type: monitor.check_type.clone(),
            interval_seconds: monitor.interval_seconds,
            timeout_seconds: monitor.timeout_seconds,
            owner_peer_id: self.peer_id.clone(),
            owner_public_key: self.owner_pubkey,
            helper_peer_id: helper_id.to_string(),
            assigned_at: chrono::Utc::now().timestamp(),
        };

        // Send via P2P network
        self.p2p_network
            .send_command(crate::p2p::messages::P2PCommand::AssignHelper {
                helper_peer_id: helper_id.to_string(),
                request,
            })
            .await?;

        debug!("Sent helper assignment request to peer {}", helper_id);
        Ok(())
    }

    /// Handle an encrypted result from a helper peer
    pub async fn handle_encrypted_result(&self, result: EncryptedResult) -> Result<()> {
        info!(
            "Received encrypted result for monitor {} from peer {}",
            result.monitor_uuid, result.helper_peer_id
        );

        // For encrypted results, we just log the receipt and distribute them
        // The owner will decrypt them during Owner Sync
        debug!(
            "Encrypted result from helper {}: monitor={}, encrypted_at={}",
            result.helper_peer_id, result.monitor_uuid, result.encrypted_at
        );

        // Gossip encrypted result for redundancy
        if let Err(e) = self.gossip_encrypted_result(&result).await {
            warn!("Failed to gossip encrypted result: {}", e);
        }

        // Store in DHT for owner sync
        if let Err(e) = self.store_in_dht_batch(&result).await {
            warn!("Failed to store encrypted result in DHT: {}", e);
        }

        Ok(())
    }

    /// Gossip encrypted result to network for redundancy
    async fn gossip_encrypted_result(&self, result: &EncryptedResult) -> Result<()> {
        debug!(
            "Gossiping encrypted result for owner {}",
            result.owner_peer_id
        );

        // Publish via P2P network to private results topic
        self.p2p_network
            .send_command(crate::p2p::messages::P2PCommand::PublishEncryptedResult(
                result.clone(),
            ))
            .await?;

        debug!("Published encrypted result to GossipSub topic");
        Ok(())
    }

    /// Store encrypted result in DHT batch
    async fn store_in_dht_batch(&self, result: &EncryptedResult) -> Result<()> {
        debug!("Storing encrypted result in DHT");

        // Create DHT key: /uppe/private/{owner_peer_id}/{monitor_uuid}/{timestamp}
        let key = format!(
            "/uppe/private/{}/{}/{}",
            result.owner_peer_id,
            result.monitor_uuid,
            result.encrypted_at
        );

        // Serialize result to bytes
        let value = serde_json::to_vec(result)?;

        // Store in DHT
        self.p2p_network
            .send_command(crate::p2p::messages::P2PCommand::PublishDHTRecord {
                key: key.as_bytes().to_vec(),
                value,
            })
            .await?;

        debug!("Stored encrypted result in DHT at key: {}", key);
        Ok(())
    }

    /// Check if this peer should perform a check for a private monitor
    ///
    /// Returns true if:
    /// - Peer is the owner, OR
    /// - Peer is assigned as a helper
    pub async fn should_check_now(&self, monitor: &Monitor) -> bool {
        if !monitor.is_private() {
            return false;
        }

        let monitor_uuid = monitor.uuid.to_string();

        // Owner always checks
        if let Some(owner_id) = &monitor.owner_peer_id {
            if owner_id == &self.peer_id {
                return true;
            }
        }

        // Check if assigned as helper
        if let Some(helpers) = self.assignments.read().await.get(&monitor_uuid) {
            return helpers.contains(&self.peer_id);
        }

        false
    }

    /// Update list of connected peers
    pub async fn update_connected_peers(&self, peers: HashSet<String>) {
        *self.connected_peers.write().await = peers;
    }

    /// Handle peer connection
    pub async fn handle_peer_connected(&self, peer_id: String) {
        self.connected_peers.write().await.insert(peer_id);
    }

    /// Handle peer disconnection
    pub async fn handle_peer_disconnected(&self, peer_id: &str) {
        self.connected_peers.write().await.remove(peer_id);

        // Rebalance assignments if a helper went offline
        self.rebalance_assignments_for_offline_peer(peer_id).await;
    }

    /// Rebalance monitor assignments when a peer goes offline
    async fn rebalance_assignments_for_offline_peer(&self, offline_peer_id: &str) {
        // Collect monitors that need rebalancing
        let monitors_to_rebalance: Vec<String> = {
            let mut assignments = self.assignments.write().await;
            let mut needs_rebalance = Vec::new();

            for (monitor_uuid, helpers) in assignments.iter_mut() {
                if helpers.contains(&offline_peer_id.to_string()) {
                    helpers.retain(|id| id != offline_peer_id);
                    info!(
                        "Peer {} went offline, removed from monitor {} ({} helpers remaining)",
                        offline_peer_id, monitor_uuid, helpers.len()
                    );
                    // Only rebalance if we're below minimum (3 helpers)
                    if helpers.len() < 3 {
                        needs_rebalance.push(monitor_uuid.clone());
                    }
                }
            }
            needs_rebalance
        };

        // Remove stale helper status
        self.helper_status.write().await.remove(offline_peer_id);

        // Try to assign replacement helpers
        for monitor_uuid in monitors_to_rebalance {
            if let Ok(uuid) = uuid::Uuid::parse_str(&monitor_uuid) {
                match self.database.get_monitor_by_uuid(uuid).await {
                    Ok(Some(monitor)) if monitor.is_private() => {
                        info!("Finding replacement helper for monitor {}", monitor_uuid);
                        if let Err(e) = self.handle_new_monitor(&monitor).await {
                            warn!("Failed to find replacement helper for {}: {}", monitor_uuid, e);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Sync encrypted results from DHT for the owner
    ///
    /// This method:
    /// 1. Queries DHT for encrypted result batches stored by helper peers
    /// 2. Decrypts each batch using the owner's private key
    /// 3. Stores results permanently in the database
    /// 4. Tracks synced keys to avoid redundant decryption
    ///
    /// Should be called:
    /// - On startup to recover missed results while offline
    /// - Periodically (e.g., every 12 hours) to sync new results
    pub async fn sync_owner_results_from_dht(&self, owner_secret_key: &[u8; 32]) -> Result<()> {
        info!("Syncing encrypted results from DHT for owner");

        // Get assignments to know which monitors to sync
        let assignments = self.assignments.read().await;

        let mut total_decrypted = 0;
        for (monitor_uuid, _helpers) in assignments.iter() {
            // Generate DHT keys for this monitor's encrypted result batches
            // Key format: "uppe-private-results-{monitor_uuid}-{batch_index}"
            // We try multiple batch indexes to find all stored batches
            for batch_index in 0..100 {
                let batch_key = format!(
                    "{}-{}",
                    monitor_uuid, batch_index
                );
                let dht_key = format!("uppe-private-results-{}", batch_key);

                // Check if we've already synced this batch
                if self.synced_keys.read().await.contains_key(&batch_key) {
                    debug!(
                        "Batch {} already synced, skipping",
                        batch_key
                    );
                    continue;
                }

                debug!(
                    "Querying DHT for encrypted results: {}",
                    dht_key
                );

                // Query DHT for this batch (now with request-response support)
                match self.p2p_network.get_dht_record(&dht_key).await {
                    Ok(Some(batch_bytes)) => {
                        // Try to deserialize and decrypt the batch
                        match self.decrypt_result_batch(batch_bytes, owner_secret_key) {
                            Ok(results) => {
                                let batch_size = results.len();
                                info!(
                                    "Decrypted {} results from batch {} for monitor {}",
                                    batch_size,
                                    batch_index,
                                    monitor_uuid
                                );

                                // Store decrypted results permanently
                                for result in results {
                                    if let Err(e) = self.database.save_result(&result).await {
                                        warn!(
                                            "Failed to store synced result for monitor {}: {}",
                                            monitor_uuid, e
                                        );
                                    }
                                }

                                // Mark this batch as synced
                                self.synced_keys.write().await.insert(
                                    batch_key,
                                    chrono::Utc::now().timestamp(),
                                );

                                total_decrypted += batch_size;
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to decrypt batch {} for monitor {}: {}",
                                    batch_index, monitor_uuid, e
                                );
                                // Continue to next batch
                            }
                        }
                    }
                    Ok(None) => {
                        // Key not found - we've reached the end of batches for this monitor
                        debug!(
                            "No DHT record found for batch {} of monitor {}",
                            batch_index, monitor_uuid
                        );
                        break;
                    }
                    Err(e) => {
                        warn!(
                            "DHT query error for batch {} of monitor {}: {}",
                            batch_index, monitor_uuid, e
                        );
                        // Continue to next batch on timeout or other errors
                    }
                }
            }
        }

        // Mark sync as completed
        self.mark_sync_completed().await;

        info!(
            "Completed syncing encrypted results from DHT ({} results total)",
            total_decrypted
        );
        Ok(())
    }

    /// Decrypt an encrypted result batch
    fn decrypt_result_batch(
        &self,
        batch_bytes: Vec<u8>,
        owner_secret_key: &[u8; 32],
    ) -> Result<Vec<crate::monitoring::types::CheckResult>> {
        use crate::crypto::EncryptedResult;

        // Deserialize the batch
        let batch: Vec<EncryptedResult> = serde_json::from_slice(&batch_bytes)?;

        // Decrypt each result in the batch
        let mut decrypted_results = Vec::new();
        for encrypted_result in batch {
            match decrypt_result_for_owner(&encrypted_result, owner_secret_key) {
                Ok(result) => {
                    decrypted_results.push(result);
                }
                Err(e) => {
                    warn!(
                        "Failed to decrypt individual result in batch: {}",
                        e
                    );
                    // Continue with other results in batch
                }
            }
        }

        Ok(decrypted_results)
    }

    /// Handle helper assignment acceptance
    pub async fn handle_helper_accepted(&self, monitor_uuid: &str, helper_peer_id: &str) -> Result<()> {
        info!("Helper {} accepted assignment for monitor {}", helper_peer_id, monitor_uuid);

        // Update helper status
        let mut status_map = self.helper_status.write().await;
        if let Some(status) = status_map.get_mut(helper_peer_id) {
            status.confirmed = true;
            status.assignment_count += 1;
        } else {
            status_map.insert(
                helper_peer_id.to_string(),
                HelperStatus {
                    peer_id: helper_peer_id.to_string(),
                    last_seen: SystemTime::now(),
                    assignment_count: 1,
                    confirmed: true,
                },
            );
        }
        drop(status_map);

        // Move from pending to confirmed assignments
        let mut pending = self.pending_assignments.write().await;
        if let Some((_helpers, _)) = pending.remove(monitor_uuid) {
            // Keep only this helper in the assignment
            let mut assignments = self.assignments.write().await;
            assignments
                .entry(monitor_uuid.to_string())
                .or_insert_with(Vec::new)
                .push(helper_peer_id.to_string());
        }

        Ok(())
    }

    /// Handle helper assignment rejection
    pub async fn handle_helper_rejected(&self, monitor_uuid: &str, helper_peer_id: &str, reason: &str) -> Result<()> {
        warn!("Helper {} rejected assignment for monitor {}: {}", helper_peer_id, monitor_uuid, reason);

        // Remove from pending
        let mut pending = self.pending_assignments.write().await;
        if let Some((helpers, _)) = pending.get_mut(monitor_uuid) {
            helpers.retain(|h| h != helper_peer_id);
        }

        // Try to find a replacement helper
        if let Ok(Some(monitor)) = self.database.get_monitor_by_uuid(uuid::Uuid::parse_str(monitor_uuid)?).await {
            if monitor.is_private() {
                info!("Finding replacement helper for monitor {}", monitor_uuid);
                if let Err(e) = self.handle_new_monitor(&monitor).await {
                    warn!("Failed to find replacement helper: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Handle encrypted result from helper (updates last_seen and trust score)
    pub async fn handle_helper_result(&self, helper_peer_id: &str) {
        let mut status_map = self.helper_status.write().await;
        if let Some(status) = status_map.get_mut(helper_peer_id) {
            status.last_seen = SystemTime::now();
        }
        drop(status_map);

        // Record successful operation in trust score
        let mut trust = self.peer_metadata.get_trust_score(helper_peer_id).await;
        trust.record_operation(true);
        self.peer_metadata.update_trust_score(trust).await;
    }

    /// Check for stale helper assignments and reassign
    pub async fn check_helper_health(&self, stale_threshold: Duration) -> Result<()> {
        let now = SystemTime::now();
        let status_map = self.helper_status.read().await;
        
        let mut stale_helpers = Vec::new();
        for (peer_id, status) in status_map.iter() {
            if let Ok(elapsed) = now.duration_since(status.last_seen) {
                if elapsed > stale_threshold {
                    stale_helpers.push(peer_id.clone());
                }
            }
        }
        drop(status_map);

        if !stale_helpers.is_empty() {
            warn!("Found {} stale helpers, reassigning their monitors", stale_helpers.len());
            
            // Find monitors assigned to stale helpers and reassign
            let assignments = self.assignments.read().await.clone();
            for (monitor_uuid, helpers) in assignments {
                let has_stale = helpers.iter().any(|h| stale_helpers.contains(h));
                if has_stale {
                    info!("Reassigning monitor {} due to stale helper", monitor_uuid);
                    if let Ok(Some(monitor)) = self.database.get_monitor_by_uuid(uuid::Uuid::parse_str(&monitor_uuid)?).await {
                        // Clear old assignments
                        self.assignments.write().await.remove(&monitor_uuid);
                        // Assign new helpers
                        if let Err(e) = self.handle_new_monitor(&monitor).await {
                            warn!("Failed to reassign monitor {}: {}", monitor_uuid, e);
                        }
                    }
                }
            }

            // Record failed operation in trust scores and clean up stale helpers
            let mut status_map = self.helper_status.write().await;
            for helper in &stale_helpers {
                status_map.remove(helper);
                let mut trust = self.peer_metadata.get_trust_score(helper).await;
                trust.record_operation(false);
                self.peer_metadata.update_trust_score(trust).await;
            }
        }

        Ok(())
    }

    /// Check pending assignments and timeout unconfirmed ones
    pub async fn check_pending_timeouts(&self) -> Result<()> {
        let now = SystemTime::now();
        let mut pending = self.pending_assignments.write().await;
        
        let mut timed_out = Vec::new();
        for (monitor_uuid, (helpers, assigned_at)) in pending.iter() {
            if let Ok(elapsed) = now.duration_since(*assigned_at) {
                if elapsed > ASSIGNMENT_TIMEOUT {
                    timed_out.push((monitor_uuid.clone(), helpers.clone()));
                }
            }
        }

        for (monitor_uuid, _helpers) in &timed_out {
            pending.remove(monitor_uuid);
        }
        drop(pending);

        if !timed_out.is_empty() {
            warn!("Found {} timed out assignments, reassigning", timed_out.len());
            for (monitor_uuid, _) in timed_out {
                if let Ok(Some(monitor)) = self.database.get_monitor_by_uuid(uuid::Uuid::parse_str(&monitor_uuid)?).await {
                    info!("Reassigning monitor {} due to timeout", monitor_uuid);
                    if let Err(e) = self.handle_new_monitor(&monitor).await {
                        warn!("Failed to reassign timed out monitor {}: {}", monitor_uuid, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Periodic maintenance task
    pub async fn run_maintenance(&self) -> Result<()> {
        // Check for stale helpers (5 minutes without results)
        self.check_helper_health(Duration::from_secs(300)).await?;
        
        // Check for timed out assignments
        self.check_pending_timeouts().await?;

        Ok(())
    }
}

/// Rate limiting for private monitors to prevent abuse
pub struct PrivateMonitorRateLimiter {
    /// Limits per owner peer
    limits: HashMap<String, OwnerLimits>,
}

#[derive(Debug, Clone)]
struct OwnerLimits {
    /// Current private monitors created by owner
    monitors_created: usize,
    /// Total checks performed in current hour
    checks_this_hour: usize,
    /// Hour window start timestamp
    hour_start: i64,
}

impl PrivateMonitorRateLimiter {
    pub fn new() -> Self {
        Self {
            limits: HashMap::new(),
        }
    }

    /// Check if owner can create another private monitor
    pub fn can_add_monitor(&mut self, owner_peer_id: &str) -> bool {
        let limits = self.limits.entry(owner_peer_id.to_string()).or_insert(OwnerLimits {
            monitors_created: 0,
            checks_this_hour: 0,
            hour_start: chrono::Utc::now().timestamp(),
        });

        // Max 10 private monitors per owner
        if limits.monitors_created >= 10 {
            return false;
        }

        limits.monitors_created += 1;
        true
    }

    /// Check if monitor can perform a check (rate limiting)
    pub fn can_check(&mut self, owner_peer_id: &str) -> bool {
        let now = chrono::Utc::now().timestamp();
        let limits = self.limits.entry(owner_peer_id.to_string()).or_insert(OwnerLimits {
            monitors_created: 0,
            checks_this_hour: 0,
            hour_start: now,
        });

        // Reset counter if new hour
        if now - limits.hour_start > 3600 {
            limits.checks_this_hour = 0;
            limits.hour_start = now;
        }

        // Max 100 checks per hour per owner (across all their private monitors)
        if limits.checks_this_hour >= 100 {
            return false;
        }

        limits.checks_this_hour += 1;
        true
    }
}

impl Default for PrivateMonitorRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let mut limiter = PrivateMonitorRateLimiter::new();

        // Should allow first 10 monitors
        for _ in 0..10 {
            assert!(limiter.can_add_monitor("peer1"));
        }

        // Should deny 11th monitor
        assert!(!limiter.can_add_monitor("peer1"));

        // Should allow first 100 checks
        for _ in 0..100 {
            assert!(limiter.can_check("peer1"));
        }

        // Should deny 101st check
        assert!(!limiter.can_check("peer1"));
    }
}
