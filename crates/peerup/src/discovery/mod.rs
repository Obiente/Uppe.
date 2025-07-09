//! Peer discovery mechanisms for PeerUP.
//!
//! This module provides peer discovery using mDNS and Kademlia DHT.

pub mod kademlia;
pub mod mdns;

// Re-export main functions
pub use kademlia::{configure_kademlia, add_bootstrap_peers, create_dev_kademlia};
pub use mdns::{configure_mdns, create_dev_mdns, is_mdns_available};
