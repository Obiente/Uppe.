//! Public Monitor Admission System
//!
//! Two ways monitors become public:
//! 1. **Threshold-based**: When N peers independently add same monitor, it auto-promotes
//! 2. **Admin-signed**: Admins can add/modify/delete public monitors directly
//!
//! No voting, no real-time consensus needed - fully asynchronous.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::database::models::Monitor;
use super::admin_trust::{AdminTrustManager};

/// Interest threshold - how many independent peers must add a monitor before it becomes public
pub const PUBLIC_PROMOTION_THRESHOLD: u32 = 5;

/// Admin-configurable threshold (can be overridden in config)
pub struct AdmissionConfig {
    pub threshold: u32,
}

impl Default for AdmissionConfig {
    fn default() -> Self {
        Self {
            threshold: PUBLIC_PROMOTION_THRESHOLD,
        }
    }
}

/// Public monitor record stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicMonitorRecord {
    /// Monitor configuration
    pub monitor: MonitorConfig,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Last modified timestamp
    pub modified_at: u64,
    
    /// Number of unique peers who have expressed interest
    pub interest_count: u32,
    
    /// Peer IDs who have signaled interest (for deduplication)
    pub interested_peers: Vec<String>,
    
    /// Optional admin signature (if admin-created)
    pub admin_signature: Option<AdminSignature>,
}

/// Monitor configuration (the "canonical" definition)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MonitorConfig {
    /// Public domain (primary key)
    pub domain: String,
    
    /// Target URL
    pub target: String,
    
    /// Display name for UI
    pub display_name: String,
    
    /// Check type
    pub check_type: String,
    
    /// Interval in seconds
    pub interval_seconds: u32,
    
    /// Timeout in seconds
    pub timeout_seconds: u32,
}

impl MonitorConfig {
    /// Create canonical key for DHT lookup
    pub fn dht_key(&self) -> String {
        // Use domain as primary key (case-insensitive)
        format!("public-monitor:{}", self.domain.to_lowercase())
    }
    
    /// Create from Monitor struct
    pub fn from_monitor(m: &Monitor) -> Option<Self> {
        if let Some(domain) = &m.public_domain {
            Some(Self {
                domain: domain.clone(),
                target: m.target.clone(),
                display_name: m.public_display_name.as_ref().cloned().unwrap_or_else(|| domain.clone()),
                check_type: m.check_type.clone(),
                interval_seconds: m.interval_seconds as u32,
                timeout_seconds: m.timeout_seconds as u32,
            })
        } else {
            None
        }
    }
}

/// Admin signature for privileged operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSignature {
    /// Admin key ID who signed this (from trust chain)
    pub admin_key_id: String,
    
    /// Ed25519 signature over serialized MonitorConfig
    pub signature: Vec<u8>,
    
    /// Timestamp when signed
    pub signed_at: u64,
}

/// Admission orchestrator - manages public monitor lifecycle
pub struct AdmissionOrchestrator {
    config: AdmissionConfig,
    trust_manager: AdminTrustManager,
    
    /// Cache of public monitors we've seen
    public_monitors: HashMap<String, PublicMonitorRecord>,
}

impl AdmissionOrchestrator {
    pub fn new(config: AdmissionConfig, trust_manager: AdminTrustManager) -> Result<Self> {
        Ok(Self {
            config,
            trust_manager,
            public_monitors: HashMap::new(),
        })
    }
    
    /// Create a new orchestrator (convenience method for testing/single-threaded use)
    pub async fn with_trust() -> Result<Self> {
        let config = AdmissionConfig::default();
        let trust_manager = AdminTrustManager::new().await?;
        Self::new(config, trust_manager)
    }
    
    /// Check if a key ID is an admin (via trust chain)
    pub fn is_admin_key(&self, key_id: &str) -> bool {
        self.trust_manager.is_admin_key(key_id)
    }
    
    /// Signal interest in a monitor (regular user action)
    pub fn signal_interest(&mut self, monitor: &Monitor, peer_id: &str) -> Result<PublicMonitorRecord> {
        let config = MonitorConfig::from_monitor(monitor)
            .ok_or_else(|| anyhow::anyhow!("Monitor missing public_domain"))?;
        
        let key = config.dht_key();
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // Get or create record
        let record = self.public_monitors.entry(key.clone()).or_insert_with(|| {
            PublicMonitorRecord {
                monitor: config.clone(),
                created_at: now,
                modified_at: now,
                interest_count: 0,
                interested_peers: Vec::new(),
                admin_signature: None,
            }
        });
        
        // Add this peer's interest if not already counted
        if !record.interested_peers.contains(&peer_id.to_string()) {
            record.interested_peers.push(peer_id.to_string());
            record.interest_count += 1;
            record.modified_at = now;
        }
        
        Ok(record.clone())
    }
    
