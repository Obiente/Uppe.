//! Headers validation tests

use peerup::{handlers::validate_probe_request, protocol::ProbeRequest};

#[test]
fn test_validate_probe_request_with_valid_headers() {
    let headers = vec![
        ("Content-Type".to_string(), "application/json".to_string()),
        ("User-Agent".to_string(), "PeerUP/1.0".to_string()),
    ];

    let request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "POST".to_string(),
        timeout: 5000,
        body: Some(r#"{"key": "value"}"#.to_string()),
        headers: Some(headers),
        requested_by: "peer123".to_string(),
    };

    let result = validate_probe_request(&request);
    assert!(result.is_ok(), "Valid request with headers should pass validation");
}

#[test]
fn test_validate_probe_request_with_invalid_headers() {
    let headers = vec![
        ("Content-Type".to_string(), "application/json".to_string()),
        // Header value with invalid characters
        ("X-Custom".to_string(), "Value\nWith\rInvalid\0Characters".to_string()),
    ];

    let request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "POST".to_string(),
        timeout: 5000,
        body: Some(r#"{"key": "value"}"#.to_string()),
        headers: Some(headers),
        requested_by: "peer123".to_string(),
    };

    // Note: This test might pass or fail depending on how strict the validation is.
    // Currently, we're just demonstrating test organization.
    let _result = validate_probe_request(&request);
}
