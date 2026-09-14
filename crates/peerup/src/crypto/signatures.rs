//! Signed result verification for distributed P2P results.
//!
//! `SignedResult` is the on-the-wire format for monitoring results
//! shared via GossipSub and DHT. It bundles the result data with
//! an Ed25519 signature and the signer's public key so that any
//! peer can verify authenticity without a pre-shared key registry.

use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use tracing::warn;

/// A signed monitoring result for P2P distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedResult {
    /// Peer ID that created this result
    pub peer_id: String,
    /// Monitor UUID
    pub monitor_uuid: String,
    /// Result data (JSON-serialized)
    pub result_data: String,
    /// Timestamp (Unix seconds)
    pub timestamp: i64,
    /// Ed25519 signature over `peer_id:monitor_uuid:result_data:timestamp`
    pub signature: Vec<u8>,
    /// Ed25519 public key of the signer (32 bytes)
    pub public_key: Vec<u8>,
}

impl SignedResult {
    /// Create a new signed result.
    pub fn new(
        peer_id: String,
        monitor_uuid: String,
        result_data: String,
        timestamp: i64,
        signing_key: &ed25519_dalek::SigningKey,
    ) -> Self {
        let message = Self::create_message(&peer_id, &monitor_uuid, &result_data, timestamp);
        let signature = signing_key.sign(&message);
        let public_key = signing_key.verifying_key().to_bytes().to_vec();

        Self {
            peer_id,
            monitor_uuid,
            result_data,
            timestamp,
            signature: signature.to_bytes().to_vec(),
            public_key,
        }
    }

    /// Verify the Ed25519 signature.
    pub fn verify(&self) -> Result<()> {
        let message = Self::create_message(
            &self.peer_id,
            &self.monitor_uuid,
            &self.result_data,
            self.timestamp,
        );

        let public_key_bytes: [u8; 32] = self
            .public_key
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Invalid public key length"))?;

        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
            .map_err(|e| anyhow!("Invalid public key: {}", e))?;

        let signature_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Invalid signature length"))?;

        let signature = Signature::from_bytes(&signature_bytes);

        verifying_key
            .verify(&message, &signature)
            .map_err(|e| anyhow!("Signature verification failed: {}", e))?;

        Ok(())
    }

    /// Verify that the peer_id matches the embedded public key.
    ///
    /// The peer_id should be the hex-encoded Ed25519 public key.
    pub fn verify_peer_id(&self) -> Result<()> {
        if self.peer_id.is_empty() {
            return Err(anyhow!("Empty peer_id"));
        }

        // Verify peer_id is the hex-encoded public key
        let expected_hex = hex::encode(&self.public_key);
        if self.peer_id != expected_hex {
            return Err(anyhow!(
                "peer_id does not match public key: {} != {}",
                self.peer_id,
                expected_hex
            ));
        }

        Ok(())
    }

    /// Full verification: signature + peer_id binding.
    pub fn verify_full(&self) -> Result<()> {
        self.verify()?;
        self.verify_peer_id()?;
        Ok(())
    }

    /// Create the canonical message bytes for signing/verification.
    fn create_message(
        peer_id: &str,
        monitor_uuid: &str,
        result_data: &str,
        timestamp: i64,
    ) -> Vec<u8> {
        format!("{}:{}:{}:{}", peer_id, monitor_uuid, result_data, timestamp).into_bytes()
    }
}

