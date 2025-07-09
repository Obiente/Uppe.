//! Tests for ProbeRequest serialization and advanced functionality

use peerup::protocol::ProbeRequest;
use serde_json;

#[test]
fn test_probe_request_serialization() {
    let probe_request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "POST".to_string(),
        timeout: 5000,
        headers: Some(vec![("Content-Type".to_string(), "application/json".to_string(),)],),
        body: Some(r#"{"name": "John Doe"}"#.to_string(),),
        requested_by: "peer123".to_string(),
    };

    let serialized = serde_json::to_string(&probe_request,).unwrap();
    let deserialized: ProbeRequest = serde_json::from_str(&serialized,).unwrap();

    assert_eq!(probe_request.target_url, deserialized.target_url);
    assert_eq!(probe_request.method, deserialized.method);
    assert_eq!(probe_request.body, deserialized.body);
}

#[test]
fn test_probe_request_with_headers() {
    let headers = vec![
        ("Content-Type".to_string(), "application/json".to_string(),),
        ("Authorization".to_string(), "Bearer token123".to_string(),),
        ("User-Agent".to_string(), "PeerUP/1.0".to_string(),),
    ];

    let probe_request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        headers: Some(headers,),
        body: None,
        requested_by: "peer123".to_string(),
    };

    assert_eq!(probe_request.headers.as_ref().unwrap().len(), 3);
}

#[test]
fn test_probe_request_methods() {
    let methods = vec!["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH"];

    for method in methods {
        let probe_request = ProbeRequest {
            target_url: "https://example.com".to_string(),
            method: method.to_string(),
            timeout: 5000,
            headers: None,
            body: None,
            requested_by: "peer123".to_string(),
        };
        assert_eq!(probe_request.method, method);
    }
}

#[test]
fn test_probe_request_empty_headers() {
    let probe_request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        headers: None,
        body: None,
        requested_by: "peer123".to_string(),
    };

    assert!(probe_request.headers.is_none());
}
