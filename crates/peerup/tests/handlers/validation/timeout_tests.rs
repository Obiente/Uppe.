//! Timeout validation tests

use peerup::handlers::validate_probe_request;
use peerup::protocol::ProbeRequest;

#[test]
fn test_validate_probe_request_timeout_too_small() {
    let request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 50, // Below minimum
        body: None,
        headers: None,
        requested_by: "peer123".to_string(),
    };
    
    let result = validate_probe_request(&request);
    assert!(result.is_err(), "Timeout too small should fail validation");
}

#[test]
fn test_validate_probe_request_timeout_too_large() {
    let request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 400_000, // Above maximum
        body: None,
        headers: None,
        requested_by: "peer123".to_string(),
    };
    
    let result = validate_probe_request(&request);
    assert!(result.is_err(), "Timeout too large should fail validation");
}
