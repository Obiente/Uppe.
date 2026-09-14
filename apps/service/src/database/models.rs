use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::monitoring::types::MonitorStatus;

/// Monitor model - represents a monitoring target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    pub id: Option<i64>,
    pub uuid: Uuid,
    pub name: String,
    pub target: String,
    pub check_type: String,
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
    pub enabled: bool,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,

    /// Visibility mode (Public or Private)
    pub visibility: MonitorVisibility,

    /// Public domain (for public monitors, e.g., "google.com")
    pub public_domain: Option<String>,

    /// Display name for public monitors
    pub public_display_name: Option<String>,

    /// Owner peer ID (for private monitors)
    pub owner_peer_id: Option<String>,
}

/// Monitor visibility (matches PeerUP visibility types)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MonitorVisibility {
    /// Public monitor - community-owned, coordinated
    Public,
    /// Private monitor - owner-controlled, privacy-preserving
    #[default]
    Private,
    /// Internal monitor - owner-only, never shared (for databases, secrets)
    Internal,
}

impl Monitor {
    /// Create a new private monitor
    pub fn new(name: String, target: String, check_type: String) -> Self {
        let now = SystemTime::now();
        Self {
            id: None,
            uuid: Uuid::new_v4(),
            name,
            target,
            check_type,
            interval_seconds: 30,
            timeout_seconds: 10,
            enabled: true,
            created_at: now,
            updated_at: now,
            visibility: MonitorVisibility::Private,
            public_domain: None,
            public_display_name: None,
            owner_peer_id: None,
        }
    }

    /// Derive a deterministic UUID v5 for a public monitor from its domain.
    /// All peers that see the same domain will produce the identical UUID,
    /// so results can be aggregated across the network without coordination.
    pub fn uuid_for_public_domain(domain: &str) -> Uuid {
        // Fixed application namespace: b"uppe-public-mons" (16 bytes)
        const NS: Uuid = Uuid::from_bytes([
            0x75, 0x70, 0x70, 0x65, 0x2d, 0x70, 0x75, 0x62, 0x6c, 0x69, 0x63, 0x2d, 0x6d, 0x6f,
            0x6e, 0x73,
        ]);
        Uuid::new_v5(&NS, domain.to_lowercase().trim_end_matches('/').as_bytes())
    }

    /// Create a new public monitor
    pub fn new_public(
        name: String,
        target: String,
        check_type: String,
        domain: String,
        display_name: String,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id: None,
            uuid: Monitor::uuid_for_public_domain(&domain),
            name,
            target,
            check_type,
            interval_seconds: 60, // Public monitors default to 60s
            timeout_seconds: 10,
            enabled: true,
            created_at: now,
            updated_at: now,
            visibility: MonitorVisibility::Public,
            public_domain: Some(domain),
            public_display_name: Some(display_name),
            owner_peer_id: None,
        }
    }

    /// Check if this is a public monitor
    pub fn is_public(&self) -> bool {
        matches!(self.visibility, MonitorVisibility::Public)
    }

    /// Check if this is a private monitor (peer-assisted)
    pub fn is_private(&self) -> bool {
        matches!(self.visibility, MonitorVisibility::Private)
    }

    /// Convert SystemTime to Unix timestamp
    pub fn timestamp_to_i64(time: SystemTime) -> i64 {
        time.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
    }

    /// Convert Unix timestamp to SystemTime
    pub fn i64_to_timestamp(timestamp: i64) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64)
    }
}

/// MonitorResult model - represents a monitoring check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorResult {
    pub id: Option<i64>,
    pub monitor_uuid: Uuid,
    pub timestamp: SystemTime,
    pub status: MonitorStatus,
    pub latency_ms: Option<u64>,
    pub status_code: Option<u16>,
    pub error_message: Option<String>,
    pub peer_id: String,
    pub signature: Option<Vec<u8>>,
    pub created_at: SystemTime,
    pub city: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
}

