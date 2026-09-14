#![allow(dead_code)] // Future infrastructure — admin key rotation not yet wired

//! Admin Trust Chain System
//!
//! Secure, auto-updating admin authentication using HTTPS bootstrap + DHT caching.
//!
//! ## Architecture:
//! 1. **HTTPS Bootstrap**: Fetch initial admin keys from keys.uppe.dev (Git-tracked)
//! 2. **DHT Caching**: Cache keys in DHT for offline/decentralized operation
//! 3. **Key Rotation**: Admins sign new keys with old keys, creating verifiable chain
//! 4. **Automatic Verification**: Nodes verify chain and update hourly
//! 5. **Revocation**: Compromised keys added to CRL (Certificate Revocation List)
//!
//! ## Security Model:
//! - Root keys served over HTTPS from keys.uppe.dev (DNS + TLS security)
//! - Git commit history provides audit trail
//! - Optional fingerprint pinning for paranoid users
//! - Key rotations signed by previous key (chain of trust)
//! - Each key has expiry timestamp (max 1 year)
//! - Revoked keys published to DHT and HTTPS

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Bootstrap URLs for fetching admin trust chain (tried in order)
pub const ADMIN_KEY_BOOTSTRAP_URLS: &[&str] = &[
    "https://keys.uppe.dev/admin-trust-chain.json", // Primary CDN
    "https://uppe.github.io/keys/admin-trust-chain.json", // GitHub Pages
    "https://raw.githubusercontent.com/Obiente/Uppe/main/admin-keys.json", // Raw GitHub
];

/// DHT key for admin trust chain (cached by all nodes)
pub const ADMIN_TRUST_CHAIN_DHT_KEY: &str = "uppe-admin-trust-chain";

/// DHT key for revocation list
pub const ADMIN_REVOCATION_LIST_DHT_KEY: &str = "uppe-admin-revocation-list";

/// Maximum key lifetime (1 year)
pub const MAX_KEY_LIFETIME_SECS: u64 = 365 * 24 * 60 * 60;

/// How often to check for updates (1 hour)
pub const UPDATE_CHECK_INTERVAL_SECS: u64 = 3600;

/// Admin key with metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminKey {
    /// Ed25519 public key (base64 encoded)
    pub public_key: String,

    /// When this key becomes valid
    pub valid_from: u64,

    /// When this key expires
    pub valid_until: u64,

    /// Key identifier (first 8 bytes of public key hash)
    pub key_id: String,

    /// Human-readable description
    pub description: String,
}

impl AdminKey {
    /// Check if key is currently valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs();

        now >= self.valid_from && now <= self.valid_until
    }

    /// Check if key is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs();

        now > self.valid_until
    }

    /// Generate key ID from public key
    pub fn compute_key_id(public_key: &str) -> String {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(public_key.as_bytes());
        format!("{:x}", hash).chars().take(16).collect()
    }
}

/// Key rotation message - signed by previous key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotation {
    /// The new key being activated
    pub new_key: AdminKey,

    /// Key ID of the previous key that signed this
    pub signed_by_key_id: String,

    /// Ed25519 signature over new_key (signed by previous key)
    pub signature: Vec<u8>,

    /// Timestamp when rotation was performed
    pub rotated_at: u64,

    /// Reason for rotation
    pub reason: String,
}

impl KeyRotation {
    /// Verify that this rotation was signed by the claimed key
    pub fn verify(&self, previous_key: &AdminKey) -> Result<bool> {
        // Verify key ID matches
        if previous_key.key_id != self.signed_by_key_id {
            return Ok(false);
        }

        verify_admin_signature_bytes(
            &previous_key.public_key,
            &canonical_rotation_bytes(&self.new_key)?,
            &self.signature,
        )
    }
}

/// Certificate Revocation List (CRL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationList {
    /// Key IDs that have been revoked
    pub revoked_keys: HashSet<String>,

    /// Reason for each revocation
    pub revocation_reasons: HashMap<String, String>,

    /// When each key was revoked
    pub revoked_at: HashMap<String, u64>,

    /// Signature over this revocation list (signed by current admin key)
    pub signature: Vec<u8>,

    /// Key ID that signed this CRL
    pub signed_by_key_id: String,

    /// CRL version number (increments on each update)
    pub version: u64,

    /// When this CRL was issued
    pub issued_at: u64,
}

