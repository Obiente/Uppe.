//! PeerNode struct definition.

use libp2p::{core::transport::ListenerId, multiaddr::Multiaddr, swarm::Swarm, PeerId};

use crate::{
    network::{PeerUPBehaviour, PeerUPBehaviourState},
    node::config::NodeConfig,
};

/// A PeerUP network node
pub struct PeerNode {
    /// The node's libp2p swarm
    pub swarm: Swarm<PeerUPBehaviour,>,

    /// The node's peer ID
    pub peer_id: PeerId,

    /// The node's configuration
    pub config: NodeConfig,

    /// Listeners that have been established
    pub listeners: Vec<(ListenerId, Multiaddr,),>,

    /// Network behaviour state
    pub state: PeerUPBehaviourState,
}

impl PeerNode {
    /// Get the peer ID of this node
    pub fn peer_id(&self,) -> PeerId {
        self.peer_id
    }

    /// Get the configuration of this node
    pub fn config(&self,) -> &NodeConfig {
        &self.config
    }

    /// Get the current listeners
    pub fn listeners(&self,) -> &[(ListenerId, Multiaddr,)] {
        &self.listeners
    }

    pub fn new_internal(
        swarm: Swarm<PeerUPBehaviour,>,
        peer_id: PeerId,
        config: NodeConfig,
        listeners: Vec<(ListenerId, Multiaddr,),>,
        state: PeerUPBehaviourState,
    ) -> Self {
        Self {
            swarm,
            peer_id,
            config,
            listeners,
            state,
        }
    }
}
