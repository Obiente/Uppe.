/// P2P networking layer using PeerUP
///
/// This module handles:
/// - Joining the decentralized network
/// - Sharing monitoring results with peers
/// - Receiving results from other peers
/// - Peer discovery and coordination
pub mod network;
pub mod receiving;
pub mod sharing;

pub use network::P2PNetwork;
