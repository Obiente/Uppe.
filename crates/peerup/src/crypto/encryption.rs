//! Private result encryption using X25519 key exchange and XChaCha20-Poly1305 AEAD.
//!
//! Provides end-to-end encryption for private data. Only the owner
//! (holder of the X25519 private key) can decrypt.
//!
//! # Security Model
//! - Ephemeral-Static Diffie-Hellman: forward secrecy per message
//! - XChaCha20-Poly1305 AEAD: authenticated encryption
//! - Each encrypted result carries its own ephemeral public key and nonce

use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

/// Encrypted result envelope for private monitors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedResult {
    /// Owner's peer ID (for routing)
    pub owner_peer_id: String,
    /// Monitor UUID this result belongs to
    pub monitor_uuid: String,
    /// Ciphertext (includes AEAD authentication tag)
    pub ciphertext: Vec<u8>,
    /// Nonce for XChaCha20-Poly1305 (24 bytes)
    pub nonce: [u8; 24],
    /// Ephemeral public key for ECDH key exchange (32 bytes)
    pub ephemeral_pubkey: [u8; 32],
    /// Timestamp when encrypted
    pub encrypted_at: i64,
    /// Peer ID of the helper peer who performed the check
    pub helper_peer_id: String,
}

/// Encrypt a JSON-serializable value for the owner.
///
/// Uses ephemeral-static ECDH with XChaCha20-Poly1305 AEAD.
/// Each call generates a fresh ephemeral keypair for forward secrecy.
pub fn encrypt_result_for_owner<T: Serialize>(
    result: &T,
    owner_pubkey: &[u8; 32],
    helper_peer_id: String,
    owner_peer_id: String,
    monitor_uuid: String,
) -> Result<EncryptedResult> {
    // 1. Generate ephemeral keypair (unique per encryption)
    let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_public = PublicKey::from(&ephemeral_secret);

    // 2. ECDH: ephemeral_secret * owner_public → shared secret
    let owner_public = PublicKey::from(*owner_pubkey);
    let shared_secret = ephemeral_secret.diffie_hellman(&owner_public);

    // 3. Derive symmetric key from shared secret
    let key = XChaCha20Poly1305::new(shared_secret.as_bytes().into());

    // 4. Random nonce (24 bytes for XChaCha20)
    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from(nonce_bytes);

    // 5. Serialize and encrypt
    let plaintext = serde_json::to_vec(result)?;
    let ciphertext = key
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    Ok(EncryptedResult {
        owner_peer_id,
        monitor_uuid,
        ciphertext,
        nonce: nonce_bytes,
        ephemeral_pubkey: ephemeral_public.to_bytes(),
        encrypted_at: chrono::Utc::now().timestamp(),
        helper_peer_id,
    })
}

/// Decrypt a result using the owner's X25519 secret key.
///
/// Performs ECDH with the ephemeral public key stored in the encrypted
/// result to recover the shared secret, then decrypts with XChaCha20-Poly1305.
pub fn decrypt_result_for_owner<T: for<'de> Deserialize<'de>>(
    encrypted: &EncryptedResult,
    owner_secret: &[u8; 32],
) -> Result<T> {
    // 1. Reconstruct owner's static secret from stored bytes
    let owner_static = StaticSecret::from(*owner_secret);

    // 2. Load the ephemeral public key that was used during encryption
    let ephemeral_public = PublicKey::from(encrypted.ephemeral_pubkey);

    // 3. ECDH: owner_secret * ephemeral_public → same shared secret
    let shared_secret = owner_static.diffie_hellman(&ephemeral_public);

    // 4. Derive symmetric key from shared secret
    let key = XChaCha20Poly1305::new(shared_secret.as_bytes().into());

    // 5. Decrypt with authentication
    let nonce = XNonce::from(encrypted.nonce);
    let plaintext = key
        .decrypt(&nonce, encrypted.ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed (wrong key or tampered data): {}", e))?;

    // 6. Deserialize
    let result: T = serde_json::from_slice(&plaintext)?;
    Ok(result)
}

/// Batch of encrypted results for efficient DHT storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedResultBatch {
    pub owner_peer_id: String,
    pub monitor_uuid: String,
    pub results: Vec<EncryptedResult>,
    pub stored_at: i64,
    /// Expiration timestamp (default: 7 days from creation)
    pub expires_at: i64,
    pub count: usize,
}

impl EncryptedResultBatch {
    pub fn new(owner_peer_id: String, monitor_uuid: String, results: Vec<EncryptedResult>) -> Self {
        let now = chrono::Utc::now().timestamp();
        let count = results.len();
        Self {
            owner_peer_id,
            monitor_uuid,
            results,
            stored_at: now,
            expires_at: now + (7 * 24 * 3600),
            count,
        }
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expires_at
    }

