//! Implementation methods for PeerNode.

use anyhow::Result;
use libp2p::PeerId;
use tracing::info;

use super::peer_node::PeerNode;
use crate::{
    network::{PeerUPBehaviour, PeerUPBehaviourState},
    node::{config::NodeConfig, crypto::load_or_generate_keypair},
    transport,
};

impl PeerNode {
    /// Create a new PeerUP node with default configuration
    pub async fn new() -> Result<Self> {
        Self::with_config(NodeConfig::default()).await
    }

    /// Create a new PeerUP node with the specified configuration
    pub async fn with_config(config: NodeConfig) -> Result<Self> {
        // Generate or load keypair
        let keypair = match &config.keypair_path {
            Some(path) => load_or_generate_keypair(path)?,
            None => libp2p::identity::Keypair::generate_ed25519(),
        };

        // Get peer ID from keypair
        let peer_id = PeerId::from(keypair.public());
        info!("Local peer id: {}", peer_id);

        // Set up transport (used by swarm builder)
        let _transport = transport::build_transport(&keypair)?;

        // Create behavior
        let behaviour = PeerUPBehaviour::new(&keypair, &config).await?;

        // Build the swarm
        let swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|_| behaviour)?
            .with_swarm_config(|c| {
                c.with_idle_connection_timeout(std::time::Duration::from_secs(60))
            })
            .build();

        let state = PeerUPBehaviourState::new();

        Ok(PeerNode::new_internal(swarm, peer_id, config, Vec::new(), state))
    }
}