impl RevocationList {
    /// Check if a key is revoked
    pub fn is_revoked(&self, key_id: &str) -> bool {
        self.revoked_keys.contains(key_id)
    }

    /// Verify CRL signature
    pub fn verify(&self, signing_key: &AdminKey) -> Result<bool> {
        if signing_key.key_id != self.signed_by_key_id {
            return Ok(false);
        }

        verify_admin_signature_bytes(
            &signing_key.public_key,
            &canonical_revocation_list_bytes(self)?,
            &self.signature,
        )
    }
}

fn canonical_rotation_bytes(new_key: &AdminKey) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(new_key)?)
}

fn canonical_revocation_list_bytes(revocation_list: &RevocationList) -> Result<Vec<u8>> {
    #[derive(Serialize)]
    struct SignableRevocationList<'a> {
        revoked_keys: BTreeSet<&'a str>,
        revocation_reasons: BTreeMap<&'a str, &'a str>,
        revoked_at: BTreeMap<&'a str, u64>,
        signed_by_key_id: &'a str,
        version: u64,
        issued_at: u64,
    }

    let revoked_keys = revocation_list.revoked_keys.iter().map(String::as_str).collect();
    let revocation_reasons = revocation_list
        .revocation_reasons
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let revoked_at = revocation_list
        .revoked_at
        .iter()
        .map(|(key, value)| (key.as_str(), *value))
        .collect();

    Ok(serde_json::to_vec(&SignableRevocationList {
        revoked_keys,
        revocation_reasons,
        revoked_at,
        signed_by_key_id: &revocation_list.signed_by_key_id,
        version: revocation_list.version,
        issued_at: revocation_list.issued_at,
    })?)
}

fn verify_admin_signature_bytes(
    public_key_b64: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool> {
    use base64::{Engine, engine::general_purpose};
    use ed25519_dalek::{Signature, VerifyingKey};

    let public_key_bytes = general_purpose::STANDARD.decode(public_key_b64)?;
    let key_array: [u8; 32] = public_key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("Invalid public key length"))?;
    let sig_array: [u8; 64] =
        signature.try_into().map_err(|_| anyhow!("Invalid signature length"))?;

    let public_key =
        VerifyingKey::from_bytes(&key_array).map_err(|e| anyhow!("Invalid public key: {}", e))?;
    let sig = Signature::from_bytes(&sig_array);

    Ok(public_key.verify_strict(message, &sig).is_ok())
}

/// Admin trust chain - verifiable path from root keys to current keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminTrustChain {
    /// All key rotations in chronological order
    pub rotations: Vec<KeyRotation>,

    /// Current valid admin keys
    pub current_keys: Vec<AdminKey>,

    /// Certificate Revocation List
    pub revocation_list: RevocationList,

    /// When this chain was last updated
    pub last_updated: u64,

    /// Version number (increments on each update)
    pub version: u64,
}

impl AdminTrustChain {
    /// Bootstrap from HTTPS (tries multiple URLs)
    pub async fn bootstrap_from_https() -> Result<Self> {
        if std::env::var("UPPE_ADMIN_ROOT_KEYS").is_err() {
            anyhow::bail!("No locally configured admin trust roots");
        }
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;

        let mut last_error = None;

        for url in ADMIN_KEY_BOOTSTRAP_URLS {
            match client.get(*url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<AdminTrustChain>().await {
                            Ok(chain) => {
                                log::info!("Successfully bootstrapped admin keys from {}", url);
                                return Ok(chain);
                            }
                            Err(e) => {
                                log::warn!("Failed to parse admin keys from {}: {}", url, e);
                                last_error = Some(anyhow!("Parse error: {}", e));
                            }
                        }
                    } else {
                        log::warn!("HTTP {} from {}", response.status(), url);
                        last_error = Some(anyhow!("HTTP {}", response.status()));
                    }
                }
                Err(e) => {
                    log::warn!("Failed to fetch from {}: {}", url, e);
                    last_error = Some(anyhow!("Request failed: {}", e));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All bootstrap URLs failed")))
    }

    /// Create empty chain (for testing)
    pub fn empty() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs();
        Self {
            rotations: Vec::new(),
            current_keys: Vec::new(),
            revocation_list: RevocationList {
                revoked_keys: HashSet::new(),
                revocation_reasons: HashMap::new(),
                revoked_at: HashMap::new(),
                signature: Vec::new(),
                signed_by_key_id: String::new(),
                version: 0,
                issued_at: now,
            },
            last_updated: now,
            version: 0,
        }
    }

