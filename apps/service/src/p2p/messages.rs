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

/// Request to query results from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsQueryRequest {
    /// Query results since this timestamp (Unix seconds)
    pub since_timestamp: u64,
    /// Optional: limit results to a specific monitor UUID
    pub monitor_uuid: Option<String>,
    /// Maximum number of results to return
    pub limit: usize,
}

/// Response containing monitoring results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsQueryResponse {
    /// Monitoring results matching the query
    pub results: Vec<QueryResult>,
    /// Whether there are more results available
    pub has_more: bool,
}

/// A monitoring result in query response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub monitor_uuid: String,
    pub timestamp: u64,
    pub status: String,
    pub latency_ms: Option<u64>,
    pub error_message: Option<String>,
    pub peer_id: String,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

/// Notification that a peer has synced our results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCompletionNotification {
    /// Peer that synced the data
    pub syncing_peer_id: String,
    /// Results synced up to this timestamp (Unix seconds)
    pub synced_until_timestamp: u64,
    /// Results from which monitors (empty = all)
    pub monitor_uuids: Vec<String>,
}

/// Request for a peer to help monitor a private service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelperAssignmentRequest {
    /// Monitor UUID
    pub monitor_uuid: String,
    /// Target URL to monitor
    pub target: String,
    /// Check type (http, https, tcp, icmp)
    pub check_type: String,
    /// Check interval in seconds
    pub interval_seconds: u64,
    /// Timeout in seconds
    pub timeout_seconds: u64,
    /// Owner's peer ID
    pub owner_peer_id: String,
    /// Owner's X25519 public key (for encrypting results)
    pub owner_public_key: [u8; 32],
    /// Target helper peer ID (who should receive this assignment)
    pub helper_peer_id: String,
    /// When this assignment was created
    pub assigned_at: i64,
}

/// Response to helper assignment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HelperAssignmentResponse {
    /// Helper accepted the assignment
    Accepted {
        monitor_uuid: String,
        helper_peer_id: String,
    },
    /// Helper rejected (rate limited, too many assignments, etc.)
    Rejected {
        monitor_uuid: String,
        reason: String,
    },
}

/// Commands sent to the P2P node
#[derive(Debug, Clone)]
pub enum P2PCommand {
    /// Publish a monitoring result to the network (public monitors only)
    PublishResult(CheckResult),
    /// Publish an encrypted result to private topic (private monitors)
    PublishEncryptedResult(crate::crypto::EncryptedResult),
    /// Request results from a specific peer
    QueryResults(ResultsQueryRequest),
    /// Notify peer about synced data (for cleanup)
    NotifySyncComplete(SyncCompletionNotification),
    /// Assign a helper peer to monitor a private service
    AssignHelper {
        helper_peer_id: String,
        request: HelperAssignmentRequest,
    },
    /// Send a helper assignment response (accept/reject)
    SendHelperResponse(HelperAssignmentResponse),
    /// Publish a record to DHT
    PublishDHTRecord {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    /// Get a record from DHT
    GetDHTRecord {
        key: Vec<u8>,
    },
    /// Subscribe to monitoring results
    #[allow(dead_code)] // Future API
    Subscribe,
    /// Unsubscribe from monitoring results
    #[allow(dead_code)] // Future API
    Unsubscribe,
    /// Publish a message to a named GossipSub topic
    PublishToTopic {
        topic: String,
        data: Vec<u8>,
    },
    /// Shutdown the P2P node
    #[allow(dead_code)] // Future API
    Shutdown,
}

/// Events received from the P2P node
#[derive(Debug, Clone)]
pub enum P2PEvent {
    /// A monitoring result was received from a peer
    ResultReceived { peer_id: String, result: Box<PeerResult> },
    /// Results retrieved from a peer query
    ResultsQueried { peer_id: String, results: Box<ResultsQueryResponse> },
    /// Peer notified us they synced our data
    SyncCompleted { peer_id: String, notification: Box<SyncCompletionNotification> },
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
    /// DHT record was successfully published
    DHTRecordPublished { key: Vec<u8> },
    /// DHT record was received (from GET operation)
    DHTRecordReceived { key: Vec<u8>, record: Vec<u8> },
    /// DHT record was not found (from GET operation)
    DHTRecordNotFound { key: Vec<u8> },
    /// DHT record publish failed
    DHTRecordPublishFailed { key: Vec<u8>, error: String },
    /// Received helper assignment request (we're being asked to help monitor)
    HelperAssignmentRequested { from_peer: String, request: Box<HelperAssignmentRequest> },
    /// Received helper assignment response (our request was accepted/rejected)
    HelperAssignmentResponse { from_peer: String, response: Box<HelperAssignmentResponse> },
    /// Received encrypted result from helper peer (private monitor)
    EncryptedResultReceived { from_peer: String, result: Box<crate::crypto::EncryptedResult> },
    /// Snapshot of the DHT routing table (kbuckets)
    DhtSnapshot { snapshot: Box<DhtSnapshot> },
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

/// DHT routing table snapshot types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtSnapshot {
    /// Local libp2p PeerId string
    pub local_peer_id: String,
    /// Buckets indexed by distance (0..=255 typically)
    pub buckets: Vec<DhtBucket>,
    /// When the snapshot was created (unix seconds)
    pub captured_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtBucket {
    /// k-bucket index / distance
    pub index: usize,
    /// Entries in this bucket
    pub peers: Vec<DhtPeerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtPeerEntry {
    /// PeerId string
    pub peer_id: String,
    /// Known multiaddrs (if available)
    pub addrs: Vec<String>,
    /// State info if available (e.g., Connected/Disconnected)
    pub state: Option<String>,
}
