//! DHT-stored metadata for distributed orchestration
//!
//! This module provides generic data structures and helpers for storing peer metadata
//! in the DHT: rate limits, trust scores, reputation, etc.
//!
//! These are generic building blocks that applications can use for their specific needs.

use libp2p::kad::{Record, RecordKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generic rate limit information stored in DHT for a peer
///
/// Applications can use this for any rate limiting needs (API calls, operations, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRateLimits {
    /// Peer ID
    pub peer_id: String,
    
    /// Current time window start timestamp
    pub window_start: i64,
    
    /// Operations performed in current window
    pub operations_this_window: usize,
    
    /// Total resources owned/created by this peer
    pub resource_count: usize,
    
    /// Maximum allowed resources
    pub max_resources: usize,
    
    /// Maximum operations per time window
    pub max_operations_per_window: usize,
    
    /// Time window duration in seconds (default: 3600 = 1 hour)
    pub window_duration_seconds: i64,
    
    /// When this record was last updated
    pub updated_at: i64,
}

impl PeerRateLimits {
    /// Create new rate limits with defaults
    pub fn new(peer_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            peer_id,
            window_start: now,
            operations_this_window: 0,
            resource_count: 0,
            max_resources: 10,
            max_operations_per_window: 100,
            window_duration_seconds: 3600, // 1 hour default
            updated_at: now,
        }
    }
    
    /// Create with custom limits
    pub fn with_limits(
        peer_id: String,
        max_resources: usize,
        max_operations_per_window: usize,
        window_duration_seconds: i64,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            peer_id,
            window_start: now,
            operations_this_window: 0,
            resource_count: 0,
            max_resources,
            max_operations_per_window,
            window_duration_seconds,
            updated_at: now,
        }
    }
    
    /// Check if peer can add another resource
    pub fn can_add_resource(&self) -> bool {
        self.resource_count < self.max_resources
    }
    
    /// Check if peer can perform an operation (rate limiting)
    pub fn can_perform_operation(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        
        // Reset if window expired
        if now - self.window_start > self.window_duration_seconds {
            return true; // New window, reset needed
        }
        
        self.operations_this_window < self.max_operations_per_window
    }
    
    /// Increment operation count
    pub fn increment_operation(&mut self) {
        let now = chrono::Utc::now().timestamp();
        
        // Reset if window expired
        if now - self.window_start > self.window_duration_seconds {
            self.window_start = now;
            self.operations_this_window = 0;
        }
        
        self.operations_this_window += 1;
        self.updated_at = now;
    }
    
    /// Increment resource count
    pub fn increment_resource(&mut self) {
        self.resource_count += 1;
        self.updated_at = chrono::Utc::now().timestamp();
    }
    
    /// Create DHT key for this peer's rate limits
    /// 
    /// `namespace` is application-specific (e.g., "uppe", "messaging", "files")
    pub fn dht_key(namespace: &str, peer_id: &str) -> RecordKey {
        RecordKey::new(&format!("/{}/metadata/rate-limits/{}", namespace, peer_id))
    }
    
    /// Serialize to DHT record
    pub fn to_record(&self, namespace: &str) -> Result<Record, Box<dyn std::error::Error>> {
        let value = serde_json::to_vec(self)?;
        Ok(Record {
            key: Self::dht_key(namespace, &self.peer_id),
            value,
            publisher: None,
            expires: None,
        })
    }
    
    /// Deserialize from DHT record
    pub fn from_record(record: &Record) -> Result<Self, Box<dyn std::error::Error>> {
        let limits: Self = serde_json::from_slice(&record.value)?;
        Ok(limits)
    }
}