    /// Trust roots are local configuration, never keys supplied by a DHT record.
    pub fn verify(&self) -> Result<bool> {
        let Ok(value) = std::env::var("UPPE_ADMIN_ROOT_KEYS") else {
            return Ok(false);
        };
        let roots: Vec<AdminKey> = serde_json::from_str(&value)?;
        self.verify_with_roots(&roots)
    }

    pub fn verify_with_roots(&self, roots: &[AdminKey]) -> Result<bool> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if roots.is_empty()
            || self.current_keys.is_empty()
            || self.rotations.len() > 100
            || self.current_keys.len() > 100
            || self.revocation_list.signature.is_empty()
            || self.revocation_list.issued_at > now + 30
        {
            return Ok(false);
        }
        let valid_metadata = |key: &AdminKey| {
            key.key_id == AdminKey::compute_key_id(&key.public_key)
                && key.valid_until > key.valid_from
                && key.valid_until - key.valid_from <= MAX_KEY_LIFETIME_SECS
        };
        let mut trusted = roots.to_vec();
        if trusted.iter().any(|key| !valid_metadata(key)) {
            return Ok(false);
        }
        for rotation in &self.rotations {
            let Some(signer) = trusted.iter().find(|key| key.key_id == rotation.signed_by_key_id)
            else {
                return Ok(false);
            };
            if !valid_metadata(&rotation.new_key)
                || !rotation.verify(signer)?
                || rotation.new_key.valid_from < signer.valid_from
                || rotation.new_key.valid_from > signer.valid_until
                || self.revocation_list.is_revoked(&signer.key_id)
            {
                return Ok(false);
            }
            if !trusted.iter().any(|key| key.key_id == rotation.new_key.key_id) {
                trusted.push(rotation.new_key.clone());
            }
        }
        if self.current_keys.iter().any(|key| !trusted.contains(key) || !key.is_valid()) {
            return Ok(false);
        }
        let Some(crl_signer) =
            trusted.iter().find(|key| key.key_id == self.revocation_list.signed_by_key_id)
        else {
            return Ok(false);
        };
        if !crl_signer.is_valid() || !self.revocation_list.verify(crl_signer)? {
            return Ok(false);
        }
        Ok(true)
    }

    /// Get all currently valid admin keys (not expired, not revoked)
    pub fn get_valid_keys(&self) -> Vec<AdminKey> {
        self.current_keys
            .iter()
            .filter(|k| k.is_valid() && !self.revocation_list.is_revoked(&k.key_id))
            .cloned()
            .collect()
    }

    /// Check if a peer ID is an admin
    pub fn is_admin(&self, peer_id: &str) -> bool {
        if !self.verify().unwrap_or(false) {
            return false;
        }
        // In practice, would map peer_id to public key and check against valid keys
        // For now, simplified check
        self.get_valid_keys().iter().any(|k| {
            // Extract peer ID from key or maintain separate mapping
            // This is a placeholder - real implementation would derive peer_id from public key
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(&k.public_key)
                .ok()
                .is_some_and(|key| hex::encode(key) == peer_id)
        })
    }

    /// Apply a new rotation (admin operation)
    pub fn apply_rotation(&mut self, rotation: KeyRotation) -> Result<()> {
        // Verify rotation is valid
        let signing_key = self
            .current_keys
            .iter()
            .find(|k| k.key_id == rotation.signed_by_key_id)
            .ok_or_else(|| anyhow!("Unknown signing key"))?;

        if !rotation.verify(signing_key)? {
            anyhow::bail!("Invalid rotation signature");
        }

        self.rotations.push(rotation.clone());
        self.current_keys.push(rotation.new_key);
        self.last_updated = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        self.version += 1;

        Ok(())
    }

    /// Update revocation list (admin operation)
    pub fn update_revocation_list(&mut self, revocation_list: RevocationList) -> Result<()> {
        // Verify CRL is signed by a current valid key
        let signing_key = self
            .get_valid_keys()
            .iter()
            .find(|k| k.key_id == revocation_list.signed_by_key_id)
            .ok_or_else(|| anyhow!("CRL signed by non-admin key"))?
            .clone();

        if !revocation_list.verify(&signing_key)? {
            anyhow::bail!("Invalid CRL signature");
        }

        // Must be newer than current CRL
        if revocation_list.version <= self.revocation_list.version {
            anyhow::bail!("CRL version must be newer");
        }

        self.revocation_list = revocation_list;
        self.last_updated = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        self.version += 1;

        Ok(())
    }
}

