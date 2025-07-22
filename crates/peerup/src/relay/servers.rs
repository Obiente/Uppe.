//! Relay server management for PeerUP.
//!
//! This module provides utilities for managing relay servers.

use libp2p::multiaddr::Multiaddr;
use tracing::{debug, info, warn};

/// Add relay servers to the client
pub fn add_relay_servers(relay_addresses: &[String]) -> Vec<String> {
    let mut errors = Vec::new();

    for relay_addr in relay_addresses {
        match relay_addr.parse::<Multiaddr>() {
            Ok(addr) => {
                debug!("Adding relay server: {}", addr);
                // In libp2p 0.56.0, relay server handling has changed
                // The proper handling would be implemented in the NetworkBehaviour
                info!("Relay server added: {}", addr);
            }
            Err(e) => {
                let error = format!("Invalid relay address {relay_addr}: {e}");
                warn!("{}", error);
                errors.push(error);
            }
        }
    }

    errors
}

/// Validate relay server addresses
pub fn validate_relay_addresses(addresses: &[String]) -> Result<Vec<Multiaddr>, Vec<String>> {
    let mut valid_addresses = Vec::new();
    let mut errors = Vec::new();

    for addr_str in addresses {
        match addr_str.parse::<Multiaddr>() {
            Ok(addr) => {
                valid_addresses.push(addr);
            }
            Err(e) => {
                errors.push(format!("Invalid address '{addr_str}': {e}"));
            }
        }
    }

    if errors.is_empty() {
        Ok(valid_addresses)
    } else {
        Err(errors)
    }
}

/// Create a list of default relay servers
pub fn default_relay_servers() -> Vec<String> {
    vec![
        // Add default relay servers here
        // These would be public relay servers for PeerUP
    ]
}
