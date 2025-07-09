// ...existing code...
//! Network behaviour for PeerUP.
//!
//! This module defines the libp2p NetworkBehaviour for PeerUP.

use std::time::Duration;

use anyhow::Result;
use libp2p::{
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
#[derive(NetworkBehaviour,)]
#[behaviour(to_swarm = "PeerUPEvent")]
pub struct PeerUPBehaviour {
    /// Request/response protocol for probes
    pub request_response: request_response::Behaviour<ProbeCodec,>,
    /// mDNS for local peer discovery
    pub mdns: Toggle<mdns::tokio::Behaviour,>,
    /// Kademlia for DHT functionality
    pub kademlia: Toggle<kad::Behaviour<MemoryStore,>,>,
    /// Relay for NAT traversal
    pub relay: Toggle<libp2p::relay::Behaviour,>,
}

impl PeerUPBehaviour {
    /// Create a new PeerUPBehaviour
    pub async fn new(keypair: &Keypair, config: &NodeConfig,) -> Result<Self,> {
        let local_peer_id = PeerId::from(keypair.public(),);
        let request_response = Self::create_probe_protocol();

        // Create mDNS if enabled
        let mdns = if config.enable_mdns {
            let mdns_config = mdns::Config::default();
            Some(mdns::tokio::Behaviour::new(mdns_config, local_peer_id,)?,)
        } else {
            None
        };

        // Create Kademlia if enabled
        let kademlia = if config.enable_kademlia {
            let store = MemoryStore::new(local_peer_id,);
            let kademlia = kad::Behaviour::new(local_peer_id, store,);
            Some(kademlia,)
        } else {
            None
        };

        // Create relay if enabled
        let relay = if config.enable_relay {
            let relay_config = libp2p::relay::Config::default();
            Some(libp2p::relay::Behaviour::new(local_peer_id, relay_config,),)
        } else {
            None
        };

        Ok(Self {
            request_response,
            mdns: mdns.into(),
            kademlia: kademlia.into(),
            relay: relay.into(),
        },)
    }

    fn create_probe_protocol() -> request_response::Behaviour<ProbeCodec,> {
        let config = request_response::Config::default()
            .with_request_timeout(Duration::from_secs(30,),)
            .with_max_concurrent_streams(5,);

        request_response::Behaviour::new(
            [(
                libp2p::StreamProtocol::new(PROBE_PROTOCOL,),
                request_response::ProtocolSupport::Full,
            ),],
            config,
        )
    }
}
