//! Relay support for NAT traversal in PeerUP.
//!
//! This module provides functionality for NAT traversal using libp2p relay.

pub mod config;
pub mod servers;

// Re-export main functions
pub use config::{configure_relay_client, configure_relay_server, create_dev_relay};
pub use servers::{add_relay_servers, default_relay_servers, validate_relay_addresses};
