//! Distributed peer data support module.
//!
//! This module provides functionality for peers to store data for each other,
//! enabling resilient distributed systems where data is preserved during
//! individual peer downtime.
//!
//! ## Core Concepts
//!
//! - **Peer Data**: Any data stored by one peer on behalf of another
//! - **Retention**: How long to keep peer data before cleanup
//! - **Sync**: Recovering peer data when coming back online
//! - **Cleanup**: Automatic deletion of expired peer data
//! - **Visibility**: Public (community-owned) vs Private (owner-controlled)
//!
//! ## Visibility Types
//!
//! Applications can define their own visibility types. The visibility module
//! provides examples for monitoring applications, but the concepts apply broadly:
//!
//! - **Public**: Community-owned, discoverable, long-term retention
//! - **Private**: Owner-controlled, temporary storage, privacy-preserving
//! - **Internal**: Owner-only, no peer orchestration
//!
//! ## Use Cases
//!
//! - Distributed monitoring (Uppe.)
//! - Distributed messaging
//! - Distributed file sharing
//! - Any P2P system needing resilience

pub mod consensus;
pub mod dht;
pub mod metadata;
pub mod messages;
pub mod storage;
pub mod sync;
pub mod visibility;

pub use consensus::{ConsensusManager, ConsensusState, OrchestrationVote, RateLimitState};
pub use dht::{PublicMonitorDHT, PublicMonitorRegistry};
pub use metadata::{PeerLocation, PeerMetadataDHT, PeerRateLimits, PeerTrustScore};
pub use messages::{
    DataQueryRequest, DataQueryResponse, PeerData, PublicMonitorMessage,
    SyncCompletionNotification,
};
pub use storage::PeerDataStorage;
pub use sync::SyncManager;
pub use visibility::{MonitorVisibility, OrchestrationSchedule, PublicMonitorGroup, RetentionPolicy};

/// Trait for types that can be stored as peer data
pub trait StorableData: Send + Sync {
    /// Serialize data to bytes
    fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

    /// Deserialize data from bytes
    fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;

    /// Get the timestamp of this data (for retention)
    fn timestamp(&self) -> i64;

    /// Get the source peer ID (who created this data)
    fn source_peer_id(&self) -> String;

    /// Verify cryptographic signature (if applicable)
    fn verify(&self) -> bool {
        true // Default: no verification needed
    }
}

/// Configuration for distributed peer data support
#[derive(Debug, Clone)]
pub struct DistributedConfig {
    /// Enable peer data support
    pub enabled: bool,

    /// Retention period in days
    pub retention_days: u64,

    /// Auto-sync on startup
    pub auto_sync: bool,

    /// Maximum data items per peer
    pub max_items_per_peer: u64,

    /// Maximum storage size (bytes)
    pub max_storage_bytes: u64,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: 7,
            auto_sync: true,
            max_items_per_peer: 10_000,
            max_storage_bytes: 100_000_000, // 100 MB
        }
    }
}

impl DistributedConfig {
    /// Create config from NodeConfig
    pub fn from_node_config(config: &crate::NodeConfig) -> Self {
        Self {
            enabled: config.enable_peer_data_support,
            retention_days: config.peer_data_retention_days,
            auto_sync: config.auto_sync_on_startup,
            ..Default::default()
        }
    }
}
