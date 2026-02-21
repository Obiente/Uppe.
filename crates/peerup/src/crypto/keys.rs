//! Ed25519 key generation and management with X25519 derivation.
//!
//! Provides a unified `KeyPair` type that supports both Ed25519 signing
//! and X25519 key exchange (for encryption).

use anyhow::{Context, Result};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};

/// KeyPair for Ed25519 signing and X25519 encryption.
///
/// The Ed25519 secret key is converted to an X25519 secret key via
/// standard clamping, and the X25519 public key is derived from that
/// via scalar multiplication with the Curve25519 base point.
#[derive(Clone)]
pub struct KeyPair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    /// Cached X25519 public key (derived from Ed25519 secret key)
    x25519_public: [u8; 32],
    /// Cached clamped X25519 secret bytes
    x25519_secret: [u8; 32],
}

impl KeyPair {
    /// Create a new keypair from a signing key.
    ///
    /// Derives the X25519 keypair from the Ed25519 secret key using
    /// standard Curve25519 clamping and base point multiplication.
    pub fn new(signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();

        // Convert Ed25519 secret key to X25519 secret key via clamping.
        // This is the standard Ed25519 → X25519 conversion:
        //   - Clear the lowest 3 bits (ensures multiple of 8)
        //   - Clear bit 255 (ensure < 2^255)
        //   - Set bit 254  (ensure high bit set for constant-time ops)
        let mut x25519_secret_bytes = signing_key.to_bytes();
        x25519_secret_bytes[0] &= 248;
        x25519_secret_bytes[31] &= 127;
        x25519_secret_bytes[31] |= 64;

        // Derive X25519 public key via scalar multiplication with base point.
        // StaticSecret::from() accepts [u8; 32] and performs clamping internally,
        // but we pre-clamp to cache the secret bytes in their clamped form.
        let static_secret = X25519StaticSecret::from(x25519_secret_bytes);
        let x25519_public = X25519PublicKey::from(&static_secret);

        Self {
            signing_key,
            verifying_key,
            x25519_public: x25519_public.to_bytes(),
            x25519_secret: x25519_secret_bytes,
        }
    }

    /// Get the Ed25519 public key as bytes.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the Ed25519 public key as hex string.
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key_bytes())
    }

    /// Get the X25519 public key for encryption (32 bytes).
    pub fn x25519_public_key(&self) -> [u8; 32] {
        self.x25519_public
    }

    /// Get X25519 secret key bytes for decryption (clamped, 32 bytes).
    ///
    /// # Security
    /// This exposes the secret key material. Only use for ECDH operations.
    pub fn x25519_secret_bytes(&self) -> [u8; 32] {
        self.x25519_secret
    }
}

/// Generate a new random Ed25519 keypair.
pub fn generate_keypair() -> KeyPair {
    let mut csprng = OsRng;
    let mut secret_bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut csprng, &mut secret_bytes);
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    KeyPair::new(signing_key)
}

/// Save a keypair's secret key to a file (32 bytes).
pub fn save_keypair(keypair: &KeyPair, path: &Path) -> Result<()> {
    let secret_bytes = keypair.signing_key.to_bytes();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, secret_bytes).context("Failed to write keypair to file")?;

    tracing::info!("Saved keypair to: {}", path.display());
    Ok(())
}

/// Load a keypair from a 32-byte secret key file.
pub fn load_keypair(path: &Path) -> Result<KeyPair> {
    let secret_bytes = fs::read(path).context("Failed to read keypair file")?;

    if secret_bytes.len() != 32 {
        anyhow::bail!(
            "Invalid keypair file: expected 32 bytes, got {}",
            secret_bytes.len()
        );
    }

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&secret_bytes);

    let signing_key = SigningKey::from_bytes(&bytes);
    Ok(KeyPair::new(signing_key))
}

/// Load an existing keypair from `path`, or generate and save a new one.
pub fn load_or_generate_keypair(path: &Path) -> Result<KeyPair> {
    if path.exists() {
        tracing::info!("Loading existing keypair from: {}", path.display());
        load_keypair(path)
    } else {
        tracing::info!("Generating new keypair and saving to: {}", path.display());
        let keypair = generate_keypair();
        save_keypair(&keypair, path)?;
        Ok(keypair)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x25519_dalek::{PublicKey as X25519Pub, StaticSecret};

    #[test]
    fn test_generate_keypair() {
        let keypair = generate_keypair();
        assert_eq!(keypair.public_key_bytes().len(), 32);
        assert_eq!(keypair.x25519_public_key().len(), 32);
    }

    #[test]
    fn test_x25519_derivation_is_correct() {
        let keypair = generate_keypair();

        // Independently derive X25519 public key from the same secret
        let secret = StaticSecret::from(keypair.x25519_secret_bytes());
        let expected_public = X25519Pub::from(&secret);

        assert_eq!(keypair.x25519_public_key(), expected_public.to_bytes());
    }

    #[test]
    fn test_x25519_differs_from_ed25519() {
        let keypair = generate_keypair();
        // X25519 and Ed25519 public keys must differ (different curve representations)
        assert_ne!(keypair.public_key_bytes(), keypair.x25519_public_key());
    }

    #[test]
    fn test_save_and_load_keypair() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_keypair.key");

        let original = generate_keypair();
        save_keypair(&original, &path).unwrap();

        let loaded = load_keypair(&path).unwrap();
        assert_eq!(original.public_key_bytes(), loaded.public_key_bytes());
        assert_eq!(original.x25519_public_key(), loaded.x25519_public_key());
    }

    #[test]
    fn test_load_or_generate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_keypair.key");

        let first = load_or_generate_keypair(&path).unwrap();
        let second = load_or_generate_keypair(&path).unwrap();

        assert_eq!(first.public_key_bytes(), second.public_key_bytes());
    }
}
