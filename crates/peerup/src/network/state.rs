//! State management for PeerUP network behaviour.
//!
//! This module manages the internal state that's not part of the
//! NetworkBehaviour.

use std::collections::HashMap;

use libp2p::PeerId;

/// Internal state for PeerUPBehaviour
pub struct PeerUPBehaviourState {
    /// Track pending outbound requests
    pub pending_requests: HashMap<u64, PeerId>,
    /// Request counter for tracking
    pub request_counter: u64,
}

impl PeerUPBehaviourState {
    /// Create a new state instance
    pub fn new() -> Self {
        Self { pending_requests: HashMap::new(), request_counter: 0 }
    }

    /// Get the next request ID
    pub fn next_request_id(&mut self) -> u64 {
        self.request_counter += 1;
        self.request_counter
    }

    /// Add a pending request
    pub fn add_pending_request(&mut self, request_id: u64, peer_id: PeerId) {
        self.pending_requests.insert(request_id, peer_id);
    }

    /// Remove a pending request
    pub fn remove_pending_request(&mut self, request_id: u64) -> Option<PeerId> {
        self.pending_requests.remove(&request_id)
    }

    /// Get the peer ID for a request
    pub fn get_peer_for_request(&self, request_id: u64) -> Option<&PeerId> {
        self.pending_requests.get(&request_id)
    }
}

impl Default for PeerUPBehaviourState {
    fn default() -> Self {
        Self::new()
    }
}
