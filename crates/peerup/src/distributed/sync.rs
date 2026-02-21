//! Sync manager for distributed peer data.
//!
//! Handles automatic synchronization of peer data on startup and
//! periodic cleanup of expired data.

use super::{
    storage::PeerDataStorage, DataQueryRequest, DataQueryResponse, DistributedConfig, PeerData,
    SyncCompletionNotification,
};
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Manages syncing and cleanup of peer data
pub struct SyncManager<S: PeerDataStorage> {
    storage: Arc<S>,
    config: DistributedConfig,
}

impl<S: PeerDataStorage> SyncManager<S> {
    /// Create a new sync manager
    pub fn new(storage: Arc<S>, config: DistributedConfig) -> Self {
        Self { storage, config }
    }

    /// Run sync on startup
    pub async fn sync_on_startup(
        &self,
        query_fn: impl Fn(String, DataQueryRequest) -> futures::future::BoxFuture<
            'static,
            Result<DataQueryResponse, Box<dyn std::error::Error>>,
        >,
        peer_ids: Vec<String>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        if !self.config.auto_sync {
            return Ok(0);
        }

        let mut total_synced = 0;

        // Get last known timestamp from storage
        let last_timestamp = self.get_last_timestamp().await?;

        // Query each peer
        for peer_id in peer_ids {
            let request = DataQueryRequest::new(last_timestamp, None, 10_000);

            match query_fn(peer_id.clone(), request).await {
                Ok(response) => {
                    // Store each data item
                let mut synced_until = 0;
                    for data in response.data {
                        synced_until = data.timestamp;
                        self.store_peer_data(data).await?;
                        total_synced += 1;
                    }

                    // Mark as synced
                    self.storage
                        .mark_as_synced(&peer_id, synced_until)
                        .await?;
                }
                Err(e) => {
                    log::warn!("Failed to sync from peer {}: {}", peer_id, e);
                }
            }
        }

        Ok(total_synced)
    }

    /// Start periodic cleanup task
    pub async fn start_cleanup_task(self: Arc<Self>) {
        let mut cleanup_interval = interval(Duration::from_secs(3600)); // Every hour

        loop {
            cleanup_interval.tick().await;

            match self.storage.cleanup_expired(self.config.retention_days).await {
                Ok(deleted) => {
                    if deleted > 0 {
                        log::info!("Cleaned up {} expired peer data items", deleted);
                    }
                }
                Err(e) => {
                    log::error!("Failed to cleanup expired data: {}", e);
                }
            }
        }
    }

    /// Store peer data with verification
    async fn store_peer_data(&self, data: PeerData) -> Result<(), Box<dyn std::error::Error>> {
        // Verify signature
        if !data.verify_signature() {
            return Err("Invalid signature".into());
        }

        self.storage
            .store_peer_data(
                data.id,
                data.source_peer_id,
                data.data_type,
                data.timestamp,
                data.payload,
                data.signature,
                data.public_key,
            )
            .await
    }

    /// Get the last known timestamp from storage
    async fn get_last_timestamp(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let results = self.storage.query_peer_data(0, None, 1).await?;
        Ok(results.first().map_or(0, |d| d.timestamp))
    }

    /// Create a sync completion notification
    pub fn create_sync_notification(
        &self,
        source_peer_id: String,
        synced_until_timestamp: i64,
        synced_count: u64,
    ) -> SyncCompletionNotification {
        SyncCompletionNotification::new(source_peer_id, synced_until_timestamp, synced_count)
    }
}
