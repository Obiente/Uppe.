//! mDNS configuration for PeerUP.
//!
//! This module provides configuration utilities for mDNS peer discovery.

use anyhow::Result;
use libp2p::{mdns, PeerId};
use tracing::debug;

/// Configure and create mDNS for local peer discovery
pub fn configure_mdns(local_peer_id: PeerId,) -> Result<mdns::tokio::Behaviour,> {
    let config = mdns::Config::default();
    let behaviour = mdns::tokio::Behaviour::new(config, local_peer_id,)?;

    debug!("mDNS discovery initialized for peer {}", local_peer_id);
    Ok(behaviour,)
}

/// Create a development mDNS configuration
pub fn create_dev_mdns(local_peer_id: PeerId,) -> Result<mdns::tokio::Behaviour,> {
    let config = mdns::Config::default();
    let behaviour = mdns::tokio::Behaviour::new(config, local_peer_id,)?;

    debug!("Development mDNS initialized for peer {}", local_peer_id);
    Ok(behaviour,)
}

/// Check if mDNS is available on the current platform
pub fn is_mdns_available() -> bool {
    // mDNS is generally available on most platforms
    // This could be extended to check for specific platform limitations
    true
}
