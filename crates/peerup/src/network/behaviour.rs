// ...existing code...
//! Network behaviour for PeerUP.
//!
//! This module defines the libp2p NetworkBehaviour for PeerUP.

use std::time::Duration;

use anyhow::Result;
use libp2p::{
    gossipsub,
    identity::Keypair,
    kad::{
        store::MemoryStore,
        {self},
    },
    mdns, request_response,
    swarm::{behaviour::toggle::Toggle, NetworkBehaviour},
    PeerId,
};

use super::events::PeerUPEvent;
use crate::{
    node::NodeConfig,
    protocol::{ProbeCodec, PROBE_PROTOCOL},
};

/// The main network behaviour for PeerUP
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "PeerUPEvent")]
pub struct PeerUPBehaviour {
    /// Gossipsub for pub/sub messaging (result broadcasting)
    pub gossipsub: gossipsub::Behaviour,
    /// Request/response protocol for probes
    pub request_response: request_response::Behaviour<ProbeCodec>,
    /// mDNS for local peer discovery
    pub mdns: Toggle<mdns::tokio::Behaviour>,
    /// Kademlia for DHT functionality
    pub kademlia: Toggle<kad::Behaviour<MemoryStore>>,
    /// Relay for NAT traversal
    pub relay: Toggle<libp2p::relay::Behaviour>,
}

impl PeerUPBehaviour {
    /// Create a new PeerUPBehaviour
    pub async fn new(keypair: &Keypair, config: &NodeConfig) -> Result<Self> {
        let local_peer_id = PeerId::from(keypair.public());

        // Create gossipsub for result broadcasting
        let gossipsub = Self::create_gossipsub(keypair)?;

        let request_response = Self::create_probe_protocol();

        // Create mDNS if enabled (gracefully handle platform limitations)
        let mdns = if config.enable_mdns {
            let mdns_config = mdns::Config::default();
            match mdns::tokio::Behaviour::new(mdns_config, local_peer_id) {
                Ok(behaviour) => {
                    tracing::info!("mDNS local peer discovery enabled");
                    Some(behaviour)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to enable mDNS (platform limitation or network config): {}",
                        e
                    );
                    tracing::info!(
                        "Peer discovery will rely on Kademlia DHT and configured bootstrap peers"
                    );
                    None
                }
            }
        } else {
            tracing::info!("mDNS disabled by configuration");
            None
        };

        // Create Kademlia if enabled (production-ready DHT peer discovery)
        let kademlia = if config.enable_kademlia {
            let store = MemoryStore::new(local_peer_id);
            let mut kademlia = kad::Behaviour::new(local_peer_id, store);

            // Set to server mode for better network participation
            kademlia.set_mode(Some(kad::Mode::Server));

            tracing::info!("Kademlia DHT peer discovery enabled");
            Some(kademlia)
        } else {
            tracing::info!("Kademlia DHT disabled by configuration");
            None
        };

        // Create relay if enabled
        let relay = if config.enable_relay {
            let relay_config = libp2p::relay::Config::default();
            Some(libp2p::relay::Behaviour::new(local_peer_id, relay_config))
        } else {
            None
        };

        Ok(Self {
            gossipsub,
            request_response,
            mdns: mdns.into(),
            kademlia: kademlia.into(),
            relay: relay.into(),
        })
    }

    fn create_gossipsub(keypair: &Keypair) -> Result<gossipsub::Behaviour> {
        // Configure gossipsub for monitoring results
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(|msg| {
                // Use message data hash as ID for deduplication
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                msg.data.hash(&mut hasher);
                gossipsub::MessageId::from(hasher.finish().to_string())
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create gossipsub config: {}", e))?;

        gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create gossipsub behaviour: {}", e))
    }

    fn create_probe_protocol() -> request_response::Behaviour<ProbeCodec> {
        let config = request_response::Config::default()
            .with_request_timeout(Duration::from_secs(30))
            .with_max_concurrent_streams(5);

        request_response::Behaviour::new(
            [(
                libp2p::StreamProtocol::new(PROBE_PROTOCOL),
                request_response::ProtocolSupport::Full,
            )],
            config,
        )
    }
}
