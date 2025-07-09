//! Node module for PeerUP.
//!
//! This module contains all node-related functionality including
//! configuration, transport setup, and event handling.

pub mod config;
pub mod core;
pub mod crypto;
pub mod events;

// Re-export main types
pub use config::{NodeConfig, NodeConfigBuilder};
pub use core::PeerNode;
pub use crypto::{load_or_generate_keypair, generate_keypair, save_keypair, load_keypair};
pub use events::{handle_peerup_event, handle_swarm_event};
