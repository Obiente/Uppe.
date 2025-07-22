//! Getter methods for PeerNode

use libp2p::PeerId;
use super::PeerNode;

impl PeerNode {
    /// Get the peer ID of this node
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }
    
    /// Get the configuration of this node
    pub fn config(&self) -> &crate::node::config::NodeConfig {
        &self.config
    }
    
    /// Get the current listeners
    pub fn listeners(&self) -> &[(libp2p::core::transport::ListenerId, libp2p::multiaddr::Multiaddr)] {
        &self.listeners
    }
}
