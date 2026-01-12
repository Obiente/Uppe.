use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Status of a monitoring check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MonitorStatus {
    Up,
    Down,
    Degraded,
    Unknown,
}

impl std::fmt::Display for MonitorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitorStatus::Up => write!(f, "up"),
            MonitorStatus::Down => write!(f, "down"),
            MonitorStatus::Degraded => write!(f, "degraded"),
            MonitorStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Result of a monitoring check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// UUID of the monitor that was checked
    pub monitor_id: Uuid,

    /// URL or target that was checked
    pub target: String,

    /// Timestamp when the check was performed
    pub timestamp: SystemTime,

    /// Status of the check (up/down/degraded/unknown)
    pub status: MonitorStatus,

    /// Response time in milliseconds
    pub latency_ms: Option<u64>,

    /// HTTP status code (if applicable)
    pub status_code: Option<u16>,

    /// Error message (if check failed)
    pub error_message: Option<String>,

    /// ID of the peer that performed this check
    pub peer_id: String,

    /// Cryptographic signature of this result
    pub signature: Option<Vec<u8>>,
}

impl CheckResult {
    /// Create a new check result
    pub fn new(monitor_id: Uuid, target: String, peer_id: String) -> Self {
        Self {
            monitor_id,
            target,
            timestamp: SystemTime::now(),
            status: MonitorStatus::Unknown,
            latency_ms: None,
            status_code: None,
            error_message: None,
            peer_id,
            signature: None,
        }
    }

    /// Mark the check as successful with latency
    pub fn success(mut self, latency_ms: u64, status_code: Option<u16>) -> Self {
        self.status = MonitorStatus::Up;
        self.latency_ms = Some(latency_ms);
        self.status_code = status_code;
        self
    }

    /// Mark the check as failed with error
    pub fn failure(mut self, error: String) -> Self {
        self.status = MonitorStatus::Down;
        self.error_message = Some(error);
        self
    }

    /// Mark the check as degraded (slow response)
    pub fn degraded(mut self, latency_ms: u64, status_code: Option<u16>) -> Self {
        self.status = MonitorStatus::Degraded;
        self.latency_ms = Some(latency_ms);
        self.status_code = status_code;
        self
    }

    /// Add cryptographic signature to the result
    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = Some(signature);
        self
    }
}
