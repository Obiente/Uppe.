//! Basic tests for ProbeRequest creation and validation

use peerup::protocol::ProbeRequest;

#[test]
fn test_probe_request_creation() {
    let headers = vec![("User-Agent".to_string(), "PeerUP/1.0".to_string(),)];

    let probe_request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        headers: Some(headers,),
        body: None,
        requested_by: "peer123".to_string(),
    };

    assert_eq!(probe_request.target_url, "https://example.com");
    assert_eq!(probe_request.method, "GET");
    assert_eq!(probe_request.headers.as_ref().unwrap().len(), 1);
    assert_eq!(probe_request.body, None);
    assert_eq!(probe_request.timeout, 5000);
    assert_eq!(probe_request.requested_by, "peer123");
}

#[test]
fn test_probe_request_with_body() {
    let headers = vec![("Content-Type".to_string(), "application/json".to_string(),)];

    let probe_request = ProbeRequest {
        target_url: "https://api.example.com/users".to_string(),
        method: "POST".to_string(),
        timeout: 5000,
        headers: Some(headers,),
        body: Some(r#"{"name": "John Doe"}"#.to_string(),),
        requested_by: "peer123".to_string(),
    };

    assert_eq!(probe_request.target_url, "https://api.example.com/users");
    assert_eq!(probe_request.method, "POST");
    assert_eq!(probe_request.headers.as_ref().unwrap().len(), 1);
    assert!(probe_request.body.is_some());
}

#[test]
fn test_probe_request_default_options() {
    // Default options
    let probe_request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        headers: None,
        body: None,
        requested_by: "peer123".to_string(),
    };

    assert_eq!(probe_request.method, "GET");
    assert!(probe_request.headers.is_none());
    assert!(probe_request.body.is_none());
}
