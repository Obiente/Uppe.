use anyhow::{Context, Result};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;

/// KeyPair for signing and verification
#[derive(Clone)]
pub struct KeyPair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Create a new keypair from a signing key
    pub fn new(signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        Self { signing_key, verifying_key }
    }

    /// Get the public key as bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key_bytes())
    }
}

/// Generate a new Ed25519 keypair
pub fn generate_keypair() -> KeyPair {
    let mut csprng = OsRng;
    let mut secret_bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut csprng, &mut secret_bytes);
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    KeyPair::new(signing_key)
}

/// Save a keypair to a file
pub fn save_keypair(keypair: &KeyPair, path: &Path) -> Result<()> {
    let secret_bytes = keypair.signing_key.to_bytes();

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, secret_bytes).context("Failed to write keypair to file")?;

    tracing::info!("Saved keypair to: {}", path.display());
    Ok(())
}

/// Load a keypair from a file
pub fn load_keypair(path: &Path) -> Result<KeyPair> {
    let secret_bytes = fs::read(path).context("Failed to read keypair file")?;

    if secret_bytes.len() != 32 {
        anyhow::bail!("Invalid keypair file: expected 32 bytes, got {}", secret_bytes.len());
    }

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&secret_bytes);

    let signing_key = SigningKey::from_bytes(&bytes);
    Ok(KeyPair::new(signing_key))
}

/// Load or generate a keypair
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
    use tempfile::tempdir;

    #[test]
    fn test_generate_keypair() {
        let keypair = generate_keypair();
        assert_eq!(keypair.public_key_bytes().len(), 32);
    }

    #[test]
    fn test_save_and_load_keypair() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_keypair.key");

        let original = generate_keypair();
        save_keypair(&original, &path).unwrap();

        let loaded = load_keypair(&path).unwrap();
        assert_eq!(original.public_key_bytes(), loaded.public_key_bytes());
    }

    #[test]
    fn test_load_or_generate() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_keypair.key");

        // First call should generate
        let first = load_or_generate_keypair(&path).unwrap();

        // Second call should load the same key
        let second = load_or_generate_keypair(&path).unwrap();

        assert_eq!(first.public_key_bytes(), second.public_key_bytes());
    }
}
