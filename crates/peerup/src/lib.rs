//! PeerUP - Peer-to-peer coordination system for distributed communities
//!
//! This library provides functionality for distributed peer-to-peer networking
//! with built-in support for community resilience through peer data storage.
//!
//! ## Core Features
//!
//! - **P2P Networking**: Built on libp2p with Kademlia DHT, GossipSub, mDNS
//! - **Peer Data Support**: Store data for peers during their downtime
//! - **Auto-Sync**: Automatically recover data from peers on startup
//! - **Retention Management**: Configurable automatic cleanup of old data
//!
//! ## Use Cases
//!
//! PeerUP is a general-purpose P2P layer for any distributed system where
//! participants support each other:
//!
//! - Distributed monitoring (Uppe.)
//! - Distributed messaging
//! - Distributed file sharing
//! - Community data backups
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  Application Layer (e.g., Uppe.)   │
//! │  - Uses PeerUP for P2P features    │
//! └─────────────┬───────────────────────┘
//!               │
//! ┌─────────────v───────────────────────┐
//! │         PeerUP Layer                │
//! │  ┌──────────────────────────────┐   │
//! │  │ Distributed Module           │   │
//! │  │ - Peer data storage          │   │
//! │  │ - Sync manager               │   │
//! │  │ - Retention & cleanup        │   │
//! │  └──────────────────────────────┘   │
//! │  ┌──────────────────────────────┐   │
//! │  │ Network Layer (libp2p)       │   │
//! │  │ - GossipSub, Kademlia, mDNS  │   │
//! │  └──────────────────────────────┘   │
//! └─────────────────────────────────────┘
//! ```

pub mod discovery;
pub mod distributed;
pub mod handlers;
pub mod network;
pub mod node;
pub mod protocol;
pub mod relay;
pub mod transport;
pub mod crypto;
pub mod trust;

// Re-export main types
/// Re-export common error types
pub use anyhow;
pub use network::{PeerUPBehaviour, PeerUPBehaviourState, PeerUPEvent};
pub use node::{core::gossipsub::MONITORING_RESULTS_TOPIC, NodeConfig, PeerNode};
pub use protocol::{ProbeCodec, ProbeRequest, ProbeResponse, PROBE_PROTOCOL};

// Re-export commonly needed libp2p types for consumers
pub mod swarm {
    pub use libp2p::swarm::SwarmEvent;
}

/// PeerUP result type using anyhow for error handling
pub type Result<T> = anyhow::Result<T>;

/// The version of the PeerUP protocol
pub const PROTOCOL_VERSION: &str = "1.0";

/// Default port range for PeerUP nodes
pub const DEFAULT_PORT_RANGE: (u16, u16) = (9000, 9010);
