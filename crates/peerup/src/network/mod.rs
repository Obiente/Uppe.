//! Network module for PeerUP.
//!
//! This module contains all networking-related functionality including
//! the main network behaviour, events, and state management.

pub mod behaviour;
pub mod events;
pub mod conversions;
pub mod state;
pub mod helpers;

// Re-export main types
pub use behaviour::PeerUPBehaviour;
pub use events::PeerUPEvent;
pub use state::PeerUPBehaviourState;
pub use helpers::{extract_peer_id_from_multiaddr, validate_multiaddr, create_test_multiaddr};
