use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use ed25519_dalek::{Signature, Signer, VerifyingKey};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::crypto::{KeyPair, load_or_generate_keypair};
use crate::database::Database;
use crate::database::models::{AuditAttestation, AuditEvent, Monitor, PeerResult};
use crate::monitoring::types::CheckResult;

#[derive(Debug, Clone, Serialize)]
struct SignableEventEnvelope<'a> {
    event_id: &'a str,
    event_type: &'a str,
    schema_version: i32,
    created_at: i64,
    actor_id: &'a str,
    actor_public_key_hex: String,
    resource_type: &'a str,
    resource_id: &'a str,
    parent_event_id: Option<String>,
    payload_json: &'a str,
    payload_hash: &'a str,
    capability_id: Option<&'a str>,
    delegated_by: Option<&'a str>,
    expires_at: Option<i64>,
    context_json: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
struct SignableAttestationEnvelope<'a> {
    attestation_id: &'a str,
    subject_event_id: &'a str,
    attestor_id: &'a str,
    attestor_public_key_hex: String,
    decision: &'a str,
    reason: Option<&'a str>,
    created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedEventEnvelope {
    pub event_id: Uuid,
    pub event_type: String,
    pub schema_version: i32,
    pub created_at: SystemTime,
    pub actor_id: String,
    pub actor_public_key: Vec<u8>,
    pub resource_type: String,
    pub resource_id: String,
    pub parent_event_id: Option<Uuid>,
    pub payload_json: String,
    pub payload_hash: String,
    pub capability_id: Option<String>,
    pub delegated_by: Option<String>,
    pub expires_at: Option<SystemTime>,
    pub context_json: Option<String>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct NewSignedEvent<'a, T: Serialize, C: Serialize> {
    pub event_type: &'a str,
    pub actor_id: &'a str,
    pub resource_type: &'a str,
    pub resource_id: &'a str,
    pub parent_event_id: Option<Uuid>,
    pub payload: &'a T,
    pub capability_id: Option<&'a str>,
    pub delegated_by: Option<&'a str>,
    pub expires_at: Option<SystemTime>,
    pub context: Option<&'a C>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditDecision {
    Accepted,
    Rejected,
    Observed,
}

impl AuditDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Observed => "observed",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedAuditAttestation {
    pub attestation_id: Uuid,
    pub subject_event_id: Uuid,
    pub attestor_id: String,
    pub attestor_public_key: Vec<u8>,
    pub decision: AuditDecision,
    pub reason: Option<String>,
    pub created_at: SystemTime,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct NewAuditAttestation<'a> {
    pub subject_event_id: Uuid,
    pub attestor_id: &'a str,
    pub decision: AuditDecision,
    pub reason: Option<&'a str>,
}

impl SignedEventEnvelope {
    pub const SCHEMA_VERSION: i32 = 1;

    pub fn sign<T: Serialize, C: Serialize>(
        spec: NewSignedEvent<'_, T, C>,
        keypair: &KeyPair,
    ) -> Result<Self> {
        anyhow::ensure!(
            spec.actor_id == keypair.public_key_hex(),
            "Actor identity does not match signing key"
        );
        let payload_json = canonical_json_string(&serde_json::to_value(spec.payload)?)?;
        let payload_hash = hex::encode(Sha256::digest(payload_json.as_bytes()));
        let context_json = match spec.context {
            Some(context) => Some(canonical_json_string(&serde_json::to_value(context)?)?),
            None => None,
        };

        let mut event = Self {
            event_id: Uuid::new_v4(),
            event_type: spec.event_type.to_string(),
            schema_version: Self::SCHEMA_VERSION,
            created_at: SystemTime::now(),
            actor_id: spec.actor_id.to_string(),
            actor_public_key: keypair.public_key_bytes().to_vec(),
            resource_type: spec.resource_type.to_string(),
            resource_id: spec.resource_id.to_string(),
            parent_event_id: spec.parent_event_id,
            payload_json,
            payload_hash,
            capability_id: spec.capability_id.map(ToOwned::to_owned),
            delegated_by: spec.delegated_by.map(ToOwned::to_owned),
            expires_at: spec.expires_at,
            context_json,
            signature: Vec::new(),
        };

        let signature = keypair.signing_key.sign(&event.signing_bytes()?);
        event.signature = signature.to_bytes().to_vec();
        Ok(event)
    }

    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "Audit verification API is exercised by integrity tests")
    )]
    pub fn verify(&self) -> Result<bool> {
        let public_key_bytes: [u8; 32] = self
            .actor_public_key
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid actor public key length"))?;
        let signature_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid signature length"))?;

        if self.actor_id != hex::encode(&self.actor_public_key)
            || self.schema_version != Self::SCHEMA_VERSION
        {
            return Ok(false);
        }
        let expected_hash = hex::encode(Sha256::digest(self.payload_json.as_bytes()));
        if expected_hash != self.payload_hash {
            return Ok(false);
        }

        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
            .map_err(|e| anyhow!("invalid actor public key: {e}"))?;
        let signature = Signature::from_bytes(&signature_bytes);
        Ok(verifying_key.verify_strict(&self.signing_bytes()?, &signature).is_ok())
    }

    #[allow(dead_code)]
    pub fn payload_value(&self) -> Result<Value> {
        Ok(serde_json::from_str(&self.payload_json)?)
    }

    #[allow(dead_code)]
    pub fn context_value(&self) -> Result<Option<Value>> {
        self.context_json
            .as_ref()
            .map(|json| serde_json::from_str(json).map_err(Into::into))
            .transpose()
    }

    fn signing_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&SignableEventEnvelope {
            event_id: &self.event_id.to_string(),
            event_type: &self.event_type,
            schema_version: self.schema_version,
            created_at: system_time_to_i64(self.created_at),
            actor_id: &self.actor_id,
            actor_public_key_hex: hex::encode(&self.actor_public_key),
            resource_type: &self.resource_type,
            resource_id: &self.resource_id,
            parent_event_id: self.parent_event_id.map(|id| id.to_string()),
            payload_json: &self.payload_json,
            payload_hash: &self.payload_hash,
            capability_id: self.capability_id.as_deref(),
            delegated_by: self.delegated_by.as_deref(),
            expires_at: self.expires_at.map(system_time_to_i64),
            context_json: self.context_json.as_deref(),
        })?)
    }
}