/// Bootstrap status for UI feedback
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapStatus {
    NotStarted,
    FetchingFromDHT,
    FetchingFromHTTPS { url: String },
    Success { source: String, version: u64 },
    Failed { error: String },
}

/// Admin trust manager - handles automatic updates from DHT and HTTPS
pub struct AdminTrustManager {
    /// Current trust chain
    chain: AdminTrustChain,

    /// Last time we queried for updates
    last_update_check: SystemTime,

    /// How often to check for updates
    check_interval: Duration,

    /// Bootstrap status (for UI)
    pub bootstrap_status: BootstrapStatus,
}

impl AdminTrustManager {
    /// Create new trust manager (async bootstrap)
    pub async fn new() -> Result<Self> {
        let mut manager = Self {
            chain: AdminTrustChain::empty(),
            last_update_check: SystemTime::now(),
            check_interval: Duration::from_secs(UPDATE_CHECK_INTERVAL_SECS),
            bootstrap_status: BootstrapStatus::NotStarted,
        };

        manager.bootstrap().await?;
        Ok(manager)
    }

    /// Test-only constructor that skips network bootstrap
    #[cfg(test)]
    pub fn new_for_tests() -> Self {
        Self {
            chain: AdminTrustChain::empty(),
            last_update_check: SystemTime::now(),
            check_interval: Duration::from_secs(UPDATE_CHECK_INTERVAL_SECS),
            bootstrap_status: BootstrapStatus::Success { source: "test".to_string(), version: 0 },
        }
    }

