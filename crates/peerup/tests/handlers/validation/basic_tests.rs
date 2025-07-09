//! Basic validation tests

use peerup::handlers::validate_probe_request;
use peerup::protocol::ProbeRequest;

#[test]
fn test_validate_probe_request_valid() {
    let request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        body: None,
        headers: None,
        requested_by: "peer123".to_string(),
    };
    
    let result = validate_probe_request(&request);
    assert!(result.is_ok(), "Valid request should pass validation");
}

#[test]
fn test_validate_probe_request_invalid_method() {
    let request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "INVALID".to_string(),
        timeout: 5000,
        body: None,
        headers: None,
        requested_by: "peer123".to_string(),
    };
    
    let result = validate_probe_request(&request);
    assert!(result.is_err(), "Invalid HTTP method should fail validation");
}