impl SignedAuditAttestation {
    pub fn sign(spec: NewAuditAttestation<'_>, keypair: &KeyPair) -> Result<Self> {
        anyhow::ensure!(
            spec.attestor_id == keypair.public_key_hex(),
            "Attestor identity does not match signing key"
        );
        let mut attestation = Self {
            attestation_id: Uuid::new_v4(),
            subject_event_id: spec.subject_event_id,
            attestor_id: spec.attestor_id.to_string(),
            attestor_public_key: keypair.public_key_bytes().to_vec(),
            decision: spec.decision,
            reason: spec.reason.map(ToOwned::to_owned),
            created_at: SystemTime::now(),
            signature: Vec::new(),
        };

        let signature = keypair.signing_key.sign(&attestation.signing_bytes()?);
        attestation.signature = signature.to_bytes().to_vec();
        Ok(attestation)
    }

    #[cfg_attr(
        not(test),
        allow(dead_code, reason = "Audit verification API is exercised by integrity tests")
    )]
    pub fn verify(&self) -> Result<bool> {
        if self.attestor_id != hex::encode(&self.attestor_public_key) {
            return Ok(false);
        }
        let public_key_bytes: [u8; 32] = self
            .attestor_public_key
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid attestor public key length"))?;
        let signature_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid attestation signature length"))?;

        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
            .map_err(|e| anyhow!("invalid attestor public key: {e}"))?;
        let signature = Signature::from_bytes(&signature_bytes);
        Ok(verifying_key.verify_strict(&self.signing_bytes()?, &signature).is_ok())
    }

    #[allow(dead_code, reason = "Conversion for retained operator-ledger attestations")]
    pub fn to_model(&self) -> AuditAttestation {
        AuditAttestation {
            id: None,
            attestation_uuid: self.attestation_id,
            subject_event_uuid: self.subject_event_id,
            attestor_id: self.attestor_id.clone(),
            attestor_public_key: self.attestor_public_key.clone(),
            decision: self.decision.as_str().to_string(),
            reason: self.reason.clone(),
            created_at: self.created_at,
            signature: self.signature.clone(),
        }
    }

    fn signing_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&SignableAttestationEnvelope {
            attestation_id: &self.attestation_id.to_string(),
            subject_event_id: &self.subject_event_id.to_string(),
            attestor_id: &self.attestor_id,
            attestor_public_key_hex: hex::encode(&self.attestor_public_key),
            decision: self.decision.as_str(),
            reason: self.reason.as_deref(),
            created_at: system_time_to_i64(self.created_at),
        })?)
    }
}

pub fn canonical_json_string(value: &Value) -> Result<String> {
    Ok(serde_json::to_string(&canonicalize_value(value))?)
}

