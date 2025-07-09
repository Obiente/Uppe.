//! PeerUP - Peer-to-peer probe coordination system for Uppe
//!
//! This library provides functionality for distributed uptime monitoring
//! through a peer-to-peer network using libp2p.

pub mod discovery;
pub mod handlers;
pub mod network;
pub mod node;
pub mod protocol;
pub mod relay;
pub mod transport;

// Re-export main types
/// Re-export common error types
pub use anyhow;
pub use network::{PeerUPBehaviour, PeerUPBehaviourState, PeerUPEvent};
pub use node::{NodeConfig, PeerNode};
pub use protocol::{ProbeCodec, ProbeRequest, ProbeResponse, PROBE_PROTOCOL};

/// PeerUP result type using anyhow for error handling
pub type Result<T,> = anyhow::Result<T,>;

/// The version of the PeerUP protocol
pub const PROTOCOL_VERSION: &str = "1.0";

/// Default port range for PeerUP nodes
pub const DEFAULT_PORT_RANGE: (u16, u16,) = (9000, 9010,);
