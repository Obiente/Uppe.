//! Relay configuration for PeerUP.
//!
//! This module provides configuration utilities for libp2p relay.

use anyhow::Result;
use libp2p::{relay, PeerId};
use tracing::debug;

/// Configure relay client
pub fn configure_relay_client(local_peer_id: PeerId,) -> Result<relay::Behaviour,> {
    let config = relay::Config::default();
    let relay_behaviour = relay::Behaviour::new(local_peer_id, config,);
    debug!("Relay client initialized for peer {}", local_peer_id);
    Ok(relay_behaviour,)
}

/// Configure relay server
pub fn configure_relay_server(local_peer_id: PeerId,) -> Result<relay::Behaviour,> {
    let config = relay::Config::default();
    // Configure as relay server
    // Note: Specific server configuration depends on libp2p version
    let relay_behaviour = relay::Behaviour::new(local_peer_id, config,);
    debug!("Relay server initialized for peer {}", local_peer_id);
    Ok(relay_behaviour,)
}

/// Create a development relay configuration
pub fn create_dev_relay(local_peer_id: PeerId,) -> Result<relay::Behaviour,> {
    let config = relay::Config::default();
    let relay_behaviour = relay::Behaviour::new(local_peer_id, config,);
    debug!("Development relay initialized for peer {}", local_peer_id);
    Ok(relay_behaviour,)
}