    pub fn add_result(&mut self, result: EncryptedResult) {
        self.results.push(result);
        self.count = self.results.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        message: String,
        value: u32,
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Generate owner keypair using StaticSecret
        let owner_secret = StaticSecret::random_from_rng(OsRng);
        let owner_public = PublicKey::from(&owner_secret);

        let data = TestData {
            message: "test encrypted data".to_string(),
            value: 42,
        };

        let encrypted = encrypt_result_for_owner(
            &data,
            &owner_public.to_bytes(),
            "helper-123".to_string(),
            "owner-456".to_string(),
            "monitor-789".to_string(),
        )
        .unwrap();

        assert_eq!(encrypted.monitor_uuid, "monitor-789");
        assert_eq!(encrypted.helper_peer_id, "helper-123");
        assert_eq!(encrypted.owner_peer_id, "owner-456");

        let decrypted: TestData =
            decrypt_result_for_owner(&encrypted, &owner_secret.to_bytes()).unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let owner_secret = StaticSecret::random_from_rng(OsRng);
        let owner_public = PublicKey::from(&owner_secret);
        let wrong_secret = StaticSecret::random_from_rng(OsRng);

        let data = TestData {
            message: "secret".to_string(),
            value: 99,
        };

        let encrypted = encrypt_result_for_owner(
            &data,
            &owner_public.to_bytes(),
            "helper".to_string(),
            "owner".to_string(),
            "mon".to_string(),
        )
        .unwrap();

        let result: Result<TestData> =
            decrypt_result_for_owner(&encrypted, &wrong_secret.to_bytes());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Decryption failed"));
    }

    #[test]
    fn test_keypair_integration() {
        // Test with the KeyPair type (Ed25519 → X25519 derivation)
        let keypair = super::super::keys::generate_keypair();

        let data = TestData {
            message: "keypair integration test".to_string(),
            value: 7,
        };

        let encrypted = encrypt_result_for_owner(
            &data,
            &keypair.x25519_public_key(),
            "helper".to_string(),
            "owner".to_string(),
            "mon".to_string(),
        )
        .unwrap();

        let decrypted: TestData =
            decrypt_result_for_owner(&encrypted, &keypair.x25519_secret_bytes()).unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_forward_secrecy() {
        // Two encryptions of the same data produce different ciphertexts
        let owner_secret = StaticSecret::random_from_rng(OsRng);
        let owner_public = PublicKey::from(&owner_secret);

        let data = TestData {
            message: "same data".to_string(),
            value: 1,
        };

        let enc1 = encrypt_result_for_owner(
            &data,
            &owner_public.to_bytes(),
            "h".to_string(),
            "o".to_string(),
            "m".to_string(),
        )
        .unwrap();

        let enc2 = encrypt_result_for_owner(
            &data,
            &owner_public.to_bytes(),
            "h".to_string(),
            "o".to_string(),
            "m".to_string(),
        )
        .unwrap();

        // Different ephemeral keys → different ciphertexts
        assert_ne!(enc1.ephemeral_pubkey, enc2.ephemeral_pubkey);
        assert_ne!(enc1.ciphertext, enc2.ciphertext);

        // Both decrypt to the same data
        let dec1: TestData =
            decrypt_result_for_owner(&enc1, &owner_secret.to_bytes()).unwrap();
        let dec2: TestData =
            decrypt_result_for_owner(&enc2, &owner_secret.to_bytes()).unwrap();
        assert_eq!(dec1, dec2);
    }

    #[test]
    fn test_batch_operations() {
        let owner_secret = StaticSecret::random_from_rng(OsRng);
        let owner_public = PublicKey::from(&owner_secret);

        let mut encrypted_results = Vec::new();
        for i in 0..5u32 {
            let data = TestData {
                message: format!("batch item {}", i),
                value: i,
            };
            let encrypted = encrypt_result_for_owner(
                &data,
                &owner_public.to_bytes(),
                format!("helper-{}", i),
                "owner".to_string(),
                "monitor".to_string(),
            )
            .unwrap();
            encrypted_results.push(encrypted);
        }

        let batch = EncryptedResultBatch::new(
            "owner".to_string(),
            "monitor".to_string(),
            encrypted_results,
        );

        assert_eq!(batch.count, 5);
        assert!(!batch.is_expired());

        for (i, encrypted) in batch.results.iter().enumerate() {
            let decrypted: TestData =
                decrypt_result_for_owner(encrypted, &owner_secret.to_bytes()).unwrap();
            assert_eq!(decrypted.value, i as u32);
        }
    }
}
