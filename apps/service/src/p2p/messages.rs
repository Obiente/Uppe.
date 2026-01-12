/// P2P messaging types for communication between the node and service
use serde::{Deserialize, Serialize};

use crate::monitoring::types::CheckResult;

/// Signed message published to the P2P network
/// This wraps a CheckResult with signature and public key for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMessage {
    /// The monitoring result
    pub result: CheckResult,
    /// Ed25519 public key of the sender (32 bytes)
    pub public_key: [u8; 32],
}

/// Commands sent to the P2P node
#[derive(Debug, Clone)]
pub enum P2PCommand {
    /// Publish a monitoring result to the network
    PublishResult(CheckResult),
    /// Subscribe to monitoring results
    #[allow(dead_code)] // Future API
    Subscribe,
    /// Unsubscribe from monitoring results
    #[allow(dead_code)] // Future API
    Unsubscribe,
    /// Shutdown the P2P node
    #[allow(dead_code)] // Future API
    Shutdown,
}

/// Events received from the P2P node
#[derive(Debug, Clone)]
pub enum P2PEvent {
    /// A monitoring result was received from a peer
    ResultReceived { peer_id: String, result: Box<PeerResult> },
    /// Successfully subscribed to results
    Subscribed,
    /// Successfully unsubscribed from results
    Unsubscribed,
    /// A peer connected
    PeerConnected(String),
    /// A peer disconnected
    PeerDisconnected(String),
    /// Node started successfully
    Started { peer_id: String },
    /// Node encountered an error
    Error(String),
}

/// A monitoring result received from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerResult {
    /// The monitoring result
    pub result: CheckResult,
    /// Signature of the result (for verification)
    pub signature: Option<Vec<u8>>,
    /// Public key of the peer that sent this result (for verification)
    pub public_key: Option<Vec<u8>>,
    /// Peer ID that sent this result
    pub peer_id: String,
    /// Timestamp when received
    pub received_at: std::time::SystemTime,
}