/// Generic trust score and reputation for a peer
///
/// Applications can use this to track peer reliability and reputation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerTrustScore {
    /// Peer ID
    pub peer_id: String,
    
    /// Overall trust score (0.0 to 1.0)
    pub score: f64,
    
    /// Number of successful operations/interactions
    pub successful_operations: u64,
    
    /// Number of failed operations/interactions
    pub failed_operations: u64,
    
    /// Uptime/availability percentage
    pub availability_percentage: f64,
    
    /// Geographic location (for diversity)
    pub location: Option<PeerLocation>,
    
    /// Last time this peer was seen
    pub last_seen: i64,
    
    /// When this record was created
    pub created_at: i64,
    
    /// When this record was last updated
    pub updated_at: i64,
}

/// Geographic location of a peer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerLocation {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}

impl PeerTrustScore {
    pub fn new(peer_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            peer_id,
            score: 1.0, // Start with neutral trust
            successful_operations: 0,
            failed_operations: 0,
            availability_percentage: 100.0,
            location: None,
            last_seen: now,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Record an operation result (success or failure)
    pub fn record_operation(&mut self, success: bool) {
        if success {
            self.successful_operations += 1;
        } else {
            self.failed_operations += 1;
        }
        
        // Calculate trust score: success rate weighted by total operations
        let total = self.successful_operations + self.failed_operations;
        if total > 0 {
            let success_rate = self.successful_operations as f64 / total as f64;
            // Weight by number of operations (more operations = more reliable score)
            let weight = (total.min(1000) as f64 / 1000.0).min(1.0);
            self.score = 0.5 + (success_rate - 0.5) * weight;
        }
        
        self.updated_at = chrono::Utc::now().timestamp();
    }
    
    /// Create DHT key for this peer's trust score
    /// 
    /// `namespace` is application-specific (e.g., "uppe", "messaging", "files")
    pub fn dht_key(namespace: &str, peer_id: &str) -> RecordKey {
        RecordKey::new(&format!("/{}/metadata/trust-scores/{}", namespace, peer_id))
    }
    
    /// Serialize to DHT record
    pub fn to_record(&self, namespace: &str) -> Result<Record, Box<dyn std::error::Error>> {
        let value = serde_json::to_vec(self)?;
        Ok(Record {
            key: Self::dht_key(namespace, &self.peer_id),
            value,
            publisher: None,
            expires: None,
        })
    }
    
    /// Deserialize from DHT record
    pub fn from_record(record: &Record) -> Result<Self, Box<dyn std::error::Error>> {
        let score: Self = serde_json::from_slice(&record.value)?;
        Ok(score)
    }
}

/// DHT operations for peer metadata
///
/// Generic helper for storing and retrieving peer metadata from DHT.
/// Applications provide a namespace to avoid collisions.
pub struct PeerMetadataDHT {
    /// Application namespace (e.g., "uppe", "messaging")
    namespace: String,
    
    /// Local cache of rate limits
    rate_limits_cache: tokio::sync::RwLock<HashMap<String, PeerRateLimits>>,
    
