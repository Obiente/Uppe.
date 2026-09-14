//! Main PeerUP node implementation.
//!
//! This module contains the core PeerNode struct and its methods.

pub mod dht;
pub mod gossipsub;
mod node_methods;
mod peer_node;
mod run;

pub use peer_node::PeerNode;