    /// Bootstrap from DHT or HTTPS
    async fn bootstrap(&mut self) -> Result<()> {
        // Try DHT first (fast, decentralized)
        self.bootstrap_status = BootstrapStatus::FetchingFromDHT;

        // TODO: Implement DHT fetch when node is available
        // if let Ok(chain) = self.fetch_from_dht().await {
        //     self.chain = chain;
        //     self.bootstrap_status = BootstrapStatus::Success {
        //         source: "DHT".to_string(),
        //         version: chain.version
        //     };
        //     return Ok(());
        // }

        // Fallback to HTTPS bootstrap
        for url in ADMIN_KEY_BOOTSTRAP_URLS {
            self.bootstrap_status = BootstrapStatus::FetchingFromHTTPS { url: url.to_string() };

            match AdminTrustChain::bootstrap_from_https().await {
                Ok(chain) => {
                    if chain.verify()? {
                        self.chain = chain.clone();
                        self.bootstrap_status = BootstrapStatus::Success {
                            source: url.to_string(),
                            version: chain.version,
                        };
                        log::info!(
                            "Admin trust chain bootstrapped from HTTPS (version {})",
                            chain.version
                        );

                        // TODO: Cache in DHT for other peers
                        // self.publish_to_dht(&chain).await?;

                        return Ok(());
                    } else {
                        log::warn!("Invalid trust chain from {}", url);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to bootstrap from {}: {}", url, e);
                }
            }
        }

        self.bootstrap_status =
            BootstrapStatus::Failed { error: "All bootstrap sources failed".to_string() };

        Err(anyhow!("Failed to bootstrap admin trust chain"))
    }

    /// Create from existing chain (loaded from DHT)
    pub fn from_chain(chain: AdminTrustChain) -> Result<Self> {
        // Verify chain before accepting
        if !chain.verify()? {
            anyhow::bail!("Invalid trust chain");
        }

        let version = chain.version;
        Ok(Self {
            chain,
            last_update_check: SystemTime::now(),
            bootstrap_status: BootstrapStatus::Success { source: "cached".to_string(), version },
            check_interval: Duration::from_secs(3600),
        })
    }

    /// Check if we should query for updates
    pub fn should_update(&self) -> bool {
        SystemTime::now()
            .duration_since(self.last_update_check)
            .unwrap_or(Duration::from_secs(0))
            > self.check_interval
    }

    /// Perform update check (DHT first, then HTTPS)
    pub async fn check_for_updates(&mut self) -> Result<bool> {
        if !self.should_update() {
            return Ok(false);
        }

        self.last_update_check = SystemTime::now();

        // Try DHT first
        // TODO: Implement when DHT node is integrated

        // Try HTTPS fallback
        match AdminTrustChain::bootstrap_from_https().await {
            Ok(new_chain) => {
                if new_chain.version > self.chain.version
                    && new_chain.revocation_list.version >= self.chain.revocation_list.version
                    && new_chain.verify()?
                {
                    log::info!(
                        "Admin keys updated: v{} -> v{}",
                        self.chain.version,
                        new_chain.version
                    );
                    self.chain = new_chain;
                    return Ok(true);
                }
            }
            Err(e) => {
                log::debug!("Update check failed: {}", e);
            }
        }

        Ok(false)
    }

    /// Update from DHT record
    pub async fn update_from_dht(&mut self, dht_chain: AdminTrustChain) -> Result<bool> {
        // Verify DHT chain
        if !dht_chain.verify()? {
            log::warn!("Invalid trust chain from DHT");
            return Ok(false);
        }

        // Only accept if newer version
        if dht_chain.version > self.chain.version
            && dht_chain.revocation_list.version >= self.chain.revocation_list.version
        {
            log::info!(
                "Updated admin keys from DHT: v{} -> v{}",
                self.chain.version,
                dht_chain.version
            );
            self.chain = dht_chain;
            self.last_update_check = SystemTime::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Serialize chain for DHT storage
    pub fn serialize_for_dht(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.chain)?)
    }

    /// Deserialize chain from DHT
    pub fn deserialize_from_dht(data: &[u8]) -> Result<AdminTrustChain> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Check if a key ID is a valid admin
    pub fn is_admin_key(&self, key_id: &str) -> bool {
        if !self.chain.verify().unwrap_or(false) {
            return false;
        }
        self.chain.get_valid_keys().iter().any(|k| k.key_id == key_id)
    }

    /// Verify a signature from an admin
    pub fn verify_admin_signature(
        &self,
        key_id: &str,
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool> {
        if !self.chain.verify()? || self.chain.revocation_list.is_revoked(key_id) {
            return Ok(false);
        }
        // Find the key in the current_keys list
        let admin_key = self
            .chain
            .current_keys
            .iter()
            .find(|k| k.key_id == key_id && k.is_valid())
            .ok_or_else(|| anyhow!("Key not found or not valid: {}", key_id))?;

        use base64::{Engine, engine::general_purpose};
        let public_key_bytes = general_purpose::STANDARD.decode(&admin_key.public_key)?;
        use ed25519_dalek::{Signature, VerifyingKey};

        // Convert slice to array for VerifyingKey
        let key_array: [u8; 32] = public_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Invalid public key length"))?;

        let public_key = VerifyingKey::from_bytes(&key_array)
            .map_err(|e| anyhow!("Invalid public key: {}", e))?;

        // Convert signature slice to array
        let sig_array: [u8; 64] =
            signature.try_into().map_err(|_| anyhow!("Invalid signature length"))?;
        let sig = Signature::from_bytes(&sig_array);

        Ok(public_key.verify_strict(message, &sig).is_ok())
    }

    /// Get current trust chain (for publishing to DHT)
    pub fn get_chain(&self) -> &AdminTrustChain {
        &self.chain
    }

    /// Get all valid admin key IDs
    pub fn get_admin_key_ids(&self) -> Vec<String> {
        if !self.chain.verify().unwrap_or(false) {
            return Vec::new();
        }
        self.chain.get_valid_keys().iter().map(|k| k.key_id.clone()).collect()
    }

    /// Get bootstrap status
    pub fn get_bootstrap_status(&self) -> &BootstrapStatus {
        &self.bootstrap_status
    }

    /// Sign data with an admin key
    /// TODO: Implement real Ed25519 signing once private keys are available
    pub fn sign_data(&self, _data: &[u8]) -> Result<Vec<u8>> {
        // Placeholder: return a 64-byte signature (Ed25519 signature length)
        // In production, this would use the actual private key stored securely
        anyhow::bail!("Admin signing key is not configured")
    }

    /// Get statistics for TUI
    pub fn get_stats(&self) -> TrustChainStats {
        let valid_keys = self.chain.get_valid_keys();
        let expired_keys: Vec<_> =
            self.chain.current_keys.iter().filter(|k| k.is_expired()).collect();

        TrustChainStats {
            version: self.chain.version,
            total_keys: self.chain.current_keys.len(),
            valid_keys: valid_keys.len(),
            expired_keys: expired_keys.len(),
            revoked_keys: self.chain.revocation_list.revoked_keys.len(),
            rotations_count: self.chain.rotations.len(),
            last_updated: self.chain.last_updated,
        }
    }
}

/// Trust chain statistics for TUI display
#[derive(Debug, Clone)]
pub struct TrustChainStats {
    pub version: u64,
    pub total_keys: usize,
    pub valid_keys: usize,
    pub expired_keys: usize,
    pub revoked_keys: usize,
    pub rotations_count: usize,
    pub last_updated: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_chain_requires_pinned_root_and_rejects_tampering() {
        use base64::Engine;
        use ed25519_dalek::{Signer, SigningKey};
        let secret = SigningKey::from_bytes(&[41; 32]);
        let public_key =
            base64::engine::general_purpose::STANDARD.encode(secret.verifying_key().to_bytes());
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let root = AdminKey {
            key_id: AdminKey::compute_key_id(&public_key),
            public_key,
            valid_from: now - 60,
            valid_until: now + 600,
            description: "Synthetic root".into(),
        };
        let mut chain = AdminTrustChain::empty();
        chain.current_keys.push(root.clone());
        chain.revocation_list.signed_by_key_id = root.key_id.clone();
        chain.revocation_list.version = 1;
        chain.revocation_list.signature = secret
            .sign(&canonical_revocation_list_bytes(&chain.revocation_list).unwrap())
            .to_bytes()
            .to_vec();
        assert!(chain.verify_with_roots(std::slice::from_ref(&root)).unwrap());
        assert!(!chain.verify_with_roots(&[]).unwrap());
        chain.current_keys[0].valid_until += 1;
        assert!(!chain.verify_with_roots(std::slice::from_ref(&root)).unwrap());
        chain.current_keys[0] = root.clone();
        chain.revocation_list.revoked_keys.insert(root.key_id.clone());
        assert!(!chain.verify_with_roots(&[root]).unwrap());
    }

    #[test]
    fn test_trust_chain_empty() {
        let chain = AdminTrustChain::empty();
        assert_eq!(chain.current_keys.len(), 0);
        assert_eq!(chain.version, 0);
    }

    #[test]
    fn test_key_expiry() {
        let mut key = AdminKey {
            public_key: "test".to_string(),
            valid_from: 0,
            valid_until: 1000,
            key_id: "test_id".to_string(),
            description: "Test".to_string(),
        };

        assert!(key.is_expired());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs();
        key.valid_from = now - 100;
        key.valid_until = now + 100;

        assert!(key.is_valid());
        assert!(!key.is_expired());
    }

    #[tokio::test]
    async fn test_bootstrap_status() {
        // Test would need mock HTTP server
        // For now, just test empty chain
        let chain = AdminTrustChain::empty();
        assert_eq!(chain.version, 0);
    }
}
