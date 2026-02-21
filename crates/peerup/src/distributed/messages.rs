//! Message types for distributed peer data support.
//!
//! These message types enable peers to query, sync, and notify each other
//! about data storage and retrieval.

use serde::{Deserialize, Serialize};

/// Request to query peer data from a remote peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQueryRequest {
    /// Get data since this timestamp (Unix timestamp)
    pub since_timestamp: i64,

    /// Optional: filter by data type
    pub data_type: Option<String>,

    /// Maximum number of items to return
    pub limit: u64,
}

/// Response containing peer data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQueryResponse {
    /// The actual data items
    pub data: Vec<PeerData>,

    /// Which peer sent this response
    pub from_peer_id: String,

    /// Total count of available data items
    pub total_count: u64,

    /// Whether there are more items available
    pub has_more: bool,
}

/// Generic peer data container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerData {
    /// Unique identifier for this data item
    pub id: String,

    /// Timestamp when data was created (Unix timestamp)
    pub timestamp: i64,

    /// Which peer originally created this data
    pub source_peer_id: String,

    /// Data type (e.g., "monitoring_result", "message", "file")
    pub data_type: String,

    /// The actual data payload (JSON, bytes, etc.)
    pub payload: Vec<u8>,

    /// Ed25519 signature of the payload
    pub signature: String,

    /// Public key for signature verification
    pub public_key: String,
}

/// Notification that a peer has synced data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCompletionNotification {
    /// Which peer's data was synced
    pub source_peer_id: String,

    /// Synced data up to this timestamp
    pub synced_until_timestamp: i64,

    /// How many items were synced
    pub synced_count: u64,

    /// When the sync was completed
    pub sync_timestamp: i64,
}

impl PeerData {
    /// Create new peer data
    pub fn new(
        id: String,
        timestamp: i64,
        source_peer_id: String,
        data_type: String,
        payload: Vec<u8>,
        signature: String,
        public_key: String,
    ) -> Self {
        Self {
            id,
            timestamp,
            source_peer_id,
            data_type,
            payload,
            signature,
            public_key,
        }
    }

    /// Verify the signature on this data
    pub fn verify_signature(&self) -> bool {
        // TODO: Implement Ed25519 signature verification
        // For now, return true (implement in storage layer)
        true
    }
}

impl DataQueryRequest {
    /// Create a new query request
    pub fn new(since_timestamp: i64, data_type: Option<String>, limit: u64) -> Self {
        Self {
            since_timestamp,
            data_type,
            limit,
        }
    }
}

impl DataQueryResponse {
    /// Create a new query response
    pub fn new(data: Vec<PeerData>, from_peer_id: String, total_count: u64) -> Self {
        let has_more = total_count > data.len() as u64;
        Self {
            data,
            from_peer_id,
            total_count,
            has_more,
        }
    }
}

impl SyncCompletionNotification {
    /// Create a new sync notification
    pub fn new(source_peer_id: String, synced_until_timestamp: i64, synced_count: u64) -> Self {
        Self {
            source_peer_id,
            synced_until_timestamp,
            synced_count,
            sync_timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

// ===== Public Monitor Coordination Messages =====

use super::{OrchestrationSchedule, PublicMonitorGroup};

/// Messages exchanged between peers for public monitor coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublicMonitorMessage {
    /// Announce a new public monitor group
    Announce {
        domain: String,
        display_name: String,
        creator_peer_id: String,
    },

    /// Request to join an existing monitor group
    Join { domain: String, peer_id: String },

    /// Notification that a peer is leaving
    Leave { domain: String, peer_id: String },

    /// Propose schedule update (triggers consensus vote)
    ScheduleUpdate {
        domain: String,
        schedule: OrchestrationSchedule,
    },

    /// Query for group information
    GroupQuery { domain: String },

    /// Response to group query
    GroupResponse {
        domain: String,
        group: Option<PublicMonitorGroup>,
    },
}

impl PublicMonitorMessage {
    /// Get the domain this message refers to
    pub fn domain(&self) -> &str {
        match self {
            Self::Announce { domain, .. }
            | Self::Join { domain, .. }
            | Self::Leave { domain, .. }
            | Self::ScheduleUpdate { domain, .. }
            | Self::GroupQuery { domain }
            | Self::GroupResponse { domain, .. } => domain,
        }
    }

    /// Get the peer ID if message is from a specific peer
    pub fn peer_id(&self) -> Option<&str> {
        match self {
            Self::Announce {
                creator_peer_id, ..
            } => Some(creator_peer_id),
            Self::Join { peer_id, .. } | Self::Leave { peer_id, .. } => Some(peer_id),
            _ => None,
        }
    }
}
