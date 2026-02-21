//! Consensus protocol for distributed orchestration.
//!
//! This module implements a consensus mechanism to coordinate monitoring
//! across peers, preventing abuse and DDoS while ensuring fair distribution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use super::visibility::OrchestrationSchedule;

/// Consensus vote for public monitor orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationVote {
    /// Public domain being orchestrated
    pub domain: String,

    /// Proposed schedule
    pub schedule: OrchestrationSchedule,

    /// Voter's peer ID
    pub voter_peer_id: String,

    /// Ed25519 signature (hex-encoded) of "{domain}:{schedule_json}:{timestamp}"
    pub signature: String,

    /// Voter's Ed25519 public key (hex-encoded, 32 bytes)
    #[serde(default)]
    pub public_key: Option<Vec<u8>>,

    /// When vote was cast
    pub timestamp: i64,
}

/// Consensus state for a public monitor group
#[derive(Debug, Clone)]
pub struct ConsensusState {
    /// Public domain
    pub domain: String,

    /// Current agreed-upon schedule
    pub current_schedule: super::visibility::OrchestrationSchedule,

    /// Pending votes for schedule changes
    pub pending_votes: Vec<OrchestrationVote>,

    /// Quorum threshold (percentage of peers needed to agree)
    pub quorum_threshold: f64,

    /// Last consensus timestamp
    pub last_consensus_at: SystemTime,

    /// Rate limiting state
    pub rate_limit: RateLimitState,
}

/// Rate limiting to prevent DDoS
#[derive(Debug, Clone)]
pub struct RateLimitState {
    /// Minimum interval between checks (seconds)
    pub min_check_interval: u64,

    /// Maximum checks per peer per hour
    pub max_checks_per_peer_per_hour: u64,

    /// Recent check counts per peer
    pub peer_check_counts: HashMap<String, u64>,

    /// Window start time
    pub window_start: SystemTime,
}

impl RateLimitState {
    /// Create new rate limit state with defaults
    pub fn new() -> Self {
        Self {
            min_check_interval: 10, // Min 10 seconds between checks
            max_checks_per_peer_per_hour: 360, // Max 360 checks/hour = 1 every 10s
            peer_check_counts: HashMap::new(),
            window_start: SystemTime::now(),
        }
    }

    /// Check if a peer can perform a check (rate limit validation)
    pub fn can_check(&mut self, peer_id: &str) -> bool {
        // Reset window if hour has passed
        if self.window_start.elapsed().unwrap_or(Duration::ZERO) > Duration::from_secs(3600) {
            self.peer_check_counts.clear();
            self.window_start = SystemTime::now();
        }

        let count = self.peer_check_counts.entry(peer_id.to_string()).or_insert(0);

        if *count >= self.max_checks_per_peer_per_hour {
            return false; // Rate limit exceeded
        }

        *count += 1;
        true
    }

