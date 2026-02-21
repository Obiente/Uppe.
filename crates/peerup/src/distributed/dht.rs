//! DHT-based public monitor discovery.
//!
//! Public monitors are discoverable via Kademlia DHT, allowing anyone
//! to find and display community monitoring data.

use super::visibility::PublicMonitorGroup;
use libp2p::kad::{Record, RecordKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Public monitor registry stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicMonitorRegistry {
    /// Map of domain -> monitor group metadata
    pub monitors: HashMap<String, PublicMonitorGroup>,

    /// When this registry was last updated
    pub last_updated: i64,
}

impl PublicMonitorRegistry {
    /// Create DHT key for a public domain
    pub fn dht_key(domain: &str) -> RecordKey {
        RecordKey::new(&format!("/uppe/public-monitor/{}", domain))
    }

    /// Serialize registry to DHT record
    pub fn to_record(&self) -> Result<Record, Box<dyn std::error::Error>> {
        let value = serde_json::to_vec(&self)?;
        // Key will be set by caller based on specific domain
        Ok(Record {
            key: RecordKey::new(&[]), // Placeholder
            value,
            publisher: None,
            expires: None,
        })
    }

    /// Deserialize registry from DHT record
    pub fn from_record(record: &Record) -> Result<Self, Box<dyn std::error::Error>> {
        let registry: Self = serde_json::from_slice(&record.value)?;
        Ok(registry)
    }
}

/// DHT operations for public monitors
pub struct PublicMonitorDHT {
    local_cache: tokio::sync::RwLock<HashMap<String, PublicMonitorGroup>>,
}

impl PublicMonitorDHT {
    pub fn new() -> Self {
        Self {
            local_cache: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Publish a public monitor group to DHT
    ///
    /// This creates a PublicMonitorRegistry, serializes it, and stores it in the DHT.
    /// The record will be replicated across 20 peers for redundancy.
    ///
    /// NOTE: Actual DHT put_record call must be made by caller via PeerNode::dht_put_record()
    /// This method prepares the data and updates local cache.
    pub async fn publish_monitor(
        &self,
        domain: String,
        group: PublicMonitorGroup,
    ) -> Result<(RecordKey, Vec<u8>), Box<dyn std::error::Error>> {
        // Update local cache
        self.local_cache.write().await.insert(domain.clone(), group.clone());

        // Create registry for this domain
        let registry = PublicMonitorRegistry {
            monitors: [(domain.clone(), group)].into_iter().collect(),
            last_updated: chrono::Utc::now().timestamp(),
        };

        // Serialize to bytes
        let value = serde_json::to_vec(&registry)?;
        let key = PublicMonitorRegistry::dht_key(&domain);

        Ok((key, value))
    }

    /// Lookup a public monitor group from DHT
    ///
    /// First checks local cache, then returns the DHT key for lookup.
    /// Caller must call PeerNode::dht_get_record() to perform actual DHT query.
    pub async fn lookup_monitor(
        &self,
        domain: &str,
    ) -> Result<(Option<PublicMonitorGroup>, RecordKey), Box<dyn std::error::Error>> {
        // Check local cache first
        if let Some(group) = self.local_cache.read().await.get(domain) {
            return Ok((Some(group.clone()), PublicMonitorRegistry::dht_key(domain)));
        }

        // Return None + key for DHT lookup
        Ok((None, PublicMonitorRegistry::dht_key(domain)))
    }

    /// List all known public monitors
    pub async fn list_all(&self) -> Vec<PublicMonitorGroup> {
        self.local_cache
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    /// Join an existing public monitor group
    pub async fn join_monitor(
        &self,
        domain: &str,
        peer_id: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut cache = self.local_cache.write().await;

        if let Some(group) = cache.get_mut(domain) {
            group.add_peer(peer_id);
            Ok(())
        } else {
            Err("Monitor group not found".into())
        }
    }

    /// Leave a public monitor group
    pub async fn leave_monitor(
        &self,
        domain: &str,
        peer_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut cache = self.local_cache.write().await;

        if let Some(group) = cache.get_mut(domain) {
            group.remove_peer(peer_id);
            Ok(())
        } else {
            Err("Monitor group not found".into())
        }
    }

    /// Process a DHT get_record result
    ///
    /// Call this when receiving a Kademlia GetRecord success event
    pub async fn process_dht_record(
        &self,
        record: &Record,
    ) -> Result<PublicMonitorRegistry, Box<dyn std::error::Error>> {
        let registry = PublicMonitorRegistry::from_record(record)?;

        // Update local cache with discovered monitors
        let mut cache = self.local_cache.write().await;
        for (domain, group) in registry.monitors.iter() {
            cache.insert(domain.clone(), group.clone());
        }

        Ok(registry)
    }
}

impl Default for PublicMonitorDHT {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_and_lookup() {
        let dht = PublicMonitorDHT::new();

        let group = PublicMonitorGroup::new(
            "google.com".to_string(),
            "Google Search".to_string(),
            "peer1".to_string(),
        );

        dht.publish_monitor("google.com".to_string(), group.clone())
            .await
            .unwrap();

        let (result, _key) = dht.lookup_monitor("google.com").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().domain, "google.com");
    }

    #[tokio::test]
    async fn test_join_leave() {
        let dht = PublicMonitorDHT::new();

        let group = PublicMonitorGroup::new(
            "github.com".to_string(),
            "GitHub".to_string(),
            "peer1".to_string(),
        );

        dht.publish_monitor("github.com".to_string(), group)
            .await
            .unwrap();

        dht.join_monitor("github.com", "peer2".to_string())
            .await
            .unwrap();

        let (group, _) = dht.lookup_monitor("github.com").await.unwrap();
        assert_eq!(group.unwrap().participating_peers.len(), 2);

        dht.leave_monitor("github.com", "peer2").await.unwrap();

        let (group, _) = dht.lookup_monitor("github.com").await.unwrap();
        assert_eq!(group.unwrap().participating_peers.len(), 1);
    }
}
