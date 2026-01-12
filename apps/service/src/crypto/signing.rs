use anyhow::Result;
use ed25519_dalek::Signer;
use serde::Serialize;
use std::time::SystemTime;

use super::keys::KeyPair;
use crate::monitoring::types::CheckResult;

/// Message structure for signing
#[derive(Serialize)]
struct SignableMessage {
    monitor_id: String,
    target: String,
    timestamp: u64,
    status: String,
    latency_ms: Option<u64>,
    status_code: Option<u16>,
    peer_id: String,
}

/// Sign a monitoring result
pub fn sign_result(result: &CheckResult, keypair: &KeyPair) -> Result<Vec<u8>> {
    // Create a canonical representation of the result for signing
    let message = SignableMessage {
        monitor_id: result.monitor_id.to_string(),
        target: result.target.clone(),
        timestamp: result.timestamp
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs(),
        status: result.status.to_string(),
        latency_ms: result.latency_ms,
        status_code: result.status_code,
        peer_id: result.peer_id.clone(),
    };

    // Serialize to JSON for canonical representation
    let message_bytes = serde_json::to_vec(&message)?;

    // Sign the message
    let signature = keypair.signing_key.sign(&message_bytes);

    Ok(signature.to_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::generate_keypair;
    use uuid::Uuid;

    #[test]
    fn test_sign_result() {
        let keypair = generate_keypair();
        let mut result = CheckResult::new(
            Uuid::new_v4(),
            "https://example.com".to_string(),
            "test-peer".to_string(),
        );
        result = result.success(100, Some(200));

        let signature = sign_result(&result, &keypair).unwrap();
        assert_eq!(signature.len(), 64); // Ed25519 signatures are 64 bytes
    }
}
