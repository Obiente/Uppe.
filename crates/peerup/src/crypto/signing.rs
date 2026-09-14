//! Ed25519 signing operations.
//!
//! Generic byte-level and JSON-level signing. Application-specific
//! signing wrappers (e.g., signing a `CheckResult`) live in the
//! application layer, not here.

use super::keys::KeyPair;
use ed25519_dalek::Signer;

/// Sign raw bytes with a keypair. Returns 64-byte Ed25519 signature.
pub fn sign_bytes(data: &[u8], keypair: &KeyPair) -> Vec<u8> {
    let signature = keypair.signing_key.sign(data);
    signature.to_bytes().to_vec()
}

/// Sign a JSON-serializable value. Serializes to canonical JSON bytes,
/// then signs with Ed25519.
pub fn sign_json<T: serde::Serialize>(value: &T, keypair: &KeyPair) -> anyhow::Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value)?;
    Ok(sign_bytes(&bytes, keypair))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::generate_keypair;
    use crate::crypto::verification::verify_signature;

    #[test]
    fn test_sign_bytes_produces_64_byte_signature() {
        let keypair = generate_keypair();
        let sig = sign_bytes(b"hello world", &keypair);
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn test_sign_bytes_verifies() {
        let keypair = generate_keypair();
        let data = b"test message";
        let sig = sign_bytes(data, &keypair);
        assert!(verify_signature(data, &sig, &keypair.public_key_bytes()).unwrap());
    }

    #[test]
    fn test_sign_json() {
        let keypair = generate_keypair();
        let value = serde_json::json!({"key": "value", "num": 42});
        let sig = sign_json(&value, &keypair).unwrap();
        assert_eq!(sig.len(), 64);

        // Verify against the same JSON serialization
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(verify_signature(&bytes, &sig, &keypair.public_key_bytes()).unwrap());
    }
}
