//! Tests for validation of HTTP methods and headers

use peerup::{handlers::validate_probe_request, protocol::ProbeRequest};

#[test]
fn test_validate_probe_request_all_http_methods() {
    let methods = vec!["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH"];

    for method in methods {
        let request = ProbeRequest {
            target_url: "https://example.com".to_string(),
            method: method.to_string(),
            timeout: 5000,
            body: None,
            headers: None,
            requested_by: "peer123".to_string(),
        };

        let result = validate_probe_request(&request,);
        assert!(result.is_ok(), "Method {method} should be valid");
    }
}

#[test]
fn test_validate_probe_request_case_insensitive_method() {
    let methods = vec!["get", "post", "PUT", "Delete", "head", "OPTIONS", "patch"];

    for method in methods {
        let request = ProbeRequest {
            target_url: "https://example.com".to_string(),
            method: method.to_string(),
            timeout: 5000,
            body: None,
            headers: None,
            requested_by: "peer123".to_string(),
        };

        let result = validate_probe_request(&request,);
        assert!(result.is_ok(), "Method {method} should be valid (case insensitive)");
    }
}