    /// Local cache of trust scores
    trust_scores_cache: tokio::sync::RwLock<HashMap<String, PeerTrustScore>>,
}

impl PeerMetadataDHT {
    /// Create new metadata DHT helper with application namespace
    pub fn new(namespace: String) -> Self {
        Self {
            namespace,
            rate_limits_cache: tokio::sync::RwLock::new(HashMap::new()),
            trust_scores_cache: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
    
    /// Get the namespace
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
    
    /// Prepare rate limits for DHT storage
    ///
    /// Returns (key, value) tuple for use with PeerNode::dht_put_record()
    pub async fn prepare_rate_limits(
        &self,
        peer_id: &str,
    ) -> Result<(RecordKey, Vec<u8>), Box<dyn std::error::Error>> {
        let cache = self.rate_limits_cache.read().await;
        let limits = cache
            .get(peer_id)
            .cloned()
            .unwrap_or_else(|| PeerRateLimits::new(peer_id.to_string()));
        drop(cache);
        
        let key = PeerRateLimits::dht_key(&self.namespace, peer_id);
        let value = serde_json::to_vec(&limits)?;
        
        Ok((key, value))
    }
    
    /// Update local cache from DHT record
    pub async fn process_rate_limits_record(
        &self,
        record: &Record,
    ) -> Result<PeerRateLimits, Box<dyn std::error::Error>> {
        let limits = PeerRateLimits::from_record(record)?;
        
        // Update cache
        self.rate_limits_cache
            .write()
            .await
            .insert(limits.peer_id.clone(), limits.clone());
        
        Ok(limits)
    }
    
    /// Get rate limits from cache (or create default)
    pub async fn get_rate_limits(&self, peer_id: &str) -> PeerRateLimits {
        let cache = self.rate_limits_cache.read().await;
        cache
            .get(peer_id)
            .cloned()
            .unwrap_or_else(|| PeerRateLimits::new(peer_id.to_string()))
    }
    
    /// Update rate limits in cache
    pub async fn update_rate_limits(&self, limits: PeerRateLimits) {
        self.rate_limits_cache
            .write()
            .await
            .insert(limits.peer_id.clone(), limits);
    }
    
    /// Prepare trust score for DHT storage
    pub async fn prepare_trust_score(
        &self,
        peer_id: &str,
    ) -> Result<(RecordKey, Vec<u8>), Box<dyn std::error::Error>> {
        let cache = self.trust_scores_cache.read().await;
        let score = cache
            .get(peer_id)
            .cloned()
            .unwrap_or_else(|| PeerTrustScore::new(peer_id.to_string()));
        drop(cache);
        
        let key = PeerTrustScore::dht_key(&self.namespace, peer_id);
        let value = serde_json::to_vec(&score)?;
        
        Ok((key, value))
    }
    
    /// Update local cache from DHT record
    pub async fn process_trust_score_record(
        &self,
        record: &Record,
    ) -> Result<PeerTrustScore, Box<dyn std::error::Error>> {
        let score = PeerTrustScore::from_record(record)?;
        
        // Update cache
        self.trust_scores_cache
            .write()
            .await
            .insert(score.peer_id.clone(), score.clone());
        
        Ok(score)
    }
    
    /// Get trust score from cache (or create default)
    pub async fn get_trust_score(&self, peer_id: &str) -> PeerTrustScore {
        let cache = self.trust_scores_cache.read().await;
        cache
            .get(peer_id)
            .cloned()
            .unwrap_or_else(|| PeerTrustScore::new(peer_id.to_string()))
    }
    
    /// Update trust score in cache
    pub async fn update_trust_score(&self, score: PeerTrustScore) {
        self.trust_scores_cache
            .write()
            .await
            .insert(score.peer_id.clone(), score);
    }
    
    /// Get DHT key for rate limits lookup
    pub fn rate_limits_key(&self, peer_id: &str) -> RecordKey {
        PeerRateLimits::dht_key(&self.namespace, peer_id)
    }
    
    /// Get DHT key for trust score lookup
    pub fn trust_score_key(&self, peer_id: &str) -> RecordKey {
        PeerTrustScore::dht_key(&self.namespace, peer_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limits() {
        let mut limits = PeerRateLimits::new("peer1".to_string());
        assert!(limits.can_add_resource());
        assert!(limits.can_perform_operation());
        
        limits.increment_resource();
        assert_eq!(limits.resource_count, 1);
        
        limits.increment_operation();
        assert_eq!(limits.operations_this_window, 1);
    }
    
    #[test]
    fn test_trust_score() {
        let mut score = PeerTrustScore::new("peer1".to_string());
        assert_eq!(score.score, 1.0);
        
        score.record_operation(true);
        assert_eq!(score.successful_operations, 1);
        
        score.record_operation(false);
        assert_eq!(score.failed_operations, 1);
    }
    
    #[test]
    fn test_metadata_dht_namespace() {
        let dht = PeerMetadataDHT::new("uppe".to_string());
        assert_eq!(dht.namespace(), "uppe");
        
        let key = dht.rate_limits_key("peer1");
        let key_debug = format!("{:?}", key);
        assert!(key_debug.contains("uppe"));
    }
}
