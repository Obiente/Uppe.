//! Network module for PeerUP.
//!
//! This module contains all networking-related functionality including
//! the main network behaviour, events, and state management.

pub mod behaviour;
pub mod conversions;
pub mod events;
pub mod helpers;
pub mod state;

// Re-export main types
pub use behaviour::PeerUPBehaviour;
pub use events::PeerUPEvent;
pub use helpers::{create_test_multiaddr, extract_peer_id_from_multiaddr, validate_multiaddr};
pub use state::PeerUPBehaviourState;
