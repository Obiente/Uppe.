//! DHT operations for distributed data storage.
//!
//! This module provides actual Kademlia DHT put/get operations for:
//! - Public monitor discovery
//! - Encrypted private monitor results
//! - Peer-to-peer data synchronization

use anyhow::{anyhow, Result};
use libp2p::kad::{Quorum, Record, RecordKey};
use tracing::{debug, info};

use super::PeerNode;

impl PeerNode {
    /// Store a record in the DHT
    ///
    /// # Arguments
    /// * `key` - DHT record key
    /// * `value` - Data to store (will be replicated across network)
    /// * `quorum` - How many peers must confirm storage (default: One for speed, All for reliability)
    ///
    /// # Returns
    /// Query ID for tracking the operation
    pub fn dht_put_record(
        &mut self,
        key: RecordKey,
        value: Vec<u8>,
        quorum: Option<Quorum>,
    ) -> Result<libp2p::kad::QueryId> {
        let kademlia = self
            .swarm
            .behaviour_mut()
            .kademlia
            .as_mut()
            .ok_or_else(|| anyhow!("Kademlia is not enabled"))?;

        // Set 7-day expiration for DHT records (especially for private monitor results)
        // This helps with DHT cleanup and prevents stale data accumulation
        let expiration = std::time::SystemTime::now()
            .checked_add(std::time::Duration::from_secs(7 * 24 * 3600)) // 7 days
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| std::time::Instant::now() + d);

        let record = Record {
            key: key.clone(),
            value,
            publisher: Some(self.peer_id),
            expires: expiration,
        };

        let query_id = kademlia.put_record(record, quorum.unwrap_or(Quorum::One))?;

        info!(
            "DHT put_record initiated: key={:?}, query_id={:?}",
            String::from_utf8_lossy(key.as_ref()),
            query_id
        );

        Ok(query_id)
    }

    /// Retrieve a record from the DHT
    ///
    /// # Arguments
    /// * `key` - DHT record key to lookup
    ///
    /// # Returns
    /// Query ID for tracking the operation (result will come via event)
    pub fn dht_get_record(&mut self, key: RecordKey) -> Result<libp2p::kad::QueryId> {
        let kademlia = self
            .swarm
            .behaviour_mut()
            .kademlia
            .as_mut()
            .ok_or_else(|| anyhow!("Kademlia is not enabled"))?;

        let query_id = kademlia.get_record(key.clone());

        debug!(
            "DHT get_record initiated: key={:?}, query_id={:?}",
            String::from_utf8_lossy(key.as_ref()),
            query_id
        );

        Ok(query_id)
    }

    /// Store a record in the DHT (simplified API - takes string key)
    ///
    /// Convenience wrapper around dht_put_record for applications that work with string keys.
    pub fn dht_put_record_simple(&mut self, key: &str, value: Vec<u8>) -> Result<libp2p::kad::QueryId> {
        let record_key = RecordKey::new(&key.as_bytes());
        self.dht_put_record(record_key, value, None)
    }

    /// Retrieve a record from the DHT (simplified API - takes string key)
    ///
    /// Convenience wrapper around dht_get_record for applications that work with string keys.
    pub fn dht_get_record_simple(&mut self, key: &str) -> Result<libp2p::kad::QueryId> {
        let record_key = RecordKey::new(&key.as_bytes());
        self.dht_get_record(record_key)
    }

    /// Remove a record from the local DHT store
    ///
    /// Note: This only removes from local node, not from network
    pub fn dht_remove_record(&mut self, key: &RecordKey) {
        if let Some(kademlia) = self.swarm.behaviour_mut().kademlia.as_mut() {
            kademlia.remove_record(key);
            debug!("Removed DHT record locally: {:?}", String::from_utf8_lossy(key.as_ref()));
        }
    }

    /// Start providing a record in the DHT (announce we have this data)
    ///
    /// Useful for large data that we don't want to store in DHT itself,
    /// but we want to be discoverable as a provider.
    pub fn dht_start_providing(&mut self, key: RecordKey) -> Result<libp2p::kad::QueryId> {
        let kademlia = self
            .swarm
            .behaviour_mut()
            .kademlia
            .as_mut()
            .ok_or_else(|| anyhow!("Kademlia is not enabled"))?;

        let query_id = kademlia.start_providing(key.clone())?;

        debug!(
            "DHT start_providing: key={:?}, query_id={:?}",
            String::from_utf8_lossy(key.as_ref()),
            query_id
        );

        Ok(query_id)
    }

    /// Find providers for a record (who has this data?)
    pub fn dht_get_providers(&mut self, key: RecordKey) -> libp2p::kad::QueryId {
        let kademlia = self
            .swarm
            .behaviour_mut()
            .kademlia
            .as_mut()
            .expect("Kademlia should be enabled");

        let query_id = kademlia.get_providers(key.clone());

        debug!(
            "DHT get_providers: key={:?}, query_id={:?}",
            String::from_utf8_lossy(key.as_ref()),
            query_id
        );

        query_id
    }

    /// Stop providing a record (announce we no longer have this data)
    pub fn dht_stop_providing(&mut self, key: &RecordKey) {
        if let Some(kademlia) = self.swarm.behaviour_mut().kademlia.as_mut() {
            kademlia.stop_providing(key);
            debug!(
                "DHT stop_providing: key={:?}",
                String::from_utf8_lossy(key.as_ref())
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeConfig;

    #[tokio::test]
    async fn test_dht_put_get() {
        let config = NodeConfig::builder().enable_kademlia().build();
        let mut node = PeerNode::with_config(config).await.unwrap();

        let key = RecordKey::new(&b"test-key");
        let value = b"test-value".to_vec();

        // Put record
        let put_query_id = node.dht_put_record(key.clone(), value.clone(), None).unwrap();
        // QueryId is opaque, just verify it was created
        let _ = put_query_id;

        // Get record
        let get_query_id = node.dht_get_record(key.clone()).unwrap();
        let _ = get_query_id;

        // Remove record
        node.dht_remove_record(&key);
    }
}