impl MonitorResult {
    /// Create a new monitor result from a check result
    #[allow(dead_code)]
    pub fn from_check_result(check_result: &crate::monitoring::types::CheckResult) -> Self {
        let location = crate::location::get_location();
        Self {
            id: None,
            monitor_uuid: check_result.monitor_id,
            timestamp: check_result.timestamp,
            status: check_result.status,
            latency_ms: check_result.latency_ms,
            status_code: check_result.status_code,
            error_message: check_result.error_message.clone(),
            peer_id: check_result.peer_id.clone(),
            signature: check_result.signature.clone(),
            created_at: SystemTime::now(),
            city: location.city,
            country: location.country,
            region: location.region,
        }
    }
}

/// PeerResult model - represents a monitoring result received from another peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerResult {
    pub id: Option<i64>,
    pub monitor_uuid: Uuid,
    pub timestamp: SystemTime,
    pub status: MonitorStatus,
    pub latency_ms: Option<u64>,
    pub status_code: Option<u16>,
    pub error_message: Option<String>,
    pub peer_id: String,
    pub signature: Vec<u8>,
    pub verified: bool,
    pub created_at: SystemTime,
    pub city: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    /// The peer that originally submitted this result (for distributed monitoring)
    pub source_peer_id: Option<String>,
    /// Whether we synced this result from a peer (for cleanup tracking)
    pub synced_from_peer: bool,
    /// When to delete this result (Unix timestamp). None = keep indefinitely
    pub retention_until: Option<i64>,
}

impl PeerResult {
    /// Create a new peer result from a P2P received result
    pub fn from_p2p_result(p2p_result: &crate::p2p::PeerResult) -> Option<Self> {
        // Extract signature or return None if missing
        let signature = p2p_result.signature.clone()?;

        Self {
            id: None,
            monitor_uuid: p2p_result.result.monitor_id,
            timestamp: p2p_result.result.timestamp,
            status: p2p_result.result.status,
            latency_ms: p2p_result.result.latency_ms,
            status_code: p2p_result.result.status_code,
            error_message: p2p_result.result.error_message.clone(),
            peer_id: p2p_result.result.peer_id.clone(),
            signature,
            verified: false, // Will be verified later
            created_at: p2p_result.received_at,
            city: None, // TODO: Add geolocation lookup
            country: None,
            region: None,
            source_peer_id: Some(p2p_result.peer_id.clone()),
            synced_from_peer: false,
            retention_until: None,
        }
        .into()
    }
}

/// Peer metadata persisted from P2P discovery/connect events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub peer_id: String,
    pub status: String,
    pub last_seen: SystemTime,
    pub joined_at: SystemTime,
    pub contribution_score: f64,
    pub uptime_percentage: f64,
    pub checks_per_day: i64,
    pub location_city: Option<String>,
    pub location_region: Option<String>,
    pub location_country: Option<String>,
}

impl Peer {
    pub fn new_online(peer_id: String, now: SystemTime) -> Self {
        Self {
            peer_id,
            status: "online".to_string(),
            last_seen: now,
            joined_at: now,
            contribution_score: 1.0,
            uptime_percentage: 100.0,
            checks_per_day: 0,
            location_city: None,
            location_region: None,
            location_country: None,
        }
    }
}

/// Snapshot of network metrics stored periodically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub timestamp: SystemTime,
    pub total_peers: i64,
    pub online_peers: i64,
    pub checks_performed: i64,
    pub checks_received: i64,
    pub bandwidth_used_mb: i64,
}

/// Signed audit event persisted in the append-only event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Option<i64>,
    pub event_uuid: Uuid,
    pub event_type: String,
    pub schema_version: i32,
    pub created_at: SystemTime,
    pub actor_id: String,
    pub actor_public_key: Vec<u8>,
    pub resource_type: String,
    pub resource_id: String,
    pub parent_event_uuid: Option<Uuid>,
    pub payload_json: String,
    pub payload_hash: String,
    pub capability_id: Option<String>,
    pub delegated_by: Option<String>,
    pub expires_at: Option<SystemTime>,
    pub context_json: Option<String>,
    pub signature: Vec<u8>,
}

/// Peer attestation over a previously stored audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditAttestation {
    pub id: Option<i64>,
    pub attestation_uuid: Uuid,
    pub subject_event_uuid: Uuid,
    pub attestor_id: String,
    pub attestor_public_key: Vec<u8>,
    pub decision: String,
    pub reason: Option<String>,
    pub created_at: SystemTime,
    pub signature: Vec<u8>,
}
