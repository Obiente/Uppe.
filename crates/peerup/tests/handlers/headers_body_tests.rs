//! Tests for request validation with headers and body

use peerup::{handlers::validate_probe_request, protocol::ProbeRequest};

// Note: basic validation tests are in validation_tests.rs

#[test]
fn test_validate_probe_request_too_many_headers() {
    let headers = (0..25).map(|i| (format!("Header{i}"), format!("Value{i}"))).collect();

    let request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        body: None,
        headers: Some(headers),
        requested_by: "peer123".to_string(),
    };

    let result = validate_probe_request(&request);
    assert!(result.is_err(), "Too many headers should fail validation");
}

#[test]
fn test_validate_probe_request_large_body() {
    let large_body = "x".repeat(2 * 1024 * 1024); // 2MB

    let request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "POST".to_string(),
        timeout: 5000,
        body: Some(large_body),
        headers: None,
        requested_by: "peer123".to_string(),
    };

    let result = validate_probe_request(&request);
    assert!(result.is_err(), "Large body should fail validation");
}