/// Verify a result received from the network.
///
/// Checks:
/// 1. Ed25519 signature validity
/// 2. Peer ID matches public key
/// 3. Timestamp within acceptable range (±5 min future, ±24h past)
pub fn verify_received_result(signed_result: &SignedResult) -> Result<()> {
    signed_result.verify().map_err(|e| {
        warn!(
            target: "uppe::audit",
            event = "signature_verification_failed",
            peer_id = %signed_result.peer_id,
            monitor_id = %signed_result.monitor_uuid,
            "Rejected invalid result: {}",
            e
        );
        anyhow!("Signature verification failed: {}", e)
    })?;

    // Timestamp not too far in the future (5 minute tolerance)
    let now = chrono::Utc::now().timestamp();
    let max_future = now + 300;
    if signed_result.timestamp > max_future {
        warn!(
            target: "uppe::audit",
            event = "timestamp_future",
            peer_id = %signed_result.peer_id,
            "Rejected result with future timestamp: {} > {}",
            signed_result.timestamp, max_future
        );
        return Err(anyhow!("Result timestamp too far in future"));
    }

    // Timestamp not too old (24 hours tolerance)
    let min_past = now - 86400;
    if signed_result.timestamp < min_past {
        warn!(
            target: "uppe::audit",
            event = "timestamp_expired",
            peer_id = %signed_result.peer_id,
            "Rejected result with old timestamp: {} < {}",
            signed_result.timestamp, min_past
        );
        return Err(anyhow!("Result timestamp too old"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use rand::RngCore;

    fn random_signing_key() -> SigningKey {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        SigningKey::from_bytes(&seed)
    }

    #[test]
    fn test_sign_and_verify() {
        let signing_key = random_signing_key();
        let peer_id = hex::encode(signing_key.verifying_key().to_bytes());

        let result = SignedResult::new(
            peer_id,
            "monitor-uuid-123".to_string(),
            r#"{"status":"up","latency_ms":42}"#.to_string(),
            chrono::Utc::now().timestamp(),
            &signing_key,
        );

        assert!(result.verify().is_ok());
        assert!(result.verify_full().is_ok());
    }

    #[test]
    fn test_tampered_data() {
        let signing_key = random_signing_key();
        let peer_id = hex::encode(signing_key.verifying_key().to_bytes());

        let mut result = SignedResult::new(
            peer_id,
            "monitor-uuid-123".to_string(),
            r#"{"status":"up","latency_ms":42}"#.to_string(),
            chrono::Utc::now().timestamp(),
            &signing_key,
        );

        result.result_data = r#"{"status":"down","latency_ms":9999}"#.to_string();
        assert!(result.verify().is_err());
    }

    #[test]
    fn test_wrong_signature() {
        let key1 = random_signing_key();
        let key2 = random_signing_key();
        let peer_id = hex::encode(key1.verifying_key().to_bytes());

        let mut result = SignedResult::new(
            peer_id,
            "monitor-uuid-123".to_string(),
            r#"{"status":"up"}"#.to_string(),
            chrono::Utc::now().timestamp(),
            &key1,
        );

        let fake_sig = key2.sign(b"fake data");
        result.signature = fake_sig.to_bytes().to_vec();
        assert!(result.verify().is_err());
    }

    #[test]
    fn test_timestamp_validation() {
        let signing_key = random_signing_key();
        let peer_id = hex::encode(signing_key.verifying_key().to_bytes());

        // Future timestamp (10 minutes ahead) → rejected
        let future_result = SignedResult::new(
            peer_id.clone(),
            "m".to_string(),
            r#"{"status":"up"}"#.to_string(),
            chrono::Utc::now().timestamp() + 600,
            &signing_key,
        );
        assert!(verify_received_result(&future_result).is_err());

        // Old timestamp (25 hours ago) → rejected
        let old_result = SignedResult::new(
            peer_id.clone(),
            "m".to_string(),
            r#"{"status":"up"}"#.to_string(),
            chrono::Utc::now().timestamp() - 90000,
            &signing_key,
        );
        assert!(verify_received_result(&old_result).is_err());

        // Current timestamp → accepted
        let valid_result = SignedResult::new(
            peer_id,
            "m".to_string(),
            r#"{"status":"up"}"#.to_string(),
            chrono::Utc::now().timestamp(),
            &signing_key,
        );
        assert!(verify_received_result(&valid_result).is_ok());
    }

    #[test]
    fn test_peer_id_verification() {
        let signing_key = random_signing_key();
        let correct_peer_id = hex::encode(signing_key.verifying_key().to_bytes());

        let result = SignedResult::new(
            correct_peer_id,
            "m".to_string(),
            "data".to_string(),
            chrono::Utc::now().timestamp(),
            &signing_key,
        );

        assert!(result.verify_peer_id().is_ok());

        // Wrong peer_id should fail
        let mut bad_result = result.clone();
        bad_result.peer_id = "wrong-peer-id".to_string();
        assert!(bad_result.verify_peer_id().is_err());
    }
}
