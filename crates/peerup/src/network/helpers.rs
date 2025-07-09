//! Network helpers and utilities for PeerUP.
//!
//! This module provides helper functions for network operations.

use anyhow::Result;
use libp2p::{Multiaddr, PeerId};

/// Extract peer ID from multiaddr
pub fn extract_peer_id_from_multiaddr(addr: &Multiaddr) -> Option<PeerId> {
    // Parse the multiaddr to extract peer ID
    for protocol in addr.iter() {
        if let libp2p::multiaddr::Protocol::P2p(peer_id) = protocol {
            return Some(peer_id);
        }
    }
    None
}

/// Validate a multiaddr for peer connection
pub fn validate_multiaddr(addr: &Multiaddr) -> Result<()> {
    // Basic validation - check if it has required components
    let mut has_transport = false;
    let mut has_peer_id = false;

    for protocol in addr.iter() {
        match protocol {
            libp2p::multiaddr::Protocol::Tcp(_)
            | libp2p::multiaddr::Protocol::Udp(_)
            | libp2p::multiaddr::Protocol::Quic => {
                has_transport = true;
            },
            libp2p::multiaddr::Protocol::P2p(_) => {
                has_peer_id = true;
            },
            _ => {},
        }
    }

    if !has_transport {
        return Err(anyhow::anyhow!("Multiaddr missing transport protocol"));
    }

    if !has_peer_id {
        return Err(anyhow::anyhow!("Multiaddr missing peer ID"));
    }

    Ok(())
}

/// Create a basic multiaddr for testing
pub fn create_test_multiaddr(port: u16) -> Multiaddr {
    format!("/ip4/127.0.0.1/tcp/{port}").parse().expect("Invalid multiaddr")
}
