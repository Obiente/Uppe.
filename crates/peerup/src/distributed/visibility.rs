//! Monitor visibility and coordination types.
//!
//! Defines public, private, and internal monitors with their orchestration behavior.

use serde::{Deserialize, Serialize};

/// Monitor visibility determines coordination and retention behavior
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MonitorVisibility {
    /// Public monitors - community-owned, coordinated
    /// - DHT discoverable
    /// - Grouped by target (e.g., "google.com")
    /// - Distributed orchestration
    /// - Long-term retention across peers
    /// - Displayed on public dashboards (uppe.rs)
    /// - NO secrets required
    Public {
        /// Public domain identifier (e.g., "google.com", "github.com")
        domain: String,
        /// Display name for the grouped monitor
        display_name: String,
    },

    /// Private monitors - owner-controlled, peer-assisted
    /// - NOT discoverable via DHT
    /// - Peers help monitor (collaborative)
    /// - Results encrypted for owner
    /// - Peers store temporarily (7 days max)
    /// - Owner syncs when online
    /// - NO secrets shared with peers
    Private {
        /// Owner's peer ID
        owner_peer_id: String,
    },

    /// Internal monitors - owner-only, secrets required
    /// - NO peer orchestration (owner monitors alone)
    /// - NOT shared with network
    /// - Requires authentication/secrets
    /// - Only owner has access
    /// - Use cases: databases, internal APIs, authenticated endpoints
    Internal {
        /// Owner's peer ID
        owner_peer_id: String,
    },
}

/// Monitor coordination state for grouped public monitors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicMonitorGroup {
    /// Public domain being monitored (e.g., "google.com")
    pub domain: String,

    /// Display name (e.g., "Google Search")
    pub display_name: String,

    /// All peer IDs participating in this monitor group
    pub participating_peers: Vec<String>,

    /// Orchestration schedule - which peer checks when
    pub schedule: OrchestrationSchedule,

    /// Total check count across all peers
    pub total_checks: u64,

    /// When this group was created
    pub created_at: i64,

    /// Last time schedule was updated
    pub last_updated: i64,
}

/// Orchestration schedule for coordinated monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationSchedule {
    /// Check interval in seconds
    pub interval_seconds: u64,

    /// Peer assignments: peer_id -> next_check_timestamp
    pub assignments: Vec<PeerAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAssignment {
    pub peer_id: String,
    pub next_check_at: i64,
    pub check_sequence: u64, // Which check in the rotation
}

impl MonitorVisibility {
    /// Check if this is a public monitor
    pub fn is_public(&self) -> bool {
        matches!(self, MonitorVisibility::Public { .. })
    }

    /// Check if this is a private monitor (peer-assisted)
    pub fn is_private(&self) -> bool {
        matches!(self, MonitorVisibility::Private { .. })
    }

    /// Check if this is an internal monitor (secrets, no peers)
    pub fn is_internal(&self) -> bool {
        matches!(self, MonitorVisibility::Internal { .. })
    }

    /// Check if this monitor requires peer orchestration
    pub fn requires_orchestration(&self) -> bool {
        matches!(self, MonitorVisibility::Public { .. } | MonitorVisibility::Private { .. })
    }

    /// Get the public domain if this is a public monitor
    pub fn public_domain(&self) -> Option<&str> {
        match self {
            MonitorVisibility::Public { domain, .. } => Some(domain),
            _ => None,
        }
    }

    /// Get the owner peer ID (for Private and Internal monitors)
    pub fn owner_peer_id(&self) -> Option<&str> {
        match self {
            MonitorVisibility::Private { owner_peer_id } 
            | MonitorVisibility::Internal { owner_peer_id } => Some(owner_peer_id),
            MonitorVisibility::Public { .. } => None,
        }
    }

    /// Get retention behavior based on visibility
    pub fn retention_policy(&self) -> RetentionPolicy {
        match self {
            MonitorVisibility::Public { .. } => RetentionPolicy::LongTerm {
                days: 30, // Public monitors: keep for 30 days across all peers
            },
            MonitorVisibility::Private { .. } => RetentionPolicy::UntilOwnerSyncs {
                max_days: 7, // Private monitors: temporary storage until owner syncs
            },
            MonitorVisibility::Internal { .. } => RetentionPolicy::OwnerOnly {
                // Internal monitors: never leave owner's node
            },
        }
    }
}

/// Retention policy based on monitor visibility
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetentionPolicy {
    /// Long-term retention (public monitors)
    /// Data persists across all peers
    LongTerm { days: u64 },

    /// Temporary retention (private monitors)
    /// Data deleted after owner syncs OR max_days
    UntilOwnerSyncs { max_days: u64 },

    /// Owner-only retention (internal monitors)
    /// Data never leaves owner's node
    OwnerOnly,
}

