/// P2P networking layer using PeerUP
///
/// This module handles:
/// - Joining the decentralized network
/// - Sharing monitoring results with peers
/// - Receiving results from other peers
/// - Peer discovery and coordination
/// - DHT storage and retrieval
/// - GossipSub topic management
pub mod messages;
pub mod network;
pub mod topics;

#[allow(unused_imports)]
pub use messages::{P2PCommand, P2PEvent, PeerResult};
pub use network::P2PNetwork;
