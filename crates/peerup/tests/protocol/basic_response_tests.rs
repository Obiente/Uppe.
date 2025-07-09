//! Basic tests for ProbeResponse creation and validation



use peerup::protocol::ProbeResponse;

#[test]
fn test_probe_response_creation() {
    let probe_response = ProbeResponse {
        status: Some(200),
        duration: 250,
        error: None,
        headers: Some(vec![("Content-Type".to_string(), "text/html".to_string())]),
        body: None,
        probed_by: "peer123".to_string(),
        timestamp: 1234567890,
    };

    assert_eq!(probe_response.status, Some(200));
    assert_eq!(probe_response.duration, 250);
    assert_eq!(probe_response.error, None);
    assert_eq!(probe_response.headers.as_ref().unwrap().len(), 1);
    assert_eq!(probe_response.probed_by, "peer123");
    assert_eq!(probe_response.timestamp, 1234567890);
}

#[test]
fn test_probe_response_with_error() {
    let probe_response = ProbeResponse {
        status: None,
        duration: 0,
        error: Some("Connection refused".to_string()),
        headers: None,
        body: None,
        probed_by: "peer123".to_string(),
        timestamp: 1234567890,
    };

    assert_eq!(probe_response.status, None);
    assert_eq!(probe_response.duration, 0);
    assert_eq!(probe_response.error, Some("Connection refused".to_string()));
    assert_eq!(probe_response.headers, None);
    assert_eq!(probe_response.probed_by, "peer123");
    assert_eq!(probe_response.timestamp, 1234567890);
}
