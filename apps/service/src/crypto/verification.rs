#![allow(dead_code)]
use anyhow::{Result, anyhow};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Serialize;
use std::time::SystemTime;

use crate::database::models::PeerResult;

/// Message structure for verification (must match signing format)
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
/// Verify a peer result signature
pub fn verify_result(
    result: &PeerResult,
    public_key_bytes: &[u8; 32],
    target: &str,
) -> Result<bool> {
    // Parse the public key
    let verifying_key = VerifyingKey::from_bytes(public_key_bytes)
        .map_err(|e| anyhow!("Invalid public key: {}", e))?;

    // Parse the signature
    if result.signature.len() != 64 {
        return Ok(false);
    }
    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&result.signature);
    let signature = Signature::from_bytes(&sig_bytes);

    // Reconstruct the message that was signed
    let message = SignableMessage {
        monitor_id: result.monitor_uuid.to_string(),
        target: target.to_string(),
        timestamp: result.timestamp.duration_since(SystemTime::UNIX_EPOCH)?.as_secs(),
        status: result.status.to_string(),
        latency_ms: result.latency_ms,
        status_code: result.status_code,
        peer_id: result.peer_id.clone(),
    };

    // Serialize to JSON (same as signing)
    let message_bytes = serde_json::to_vec(&message)?;

    // Verify the signature
    match verifying_key.verify(&message_bytes, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::generate_keypair;
    use crate::crypto::signing::sign_result;
    use crate::monitoring::types::CheckResult;
    use crate::monitoring::types::MonitorStatus;
    use uuid::Uuid;

    #[test]
    fn test_verify_result() {
        let keypair = generate_keypair();
        let monitor_id = Uuid::new_v4();
        let target = "https://example.com".to_string();

        // Create and sign a result
        let mut check_result =
            CheckResult::new(monitor_id, target.clone(), "test-peer".to_string());
        check_result = check_result.success(100, Some(200));

        let signature = sign_result(&check_result, &keypair).unwrap();

        // Convert to PeerResult
        let peer_result = PeerResult {
            id: None,
            monitor_uuid: monitor_id,
            timestamp: check_result.timestamp,
            status: check_result.status,
            latency_ms: check_result.latency_ms,
            status_code: check_result.status_code,
            error_message: check_result.error_message,
            peer_id: check_result.peer_id,
            signature,
            verified: false,
            created_at: SystemTime::now(),
            city: None,
            country: None,
            region: None,
        };

        // Verify the signature
        let is_valid = verify_result(&peer_result, &keypair.public_key_bytes(), &target).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_verify_invalid_signature() {
        let keypair = generate_keypair();
        let monitor_id = Uuid::new_v4();

        // Create a peer result with invalid signature
        let peer_result = PeerResult {
            id: None,
            monitor_uuid: monitor_id,
            timestamp: SystemTime::now(),
            status: MonitorStatus::Up,
            latency_ms: Some(100),
            status_code: Some(200),
            error_message: None,
            peer_id: "test-peer".to_string(),
            signature: vec![0u8; 64], // Invalid signature
            verified: false,
            created_at: SystemTime::now(),
            city: None,
            country: None,
            region: None,
        };

        let is_valid =
            verify_result(&peer_result, &keypair.public_key_bytes(), "https://example.com")
                .unwrap();

        assert!(!is_valid);
    }
}
