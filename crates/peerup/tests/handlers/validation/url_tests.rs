//! URL validation tests

use peerup::{handlers::validate_probe_request, protocol::ProbeRequest};

#[test]
fn test_validate_probe_request_invalid_url() {
    let request = ProbeRequest {
        target_url: "ftp://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        body: None,
        headers: None,
        requested_by: "peer123".to_string(),
    };

    let result = validate_probe_request(&request);
    assert!(result.is_err(), "Invalid URL scheme should fail validation");
}

#[test]
fn test_validate_probe_request_malformed_url() {
    let request = ProbeRequest {
        target_url: "not-a-url".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        body: None,
        headers: None,
        requested_by: "peer123".to_string(),
    };

    let result = validate_probe_request(&request);
    assert!(result.is_err(), "Malformed URL should fail validation");
}