#[derive(Debug, Serialize)]
struct MonitorAuditPayload<'a> {
    action: &'a str,
    monitor_uuid: Uuid,
    name: &'a str,
    target: &'a str,
    check_type: &'a str,
    interval_seconds: u64,
    timeout_seconds: u64,
    enabled: bool,
    visibility: &'a crate::database::models::MonitorVisibility,
    public_domain: Option<&'a str>,
    public_display_name: Option<&'a str>,
    owner_peer_id: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct ResultAuditPayload<'a> {
    source: &'a str,
    monitor_uuid: Uuid,
    target: &'a str,
    check_type: &'a str,
    status: &'a crate::monitoring::types::MonitorStatus,
    latency_ms: Option<u64>,
    status_code: Option<u16>,
    error_message: Option<&'a str>,
    peer_id: &'a str,
    timestamp: i64,
    verified: Option<bool>,
    source_peer_id: Option<&'a str>,
    synced_from_peer: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct AdminTrustAuditPayload<'a> {
    action: &'a str,
    source: &'a str,
    version: u64,
    previous_version: Option<u64>,
    key_count: usize,
    rotation_count: usize,
    revoked_key_count: usize,
}

pub fn load_local_keypair() -> Result<KeyPair> {
    let keypair_path = std::env::var("UPPE_KEYPAIR_PATH").unwrap_or_else(|_| {
        PathBuf::from(std::env::var("UPPE_DATA_DIR").unwrap_or_else(|_| ".uppe".into()))
            .join("node.key")
            .to_string_lossy()
            .into_owned()
    });
    let keypair_path = PathBuf::from(keypair_path);
    load_or_generate_keypair(&keypair_path)
        .with_context(|| format!("failed to load local audit keypair from {:?}", keypair_path))
}

pub async fn record_monitor_event(
    database: &dyn Database,
    keypair: &KeyPair,
    actor_id: &str,
    action: &str,
    monitor: &Monitor,
) -> Result<Uuid> {
    let event_type = match action {
        "created" => "monitor.created",
        "updated" => "monitor.updated",
        "deleted" => "monitor.deleted",
        _ => "monitor.updated",
    };
    let resource_id = monitor.uuid.to_string();

    let payload = MonitorAuditPayload {
        action,
        monitor_uuid: monitor.uuid,
        name: &monitor.name,
        target: &monitor.target,
        check_type: &monitor.check_type,
        interval_seconds: monitor.interval_seconds,
        timeout_seconds: monitor.timeout_seconds,
        enabled: monitor.enabled,
        visibility: &monitor.visibility,
        public_domain: monitor.public_domain.as_deref(),
        public_display_name: monitor.public_display_name.as_deref(),
        owner_peer_id: monitor.owner_peer_id.as_deref(),
    };

    save_signed_event(
        database,
        keypair,
        NewSignedEvent::<_, Value> {
            event_type,
            actor_id,
            resource_type: "monitor",
            resource_id: &resource_id,
            parent_event_id: None,
            payload: &payload,
            capability_id: None,
            delegated_by: None,
            expires_at: None,
            context: None,
        },
    )
    .await
}

pub async fn record_local_result_event(
    database: &dyn Database,
    keypair: &KeyPair,
    actor_id: &str,
    source: &str,
    result: &CheckResult,
) -> Result<Uuid> {
    let resource_id = result.monitor_id.to_string();
    let payload = ResultAuditPayload {
        source,
        monitor_uuid: result.monitor_id,
        target: &result.target,
        check_type: &result.check_type,
        status: &result.status,
        latency_ms: result.latency_ms,
        status_code: result.status_code,
        error_message: result.error_message.as_deref(),
        peer_id: &result.peer_id,
        timestamp: system_time_to_i64(result.timestamp),
        verified: None,
        source_peer_id: None,
        synced_from_peer: None,
    };

    save_signed_event(
        database,
        keypair,
        NewSignedEvent::<_, Value> {
            event_type: "monitor.result_recorded",
            actor_id,
            resource_type: "monitor",
            resource_id: &resource_id,
            parent_event_id: None,
            payload: &payload,
            capability_id: None,
            delegated_by: None,
            expires_at: None,
            context: None,
        },
    )
    .await
}

/// Sign an expiring receipt without appending remote claims to the permanent ledger.
pub fn peer_result_receipt(
    keypair: &KeyPair,
    actor_id: &str,
    source: &str,
    result: &PeerResult,
    target: &str,
) -> Result<String> {
    let resource_id = result.monitor_uuid.to_string();
    let payload = serde_json::json!({"source":source, "monitor_uuid":resource_id,
        "target":target, "timestamp":system_time_to_i64(result.timestamp), "status":result.status,
        "latency_ms":result.latency_ms, "status_code":result.status_code,
        "error_message":result.error_message, "peer_id":result.peer_id,
        "peer_signature":hex::encode(&result.signature), "verified":result.verified});
    let envelope = SignedEventEnvelope::sign(
        NewSignedEvent::<_, Value> {
            event_type: "peer.result_verified",
            actor_id,
            resource_type: "monitor",
            resource_id: &resource_id,
            parent_event_id: None,
            payload: &payload,
            capability_id: None,
            delegated_by: None,
            expires_at: Some(SystemTime::now() + std::time::Duration::from_secs(604800)),
            context: None,
        },
        keypair,
    )?;
    let attestation = SignedAuditAttestation::sign(
        NewAuditAttestation {
            subject_event_id: envelope.event_id,
            attestor_id: actor_id,
            decision: AuditDecision::Accepted,
            reason: Some("signature valid"),
        },
        keypair,
    )?;
    Ok(serde_json::to_string(
        &serde_json::json!({"event":envelope,"attestation":attestation}),
    )?)
}

