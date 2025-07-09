//! Request validation for PeerUP handlers.
//!
//! This module provides validation for incoming probe requests.

use anyhow::Result;
use url::Url;

use crate::protocol::ProbeRequest;

/// Validate a probe request
pub fn validate_probe_request(request: &ProbeRequest,) -> Result<(),> {
    // Validate URL
    validate_url(&request.target_url,)?;

    // Validate HTTP method
    validate_http_method(&request.method,)?;

    // Validate timeout
    validate_timeout(request.timeout,)?;

    // Validate headers
    if let Some(headers,) = &request.headers {
        validate_headers(headers,)?;
    }

    // Validate body size
    if let Some(body,) = &request.body {
        validate_body_size(body,)?;
    }

    Ok((),)
}

/// Validate URL format and scheme
fn validate_url(url: &str,) -> Result<(),> {
    let parsed = Url::parse(url,)?;

    match parsed.scheme() {
        "http" | "https" => Ok((),),
        _ => Err(anyhow::anyhow!("Unsupported URL scheme: {}", parsed.scheme()),),
    }
}

/// Validate HTTP method
fn validate_http_method(method: &str,) -> Result<(),> {
    match method.to_uppercase().as_str() {
        "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "PATCH" => Ok((),),
        _ => Err(anyhow::anyhow!("Unsupported HTTP method: {}", method),),
    }
}

/// Validate timeout value
fn validate_timeout(timeout: u64,) -> Result<(),> {
    const MAX_TIMEOUT: u64 = 300_000; // 5 minutes
    const MIN_TIMEOUT: u64 = 100; // 100ms

    if timeout < MIN_TIMEOUT {
        return Err(anyhow::anyhow!("Timeout too small: {} ms (min: {} ms)", timeout, MIN_TIMEOUT),);
    }

    if timeout > MAX_TIMEOUT {
        return Err(anyhow::anyhow!("Timeout too large: {} ms (max: {} ms)", timeout, MAX_TIMEOUT),);
    }

    Ok((),)
}

/// Validate headers
fn validate_headers(headers: &[(String, String,)],) -> Result<(),> {
    const MAX_HEADERS: usize = 20;
    const MAX_HEADER_SIZE: usize = 8192;

    if headers.len() > MAX_HEADERS {
        return Err(anyhow::anyhow!("Too many headers: {} (max: {})", headers.len(), MAX_HEADERS),);
    }

    for (key, value,) in headers {
        if key.len() + value.len() > MAX_HEADER_SIZE {
            return Err(anyhow::anyhow!(
                "Header too large: {} bytes (max: {} bytes)",
                key.len() + value.len(),
                MAX_HEADER_SIZE
            ),);
        }
    }

    Ok((),)
}

/// Validate body size
fn validate_body_size(body: &str,) -> Result<(),> {
    const MAX_BODY_SIZE: usize = 1024 * 1024; // 1MB

    if body.len() > MAX_BODY_SIZE {
        return Err(anyhow::anyhow!(
            "Body too large: {} bytes (max: {} bytes)",
            body.len(),
            MAX_BODY_SIZE
        ),);
    }

    Ok((),)
}
