//! Tests for response structure and validation

use peerup::protocol::ProbeResponse;

#[test]
fn test_probe_response_fields() {
    let response = ProbeResponse {
        status: Some(200,),
        duration: 150,
        error: None,
        probed_by: "peer456".to_string(),
        timestamp: 1234567890,
        headers: Some(vec![("Content-Type".to_string(), "application/json".to_string(),)],),
        body: None,
    };

    assert_eq!(response.status, Some(200));
    assert_eq!(response.duration, 150);
    assert_eq!(response.error, None);
    assert_eq!(response.probed_by, "peer456");
    assert_eq!(response.timestamp, 1234567890);
    assert_eq!(response.headers.as_ref().unwrap().len(), 1);
}

#[test]
fn test_probe_response_error() {
    let response = ProbeResponse {
        status: None,
        duration: 0,
        error: Some("Connection timeout".to_string(),),
        probed_by: "peer456".to_string(),
        timestamp: 1234567890,
        headers: None,
        body: None,
    };

    assert_eq!(response.status, None);
    assert_eq!(response.duration, 0);
    assert_eq!(response.error, Some("Connection timeout".to_string()));
    assert_eq!(response.probed_by, "peer456");
    assert_eq!(response.timestamp, 1234567890);
    assert_eq!(response.headers, None);
}