#[allow(
    dead_code,
    clippy::too_many_arguments,
    reason = "Typed admin audit adapter retained for the trust manager"
)]
pub async fn record_admin_trust_event(
    database: &dyn Database,
    keypair: &KeyPair,
    actor_id: &str,
    action: &str,
    source: &str,
    version: u64,
    previous_version: Option<u64>,
    key_count: usize,
    rotation_count: usize,
    revoked_key_count: usize,
) -> Result<Uuid> {
    let payload = AdminTrustAuditPayload {
        action,
        source,
        version,
        previous_version,
        key_count,
        rotation_count,
        revoked_key_count,
    };

    save_signed_event(
        database,
        keypair,
        NewSignedEvent::<_, Value> {
            event_type: "admin_trust.chain_updated",
            actor_id,
            resource_type: "admin_trust_chain",
            resource_id: "global",
            parent_event_id: None,
            payload: &payload,
            capability_id: None,
            delegated_by: None,
            expires_at: None,
            context: None,
        },
    )
    .await
}

async fn save_signed_event<T: Serialize, C: Serialize>(
    database: &dyn Database,
    keypair: &KeyPair,
    spec: NewSignedEvent<'_, T, C>,
) -> Result<Uuid> {
    let envelope = SignedEventEnvelope::sign(spec, keypair)?;
    let event_uuid = envelope.event_id;
    let event = envelope_to_model(envelope);
    database.save_audit_event(&event).await?;
    Ok(event_uuid)
}

fn envelope_to_model(envelope: SignedEventEnvelope) -> AuditEvent {
    AuditEvent {
        id: None,
        event_uuid: envelope.event_id,
        event_type: envelope.event_type,
        schema_version: envelope.schema_version,
        created_at: envelope.created_at,
        actor_id: envelope.actor_id,
        actor_public_key: envelope.actor_public_key,
        resource_type: envelope.resource_type,
        resource_id: envelope.resource_id,
        parent_event_uuid: envelope.parent_event_id,
        payload_json: envelope.payload_json,
        payload_hash: envelope.payload_hash,
        capability_id: envelope.capability_id,
        delegated_by: envelope.delegated_by,
        expires_at: envelope.expires_at,
        context_json: envelope.context_json,
        signature: envelope.signature,
    }
}

fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut ordered = BTreeMap::new();
            for (key, value) in map {
                ordered.insert(key.clone(), canonicalize_value(value));
            }
            serde_json::to_value(ordered).expect("ordered map should serialize")
        }
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_value).collect()),
        _ => value.clone(),
    }
}

pub fn system_time_to_i64(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

#[allow(dead_code)]
pub fn i64_to_system_time(timestamp: i64) -> SystemTime {
    UNIX_EPOCH + std::time::Duration::from_secs(timestamp.max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use peerup::crypto::generate_keypair;
    use serde_json::json;

    #[test]
    fn canonical_json_orders_keys_recursively() {
        let value = json!({
            "z": 1,
            "a": { "b": 2, "a": 1 }
        });
        assert_eq!(canonical_json_string(&value).unwrap(), r#"{"a":{"a":1,"b":2},"z":1}"#);
    }

    #[test]
    fn signed_event_round_trips_verification() {
        let keypair = generate_keypair();
        let payload = json!({ "status": "up", "monitor_id": "mon-1" });

        let event = SignedEventEnvelope::sign(
            NewSignedEvent::<_, serde_json::Value> {
                event_type: "monitor.result.recorded",
                actor_id: &keypair.public_key_hex(),
                resource_type: "monitor",
                resource_id: "mon-1",
                parent_event_id: None,
                payload: &payload,
                capability_id: None,
                delegated_by: None,
                expires_at: None,
                context: None,
            },
            &keypair,
        )
        .unwrap();

        assert!(event.verify().unwrap());
    }

    #[test]
    fn signed_attestation_round_trips_verification() {
        let keypair = generate_keypair();
        let attestation = SignedAuditAttestation::sign(
            NewAuditAttestation {
                subject_event_id: Uuid::new_v4(),
                attestor_id: &keypair.public_key_hex(),
                decision: AuditDecision::Accepted,
                reason: Some("signature valid"),
            },
            &keypair,
        )
        .unwrap();

        assert!(attestation.verify().unwrap());
    }
}
