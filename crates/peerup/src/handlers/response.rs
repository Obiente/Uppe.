//! Response utilities for PeerUP handlers.
//!
//! This module provides utilities for building probe responses.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::protocol::ProbeResponse;

/// Build a successful probe response
pub fn build_success_response(
    status: u16,
    duration: u64,
    probed_by: String,
    headers: Option<Vec<(String, String,),>,>,
) -> ProbeResponse {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH,).unwrap_or_default().as_secs();

    ProbeResponse {
        status: Some(status,),
        duration,
        error: None,
        probed_by,
        timestamp,
        headers,
        body: None, // TODO: Add body if needed
    }
}

/// Build an error probe response
pub fn build_error_response(error: String, duration: u64, probed_by: String,) -> ProbeResponse {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH,).unwrap_or_default().as_secs();

    ProbeResponse {
        status: None,
        duration,
        error: Some(error,),
        probed_by,
        timestamp,
        headers: None,
        body: None,
    }
}

/// Build a timeout probe response
pub fn build_timeout_response(duration: u64, probed_by: String,) -> ProbeResponse {
    build_error_response("Request timed out".to_string(), duration, probed_by,)
}

/// Build a network error probe response
pub fn build_network_error_response(
    error: String,
    duration: u64,
    probed_by: String,
) -> ProbeResponse {
    build_error_response(format!("Network error: {error}"), duration, probed_by,)
}
