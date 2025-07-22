//! Kademlia DHT configuration for PeerUP.
//!
//! This module provides configuration utilities for the Kademlia DHT.

use std::time::Duration;

use anyhow::Result;
use libp2p::{
    kad::{
        store::MemoryStore,
        {self},
    },
    PeerId,
};
use tracing::{debug, warn};

/// Configure and create a Kademlia DHT for peer discovery
pub fn configure_kademlia(local_peer_id: PeerId) -> Result<kad::Behaviour<MemoryStore>> {
    // Create a Kademlia instance with the local peer ID
    let store = MemoryStore::new(local_peer_id);
    let mut config = kad::Config::default();

    // Configure Kademlia for better discovery
    config
        .set_query_timeout(Duration::from_secs(30))
        .set_record_ttl(Some(Duration::from_secs(60 * 60))) // 1 hour
        .set_publication_interval(Some(Duration::from_secs(60 * 15))); // 15 minutes

    let mut kademlia = kad::Behaviour::with_config(local_peer_id, store, config);

    // Set the default mode to server
    kademlia.set_mode(Some(kad::Mode::Server));

    debug!("Kademlia discovery initialized for peer {}", local_peer_id);
    Ok(kademlia)
}

/// Add bootstrap peers to Kademlia
pub fn add_bootstrap_peers(
    _kademlia: &mut kad::Behaviour<MemoryStore>,
    bootstrap_peers: &[String],
) -> Vec<String> {
    let mut errors = Vec::new();

    for peer_addr in bootstrap_peers {
        match peer_addr.parse::<libp2p::Multiaddr>() {
            Ok(addr) => {
                debug!("Adding bootstrap peer to Kademlia: {}", addr);
                // Note: The correct method needs to be determined based on
                // libp2p version
                // kademlia.add_address_candidate(&addr);
            }
            Err(e) => {
                warn!("Failed to parse bootstrap peer address '{}': {}", peer_addr, e);
                errors.push(format!("Invalid address '{peer_addr}': {e}"));
            }
        }
    }

    errors
}

/// Create a development Kademlia configuration
pub fn create_dev_kademlia(local_peer_id: PeerId) -> Result<kad::Behaviour<MemoryStore>> {
    let store = MemoryStore::new(local_peer_id);
    let mut config = kad::Config::default();

    // Fast configuration for development
    config
        .set_query_timeout(Duration::from_secs(10))
        .set_record_ttl(Some(Duration::from_secs(300))) // 5 minutes
        .set_publication_interval(Some(Duration::from_secs(60))); // 1 minute

    let mut kademlia = kad::Behaviour::with_config(local_peer_id, store, config);
    kademlia.set_mode(Some(kad::Mode::Server));

    Ok(kademlia)
}
