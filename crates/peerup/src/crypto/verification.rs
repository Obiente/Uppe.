//! Ed25519 signature verification.
//!
//! Generic byte-level verification. Application-specific verification
//! (e.g., reconstructing a `SignableMessage` from a `PeerResult`) lives
//! in the application layer.

use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Verify an Ed25519 signature over raw bytes.
///
/// Returns `Ok(true)` if valid, `Ok(false)` if the signature doesn't match,
/// or `Err` if the public key is malformed.
pub fn verify_signature(
    data: &[u8],
    signature_bytes: &[u8],
    public_key_bytes: &[u8; 32],
) -> Result<bool> {
    let verifying_key = VerifyingKey::from_bytes(public_key_bytes)
        .map_err(|e| anyhow!("Invalid public key: {}", e))?;

    if signature_bytes.len() != 64 {
        return Ok(false);
    }

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(signature_bytes);
    let signature = Signature::from_bytes(&sig_arr);

    Ok(verifying_key.verify(data, &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::generate_keypair;
    use crate::crypto::signing::sign_bytes;

    #[test]
    fn test_valid_signature() {
        let keypair = generate_keypair();
        let data = b"test data";
        let sig = sign_bytes(data, &keypair);

        assert!(verify_signature(data, &sig, &keypair.public_key_bytes()).unwrap());
    }

    #[test]
    fn test_invalid_signature_bytes() {
        let keypair = generate_keypair();
        let bad_sig = vec![0u8; 64];

        assert!(!verify_signature(b"data", &bad_sig, &keypair.public_key_bytes()).unwrap());
    }

    #[test]
    fn test_wrong_length_signature() {
        let keypair = generate_keypair();
        let short_sig = vec![0u8; 32];

        assert!(!verify_signature(b"data", &short_sig, &keypair.public_key_bytes()).unwrap());
    }

    #[test]
    fn test_tampered_data() {
        let keypair = generate_keypair();
        let sig = sign_bytes(b"original", &keypair);

        assert!(!verify_signature(b"tampered", &sig, &keypair.public_key_bytes()).unwrap());
    }

    #[test]
    fn test_wrong_key() {
        let keypair1 = generate_keypair();
        let keypair2 = generate_keypair();
        let sig = sign_bytes(b"data", &keypair1);

        assert!(!verify_signature(b"data", &sig, &keypair2.public_key_bytes()).unwrap());
    }
}
