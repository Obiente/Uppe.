//! Protocol serialization tests for ProbeRequest and ProbeResponse

use peerup::protocol::{ProbeRequest, ProbeResponse};
use serde_json;

#[test]
fn test_probe_request_serialization_roundtrip() {
    let probe_request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        headers: None,
        body: Some("test body".to_string(),),
        requested_by: "peer123".to_string(),
    };
    let serialized = serde_json::to_string(&probe_request,).unwrap();
    let deserialized: ProbeRequest = serde_json::from_str(&serialized,).unwrap();
    assert_eq!(probe_request.target_url, deserialized.target_url);
    assert_eq!(probe_request.method, deserialized.method);
    assert_eq!(probe_request.timeout, deserialized.timeout);
    assert_eq!(probe_request.headers, deserialized.headers);
    assert_eq!(probe_request.body, deserialized.body);
    assert_eq!(probe_request.requested_by, deserialized.requested_by);
}

#[test]
fn test_probe_response_serialization_roundtrip() {
    let probe_response = ProbeResponse {
        status: Some(200,),
        duration: 100,
        error: None,
        headers: None,
        body: Some("OK".to_string(),),
        probed_by: "peer123".to_string(),
        timestamp: 1234567890,
    };
    let serialized = serde_json::to_string(&probe_response,).unwrap();
    let deserialized: ProbeResponse = serde_json::from_str(&serialized,).unwrap();
    assert_eq!(probe_response.status, deserialized.status);
    assert_eq!(probe_response.duration, deserialized.duration);
    assert_eq!(probe_response.body, deserialized.body);
    assert_eq!(probe_response.probed_by, deserialized.probed_by);
    assert_eq!(probe_response.timestamp, deserialized.timestamp);
}
