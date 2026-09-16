//! Monitor visibility and coordination types.
//!
//! Defines public, private, and internal monitors with their orchestration behavior.

use serde::{Deserialize, Serialize};

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
    /// Update check time for a peer after they complete a check
    pub fn mark_check_completed(&mut self, peer_id: &str) {
        if let Some(assignment) =
            self.schedule.assignments.iter_mut().find(|a| a.peer_id == peer_id)
        {
            assignment.next_check_at +=
                (self.schedule.interval_seconds * self.participating_peers.len() as u64) as i64;
            self.total_checks += 1;
        }
    }
}