impl PublicMonitorGroup {
    /// Create a new public monitor group
    pub fn new(domain: String, display_name: String, creator_peer_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            domain,
            display_name,
            participating_peers: vec![creator_peer_id.clone()],
            schedule: OrchestrationSchedule {
                interval_seconds: 60, // Default: check every 60 seconds
                assignments: vec![PeerAssignment {
                    peer_id: creator_peer_id,
                    next_check_at: now + 60,
                    check_sequence: 0,
                }],
            },
            total_checks: 0,
            created_at: now,
            last_updated: now,
        }
    }

    /// Add a peer to this monitor group
    pub fn add_peer(&mut self, peer_id: String) {
        if !self.participating_peers.contains(&peer_id) {
            self.participating_peers.push(peer_id.clone());
            self.rebalance_schedule();
        }
    }

    /// Remove a peer from this monitor group
    pub fn remove_peer(&mut self, peer_id: &str) {
        self.participating_peers.retain(|p| p != peer_id);
        self.rebalance_schedule();
    }

    /// Rebalance the orchestration schedule across all peers
    fn rebalance_schedule(&mut self) {
        let now = chrono::Utc::now().timestamp();
        let peer_count = self.participating_peers.len() as u64;

        if peer_count == 0 {
            self.schedule.assignments.clear();
            return;
        }

        // Distribute checks evenly across peers
        // If interval is 60s and we have 3 peers, each peer checks every 180s
        // But checks are staggered: peer1 at t+0, peer2 at t+20, peer3 at t+40
        let time_between_checks = self.schedule.interval_seconds / peer_count.max(1);

        self.schedule.assignments = self
            .participating_peers
            .iter()
            .enumerate()
            .map(|(idx, peer_id)| PeerAssignment {
                peer_id: peer_id.clone(),
                next_check_at: now + (idx as i64 * time_between_checks as i64),
                check_sequence: idx as u64,
            })
            .collect();

        self.last_updated = now;
    }

    /// Get which peer should perform the next check
    pub fn next_checker(&self) -> Option<&str> {
        self.schedule
            .assignments
            .iter()
            .min_by_key(|a| a.next_check_at)
            .map(|a| a.peer_id.as_str())
    }

    /// Update check time for a peer after they complete a check
    pub fn mark_check_completed(&mut self, peer_id: &str) {
        if let Some(assignment) = self
            .schedule
            .assignments
            .iter_mut()
            .find(|a| a.peer_id == peer_id)
        {
            assignment.next_check_at += (self.schedule.interval_seconds
                * self.participating_peers.len() as u64)
                as i64;
            self.total_checks += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_monitor_group_creation() {
        let group = PublicMonitorGroup::new(
            "google.com".to_string(),
            "Google Search".to_string(),
            "peer1".to_string(),
        );

        assert_eq!(group.domain, "google.com");
        assert_eq!(group.participating_peers.len(), 1);
        assert_eq!(group.schedule.assignments.len(), 1);
    }

    #[test]
    fn test_add_peer_rebalances() {
        let mut group = PublicMonitorGroup::new(
            "github.com".to_string(),
            "GitHub".to_string(),
            "peer1".to_string(),
        );

        group.add_peer("peer2".to_string());
        group.add_peer("peer3".to_string());

        assert_eq!(group.participating_peers.len(), 3);
        assert_eq!(group.schedule.assignments.len(), 3);

        // Check that checks are staggered
        let times: Vec<i64> = group
            .schedule
            .assignments
            .iter()
            .map(|a| a.next_check_at)
            .collect();
        assert!(times[0] < times[1]);
        assert!(times[1] < times[2]);
    }

    #[test]
    fn test_retention_policy() {
        let public = MonitorVisibility::Public {
            domain: "google.com".to_string(),
            display_name: "Google".to_string(),
        };

        let private = MonitorVisibility::Private {
            owner_peer_id: "peer1".to_string(),
        };

        assert!(public.is_public());
        assert!(!public.is_private());
        assert!(!private.is_public());
        assert!(private.is_private());

        match public.retention_policy() {
            RetentionPolicy::LongTerm { days } => assert_eq!(days, 30),
            _ => panic!("Expected LongTerm retention"),
        }

        match private.retention_policy() {
            RetentionPolicy::UntilOwnerSyncs { max_days } => assert_eq!(max_days, 7),
            _ => panic!("Expected UntilOwnerSyncs retention"),
        }
    }
}