    /// Admin creates/modifies public monitor (privileged operation)
    /// Requires admin_key_id and signature from external signing process
    pub fn admin_create_or_modify(&mut self, monitor: &Monitor, admin_key_id: &str, signature: Vec<u8>) -> Result<PublicMonitorRecord> {
        // Verify this is a valid admin key
        if !self.is_admin_key(admin_key_id) {
            anyhow::bail!("Invalid admin key ID: {}", admin_key_id);
        }
        
        let config = MonitorConfig::from_monitor(monitor)
            .ok_or_else(|| anyhow::anyhow!("Monitor missing public_domain"))?;
        
        // Verify signature
        let message = serde_json::to_vec(&config)?;
        if !self.trust_manager.verify_admin_signature(admin_key_id, &message, &signature)? {
            anyhow::bail!("Invalid admin signature");
        }
                let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let key = config.dht_key();
        
        let admin_sig = AdminSignature {
            admin_key_id: admin_key_id.to_string(),
            signature,
            signed_at: now,
        };
        
        let record = PublicMonitorRecord {
            monitor: config,
            created_at: now,
            modified_at: now,
            interest_count: 0, // Admin monitors don't need interest count
            interested_peers: Vec::new(),
            admin_signature: Some(admin_sig),
        };
        
        self.public_monitors.insert(key, record.clone());
        Ok(record)
    }
    
    /// Admin deletes public monitor
    pub fn admin_delete(&mut self, domain: &str, admin_key_id: &str) -> Result<()> {
        if !self.is_admin_key(admin_key_id) {
            anyhow::bail!("Only admins can delete public monitors");
        }
        
        let key = format!("public-monitor:{}", domain.to_lowercase());
        self.public_monitors.remove(&key);
        Ok(())
    }
    
    /// Check if a monitor should be promoted to public (threshold reached)
    pub fn should_promote(&self, record: &PublicMonitorRecord) -> bool {
        // Admin-signed monitors are always public
        if record.admin_signature.is_some() {
            return true;
        }
        
        // Check if threshold reached
        record.interest_count >= self.config.threshold
    }
    
    /// Verify admin signature on a monitor record
    pub fn verify_admin_signature(&self, record: &PublicMonitorRecord) -> Result<bool> {
        if let Some(sig) = &record.admin_signature {
            // Check signature age (must be < 7 days old)
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let age = now.saturating_sub(sig.signed_at);
            if age > 7 * 24 * 60 * 60 {
                return Ok(false); // Signature too old
            }
            
            // Verify via trust manager
            let message = serde_json::to_vec(&record.monitor)?;
            self.trust_manager.verify_admin_signature(&sig.admin_key_id, &message, &sig.signature)
        } else {
            Ok(false) // No signature present
        }
    }
    
    /// Process a monitor record received from DHT
    pub fn process_dht_record(&mut self, record: PublicMonitorRecord) -> Result<bool> {
        let key = record.monitor.dht_key();
        
        // If admin-signed, verify signature
        if record.admin_signature.is_some() {
            if !self.verify_admin_signature(&record)? {
                anyhow::bail!("Invalid admin signature");
            }
        }
        
        // Check if we should accept this record
        let should_accept = if let Some(existing) = self.public_monitors.get(&key) {
            // If new record is admin-signed and existing is not, prefer new
            if record.admin_signature.is_some() && existing.admin_signature.is_none() {
                true
            }
            // If both admin-signed, prefer newer
            else if record.admin_signature.is_some() && existing.admin_signature.is_some() {
                record.modified_at > existing.modified_at
            }
            // If neither admin-signed, merge interest counts
            else {
                // Merge interested peers
                true
            }
        } else {
            // New record, accept it
            true
        };
        
        if should_accept {
            self.public_monitors.insert(key, record);
        }
        
        Ok(should_accept)
    }
    
    /// Get all public monitors (threshold met or admin-signed)
    pub fn get_public_monitors(&self) -> Vec<PublicMonitorRecord> {
        self.public_monitors
            .values()
            .filter(|r| self.should_promote(r))
            .cloned()
            .collect()
    }
    
    /// Update cache from DHT query results
    pub fn update_from_dht(&mut self, records: Vec<PublicMonitorRecord>) -> Result<()> {
        for record in records {
            let _ = self.process_dht_record(record);
        }
        Ok(())
    }
    
    /// Update admin trust chain from DHT
    pub async fn update_trust_chain_from_dht(&mut self, dht_data: Vec<u8>) -> Result<bool> {
        use super::admin_trust::AdminTrustChain;
        let chain: AdminTrustChain = serde_json::from_slice(&dht_data)?;
        self.trust_manager.update_from_dht(chain).await
    }
    
    /// Get current admin key IDs
    pub fn get_admin_key_ids(&self) -> Vec<String> {
        self.trust_manager.get_admin_key_ids()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_threshold_promotion() {
        let config = AdmissionConfig {
            threshold: 3,
        };
        let trust_manager = AdminTrustManager::new_for_tests();
        let mut orchestrator = AdmissionOrchestrator::new(config, trust_manager).unwrap();
        
        let monitor = Monitor::new_public(
            "Test".into(),
            "https://example.com".into(),
            "example.com".into(),
            "Example Site".into(),
            "https".into(),
        );
        
        // First interest signal
        let record = orchestrator.signal_interest(&monitor, "peer1").unwrap();
        assert_eq!(record.interest_count, 1);
        assert!(!orchestrator.should_promote(&record));
        
        // Second and third peers
        orchestrator.signal_interest(&monitor, "peer2").unwrap();
        let record = orchestrator.signal_interest(&monitor, "peer3").unwrap();
        assert_eq!(record.interest_count, 3);
        assert!(orchestrator.should_promote(&record));
    }
}

