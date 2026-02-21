//! Implementation methods for PeerNode.

use anyhow::Result;
use libp2p::PeerId;
use tracing::{debug, info};

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

    /// Start listening on configured addresses
    pub fn start_listening(&mut self) -> Result<()> {
        use libp2p::Multiaddr;

        // Listen on all interfaces with configured port range
        let (start_port, end_port) = self.config.port_range;

        for port in start_port..=end_port {
            let addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{port}")
                .parse()
                .map_err(|e| anyhow::anyhow!("Failed to parse multiaddr: {}", e))?;

            match self.swarm.listen_on(addr.clone()) {
                Ok(listener_id) => {
                    info!("Starting listener on {}", addr);
                    self.listeners.push((listener_id, addr));
                }
                Err(e) => {
                    tracing::warn!("Failed to listen on {}: {}", addr, e);
                }
            }
        }

        if self.listeners.is_empty() {
            anyhow::bail!("Failed to start any listeners");
        }

        info!("Started {} listener(s)", self.listeners.len());
        Ok(())
    }

    /// Dial a peer at the specified address
    pub fn dial(&mut self, addr: &str) -> Result<()> {
        use libp2p::Multiaddr;

        let multiaddr: Multiaddr =
            addr.parse().map_err(|e| anyhow::anyhow!("Invalid multiaddr '{}': {}", addr, e))?;

        self.swarm
            .dial(multiaddr.clone())
            .map_err(|e| anyhow::anyhow!("Failed to dial {}: {}", multiaddr, e))?;

        info!("Dialing peer at {}", multiaddr);
        Ok(())
    }

    /// Add bootstrap peers to Kademlia DHT for peer discovery
    /// This is the proper way to bootstrap a Kademlia DHT network
    pub fn add_kademlia_bootstrap_peers(
        &mut self,
        peers: &[(libp2p::PeerId, libp2p::Multiaddr)],
    ) -> Result<()> {
        let kademlia_opt = self.swarm.behaviour_mut().kademlia.as_mut();

        if let Some(kademlia) = kademlia_opt {
            for (peer_id, addr) in peers {
                kademlia.add_address(peer_id, addr.clone());
                info!("Added Kademlia bootstrap peer: {} at {}", peer_id, addr);
            }

            // Only trigger bootstrap if we have peers to bootstrap from
            if !peers.is_empty() {
                match kademlia.bootstrap() {
                    Ok(_) => info!("Kademlia bootstrap initiated with {} peer(s)", peers.len()),
                    Err(e) => tracing::warn!("Kademlia bootstrap error: {:?}", e),
                }
            } else {
                debug!("No bootstrap peers provided, skipping Kademlia bootstrap (will discover via mDNS/connections)");
            }
        } else {
            tracing::warn!("Kademlia is not enabled, cannot add bootstrap peers");
        }

        Ok(())
    }

    /// Dial multiple bootstrap peers (for initial network join)
    /// Note: For production, prefer using add_kademlia_bootstrap_peers for DHT-based discovery
    pub fn dial_bootstrap_peers(&mut self, addrs: &[String]) -> Result<()> {
        if addrs.is_empty() {
            return Ok(());
        }

        info!("Dialing {} bootstrap peer(s)", addrs.len());
        let mut success_count = 0;

        for addr in addrs {
            match self.dial(addr) {
                Ok(_) => success_count += 1,
                Err(e) => tracing::warn!("Failed to dial bootstrap peer {}: {}", addr, e),
            }
        }

        if success_count == 0 && !addrs.is_empty() {
            tracing::warn!("Failed to dial any bootstrap peers ({} attempted)", addrs.len());
        } else {
            info!(
                "Successfully initiated {} of {} bootstrap connections",
                success_count,
                addrs.len()
            );
        }

        Ok(())
    }
}
