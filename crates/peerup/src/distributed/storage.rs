//! Peer data storage abstraction.
//!
//! This module provides traits and types for storing peer data with
//! automatic retention and cleanup.

use super::DistributedConfig;
use async_trait::async_trait;
use std::error::Error;

/// Trait for peer data storage backends
#[async_trait]
pub trait PeerDataStorage: Send + Sync {
    /// Store peer data
    async fn store_peer_data(&self, item: super::PeerData) -> Result<(), Box<dyn Error>>;

    /// Query peer data since a timestamp
    async fn query_peer_data(
        &self,
        since_timestamp: i64,
        data_type: Option<String>,
        limit: u64,
    ) -> Result<Vec<super::PeerData>, Box<dyn Error>>;

    /// Mark peer data as synced (safe for cleanup)
    async fn mark_as_synced(
        &self,
        source_peer_id: &str,
        until_timestamp: i64,
    ) -> Result<u64, Box<dyn Error>>;

    /// Clean up expired peer data
    async fn cleanup_expired(&self, retention_days: u64) -> Result<u64, Box<dyn Error>>;

    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats, Box<dyn Error>>;
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total number of peer data items
    pub total_items: u64,

    /// Number of items marked as synced
    pub synced_items: u64,

    /// Total storage size in bytes
    pub total_bytes: u64,

    /// Number of unique source peers
    pub unique_peers: u64,
}

/// In-memory storage implementation (for testing)
pub struct MemoryStorage {
    data: std::sync::Arc<tokio::sync::RwLock<Vec<StoredPeerData>>>,
    config: DistributedConfig,
}

#[derive(Debug, Clone)]
struct StoredPeerData {
    id: String,
    source_peer_id: String,
    data_type: String,
    timestamp: i64,
    payload: Vec<u8>,
    signature: String,
    public_key: String,
    synced: bool,
    retention_until: i64,
}

impl MemoryStorage {
    pub fn new(config: DistributedConfig) -> Self {
        Self { data: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())), config }
    }
}

#[async_trait]
impl PeerDataStorage for MemoryStorage {
    async fn store_peer_data(&self, item: super::PeerData) -> Result<(), Box<dyn Error>> {
        if !item.verify_signature() || item.payload.len() > 1_048_576 {
            return Err("Invalid peer data".into());
        }
        let super::PeerData {
            id,
            source_peer_id,
            data_type,
            timestamp,
            payload,
            signature,
            public_key,
        } = item;
        let retention_until =
            chrono::Utc::now().timestamp() + (self.config.retention_days as i64 * 86400);

        let stored = StoredPeerData {
            id,
            source_peer_id,
            data_type,
            timestamp,
            payload,
            signature,
            public_key,
            synced: false,
            retention_until,
        };

        let mut data = self.data.write().await;

        if data
            .iter()
            .any(|item| item.id == stored.id && item.source_peer_id == stored.source_peer_id)
        {
            return Ok(());
        }
        let peer_count =
            data.iter().filter(|item| item.source_peer_id == stored.source_peer_id).count() as u64;
        let used = data.iter().map(|item| item.payload.len() as u64 + 512).sum::<u64>();
        if peer_count >= self.config.max_items_per_peer
            || used.saturating_add(stored.payload.len() as u64 + 512)
                > self.config.max_storage_bytes
        {
            return Err("Peer storage limit reached".into());
        }
        data.push(stored);
        Ok(())
    }

    async fn query_peer_data(
        &self,
        since_timestamp: i64,
        data_type: Option<String>,
        limit: u64,
    ) -> Result<Vec<super::PeerData>, Box<dyn Error>> {
        let data = self.data.read().await;
        let mut results: Vec<_> = data
            .iter()
            .filter(|item| {
                item.timestamp > since_timestamp
                    && data_type.as_ref().is_none_or(|dt| dt == &item.data_type)
            })
            .map(|item| super::PeerData {
                id: item.id.clone(),
                timestamp: item.timestamp,
                source_peer_id: item.source_peer_id.clone(),
                data_type: item.data_type.clone(),
                payload: item.payload.clone(),
                signature: item.signature.clone(),
                public_key: item.public_key.clone(),
            })
            .collect();

        results.sort_by_key(|item| (std::cmp::Reverse(item.timestamp), item.id.clone()));
        results.truncate(limit.min(10_000) as usize);
        Ok(results)
    }

    async fn mark_as_synced(
        &self,
        source_peer_id: &str,
        until_timestamp: i64,
    ) -> Result<u64, Box<dyn Error>> {
        let mut data = self.data.write().await;
        let mut count = 0;
        for item in data.iter_mut() {
            if item.source_peer_id == source_peer_id && item.timestamp <= until_timestamp {
                item.synced = true;
                count += 1;
            }
        }
        Ok(count)
    }

    async fn cleanup_expired(&self, _retention_days: u64) -> Result<u64, Box<dyn Error>> {
        let now = chrono::Utc::now().timestamp();
        let mut data = self.data.write().await;
        let before_count = data.len();

        data.retain(|item| !(item.synced && item.retention_until < now));

        Ok((before_count - data.len()) as u64)
    }

    async fn get_stats(&self) -> Result<StorageStats, Box<dyn Error>> {
        let data = self.data.read().await;
        let total_items = data.len() as u64;
        let synced_items = data.iter().filter(|item| item.synced).count() as u64;
        let total_bytes: u64 = data.iter().map(|item| item.payload.len() as u64).sum();
        let unique_peers = data
            .iter()
            .map(|item| item.source_peer_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .len() as u64;

        Ok(StorageStats { total_items, synced_items, total_bytes, unique_peers })
    }
}
