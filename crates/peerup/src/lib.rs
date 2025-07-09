//! PeerUP - Peer-to-peer probe coordination system for Uppe
//! 
//! This library provides functionality for distributed uptime monitoring
//! through a peer-to-peer network using libp2p.

pub mod node;
pub mod network;
pub mod protocol;
pub mod discovery;
pub mod relay;
pub mod handlers;
pub mod transport;

// Re-export main types
pub use node::{PeerNode, NodeConfig};
pub use network::{PeerUPBehaviour, PeerUPEvent, PeerUPBehaviourState};
pub use protocol::{ProbeRequest, ProbeResponse, ProbeCodec, PROBE_PROTOCOL};

/// Re-export common error types
pub use anyhow;

/// PeerUP result type using anyhow for error handling
pub type Result<T> = anyhow::Result<T>;

/// The version of the PeerUP protocol
pub const PROTOCOL_VERSION: &str = "1.0";

/// Default port range for PeerUP nodes
pub const DEFAULT_PORT_RANGE: (u16, u16) = (9000, 9010);