    /// Validate proposed schedule against rate limits
    pub fn validate_schedule(
        &self,
        schedule: &super::visibility::OrchestrationSchedule,
        peer_count: usize,
    ) -> Result<(), String> {
        // Check minimum interval
        if schedule.interval_seconds < self.min_check_interval {
            return Err(format!(
                "Check interval too low: {} < {}",
                schedule.interval_seconds, self.min_check_interval
            ));
        }

        // Check peer count is valid
        if peer_count == 0 {
            return Err("Cannot validate schedule with zero peers".to_string());
        }

        // Check that with peer_count, we don't exceed max checks/hour
        let divisor = schedule.interval_seconds * peer_count as u64;
        if divisor == 0 {
            return Err("Invalid schedule: interval cannot be zero".to_string());
        }
        let checks_per_peer_per_hour = 3600 / divisor;

        if checks_per_peer_per_hour > self.max_checks_per_peer_per_hour {
            return Err(format!(
                "Schedule would cause {} checks/peer/hour (max: {})",
                checks_per_peer_per_hour, self.max_checks_per_peer_per_hour
            ));
        }

        Ok(())
    }
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsensusState {
    /// Create new consensus state for a domain
    pub fn new(domain: String, initial_schedule: super::visibility::OrchestrationSchedule) -> Self {
        Self {
            domain,
            current_schedule: initial_schedule,
            pending_votes: Vec::new(),
            quorum_threshold: 0.67, // 67% consensus required
            last_consensus_at: SystemTime::now(),
            rate_limit: RateLimitState::new(),
        }
    }

    /// Cast a vote for a schedule change
    pub fn cast_vote(&mut self, vote: OrchestrationVote) -> Result<(), String> {
        // Validate signature
        if !self.verify_vote_signature(&vote) {
            return Err("Invalid vote signature".to_string());
        }

        // Validate schedule against rate limits
        self.rate_limit.validate_schedule(
            &vote.schedule,
            vote.schedule.assignments.len(),
        )?;

        // Check if already voted
        if self.pending_votes.iter().any(|v| v.voter_peer_id == vote.voter_peer_id) {
            return Err("Peer already voted".to_string());
        }

        self.pending_votes.push(vote);
        Ok(())
    }

    /// Check if consensus has been reached
    pub fn check_consensus(&mut self, total_peers: usize) -> Option<super::visibility::OrchestrationSchedule> {
        if self.pending_votes.is_empty() {
            return None;
        }

        // Group votes by proposed schedule (serialize to compare)
        let mut vote_groups: HashMap<String, Vec<&OrchestrationVote>> = HashMap::new();

        for vote in &self.pending_votes {
            let schedule_json = serde_json::to_string(&vote.schedule).unwrap_or_default();
            vote_groups.entry(schedule_json).or_default().push(vote);
        }

        // Find schedule with most votes
        let (winning_schedule_json, winning_votes) = vote_groups
            .iter()
            .max_by_key(|(_, votes)| votes.len())?;

        let vote_percentage = winning_votes.len() as f64 / total_peers as f64;

        // Check if quorum reached
        if vote_percentage >= self.quorum_threshold {
            // Deserialize winning schedule
            if let Ok(schedule) =
                serde_json::from_str::<OrchestrationSchedule>(winning_schedule_json)
            {
                self.current_schedule = schedule.clone();
                self.last_consensus_at = SystemTime::now();
                self.pending_votes.clear(); // Clear votes after consensus
                return Some(schedule);
            }
        }

        None
    }

    /// Verify vote signature using Ed25519
    fn verify_vote_signature(&self, vote: &OrchestrationVote) -> bool {
        let Some(public_key_bytes) = &vote.public_key else {
            tracing::warn!(
                target: "uppe::audit",
                peer = %vote.voter_peer_id,
                domain = %vote.domain,
                "Vote missing public key, rejecting"
            );
            return false;
        };

        if public_key_bytes.len() != 32 {
            tracing::warn!(
                target: "uppe::audit",
                peer = %vote.voter_peer_id,
                "Invalid public key length: {} (expected 32)",
                public_key_bytes.len()
            );
            return false;
        }

        // Decode hex signature
        let Ok(signature_bytes) = hex::decode(&vote.signature) else {
            tracing::warn!(
                target: "uppe::audit",
                peer = %vote.voter_peer_id,
                "Invalid hex signature in vote"
            );
            return false;
        };

        // Reconstruct the signed data
        let schedule_json = match serde_json::to_string(&vote.schedule) {
            Ok(j) => j,
            Err(_) => return false,
        };
        let vote_data = format!("{}:{}:{}", vote.domain, schedule_json, vote.timestamp);

        // Verify using peerup's crypto
        let mut pubkey_array = [0u8; 32];
        pubkey_array.copy_from_slice(public_key_bytes);

        match crate::crypto::verify_signature(vote_data.as_bytes(), &signature_bytes, &pubkey_array) {
            Ok(valid) => {
                if !valid {
                    tracing::warn!(
                        target: "uppe::audit",
                        peer = %vote.voter_peer_id,
                        domain = %vote.domain,
                        "Vote signature verification failed"
                    );
                }
                valid
            }
            Err(e) => {
                tracing::warn!(
                    target: "uppe::audit",
                    peer = %vote.voter_peer_id,
                    error = %e,
                    "Vote signature verification error"
                );
                false
            }
        }
    }

    /// Check if a peer should perform next check (consensus-based)
    pub fn should_check_now(&mut self, peer_id: &str) -> bool {
        // Find peer's assignment
        if let Some(assignment) = self.current_schedule.assignments.iter().find(|a| a.peer_id == peer_id) {
            let now = chrono::Utc::now().timestamp();
            
            // Check rate limit first
            if !self.rate_limit.can_check(peer_id) {
                return false;
            }

            // Check if it's time for this peer to perform check
            assignment.next_check_at <= now
        } else {
            false
        }
    }

    /// Mark check completed and update schedule
    pub fn mark_check_completed(&mut self, peer_id: &str) {
        // Get assignment index first
        let assignment_idx = self
            .current_schedule
            .assignments
            .iter()
            .position(|a| a.peer_id == peer_id);

        if let Some(idx) = assignment_idx {
            let interval_per_peer = self.current_schedule.interval_seconds
                * self.current_schedule.assignments.len() as u64;
            self.current_schedule.assignments[idx].next_check_at += interval_per_peer as i64;
        }
    }
}

/// Consensus manager for all public monitor groups
pub struct ConsensusManager {
    /// Consensus states per domain
    pub states: tokio::sync::RwLock<HashMap<String, ConsensusState>>,
}

impl ConsensusManager {
    pub fn new() -> Self {
        Self {
            states: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Get or create consensus state for a domain
    pub async fn get_or_create(
        &self,
        domain: &str,
        initial_schedule: super::visibility::OrchestrationSchedule,
    ) -> ConsensusState {
        let mut states = self.states.write().await;
        states
            .entry(domain.to_string())
            .or_insert_with(|| ConsensusState::new(domain.to_string(), initial_schedule))
            .clone()
    }

    /// Cast a vote for orchestration change
    pub async fn cast_vote(&self, domain: &str, vote: OrchestrationVote) -> Result<(), String> {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(domain) {
            state.cast_vote(vote)
        } else {
            Err("Domain not found".to_string())
        }
    }

    /// Check consensus for a domain
    pub async fn check_consensus(&self, domain: &str, total_peers: usize) -> Option<super::visibility::OrchestrationSchedule> {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(domain) {
            state.check_consensus(total_peers)
        } else {
            None
        }
    }

    /// Check if peer should perform check now (consensus-based)
    pub async fn should_check_now(&self, domain: &str, peer_id: &str) -> bool {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(domain) {
            state.should_check_now(peer_id)
        } else {
            false
        }
    }

    /// Mark check completed
    pub async fn mark_check_completed(&self, domain: &str, peer_id: &str) {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(domain) {
            state.mark_check_completed(peer_id);
        }
    }
}

impl Default for ConsensusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_validation() {
        let rate_limit = RateLimitState::new();

        // Valid schedule: 60s interval with 1 peer = 60 checks/hour
        let schedule = super::super::visibility::OrchestrationSchedule {
            interval_seconds: 60,
            assignments: vec![],
        };
        assert!(rate_limit.validate_schedule(&schedule, 1).is_ok());

        // Invalid schedule: 5s interval with 1 peer = 720 checks/hour (too many)
        let schedule = super::super::visibility::OrchestrationSchedule {
            interval_seconds: 5,
            assignments: vec![],
        };
        assert!(rate_limit.validate_schedule(&schedule, 1).is_err());
    }

    #[test]
    fn test_peer_rate_limiting() {
        let mut rate_limit = RateLimitState::new();

        // First check should succeed
        assert!(rate_limit.can_check("peer1"));

        // Subsequent checks within limits should succeed
        for _ in 0..100 {
            assert!(rate_limit.can_check("peer1"));
        }

        // After exceeding limit, should fail
        for _ in 0..260 {
            rate_limit.can_check("peer1");
        }
        assert!(!rate_limit.can_check("peer1"));
    }

    #[tokio::test]
    async fn test_consensus() {
        let manager = ConsensusManager::new();

        let schedule = super::super::visibility::OrchestrationSchedule {
            interval_seconds: 60,
            assignments: vec![
                super::super::visibility::PeerAssignment {
                    peer_id: "peer0".to_string(),
                    next_check_at: 0,
                    check_sequence: 0,
                },
                super::super::visibility::PeerAssignment {
                    peer_id: "peer1".to_string(),
                    next_check_at: 20,
                    check_sequence: 1,
                },
                super::super::visibility::PeerAssignment {
                    peer_id: "peer2".to_string(),
                    next_check_at: 40,
                    check_sequence: 2,
                },
            ],
        };

        let _state = manager.get_or_create("test.com", schedule.clone()).await;

        // Cast votes with real signatures
        for i in 0..3 {
            let keypair = crate::crypto::generate_keypair();
            let timestamp = chrono::Utc::now().timestamp();
            let schedule_json = serde_json::to_string(&schedule).unwrap();
            let vote_data = format!("test.com:{}:{}", schedule_json, timestamp);
            let sig_bytes = crate::crypto::sign_bytes(vote_data.as_bytes(), &keypair);

            let vote = OrchestrationVote {
                domain: "test.com".to_string(),
                schedule: schedule.clone(),
                voter_peer_id: format!("peer{}", i),
                signature: hex::encode(&sig_bytes),
                public_key: Some(keypair.public_key_bytes().to_vec()),
                timestamp,
            };

            manager.cast_vote("test.com", vote).await.unwrap();
        }

        // Check consensus (3 out of 3 peers voted = 100%)
        let consensus = manager.check_consensus("test.com", 3).await;
        assert!(consensus.is_some());
    }
}
