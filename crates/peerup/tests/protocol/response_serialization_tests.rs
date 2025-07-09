//! Advanced tests for ProbeResponse serialization and status codes

use peerup::protocol::ProbeResponse;
use serde_json;

#[test]
fn test_probe_response_serialization() {
    let probe_response = ProbeResponse {
        status: Some(201,),
        duration: 300,
        error: None,
        headers: Some(vec![("Server".to_string(), "nginx/1.18.0".to_string(),)],),
        probed_by: "peer456".to_string(),
        timestamp: 1234567890,
        body: None,
    };

    let serialized = serde_json::to_string(&probe_response,).unwrap();
    let deserialized: ProbeResponse = serde_json::from_str(&serialized,).unwrap();

    assert_eq!(probe_response.status, deserialized.status);
    assert_eq!(probe_response.duration, deserialized.duration);
    assert_eq!(probe_response.error, deserialized.error);
    assert_eq!(probe_response.headers, deserialized.headers);
    assert_eq!(probe_response.probed_by, deserialized.probed_by);
    assert_eq!(probe_response.timestamp, deserialized.timestamp);
}

#[test]
fn test_probe_response_statuss() {
    let statuss = vec![200, 201, 400, 401, 403, 404, 500, 502, 503];

    for code in statuss {
        let probe_response = ProbeResponse {
            status: Some(code,),
            duration: 100,
            error: None,
            headers: None,
            probed_by: "peer456".to_string(),
            timestamp: 1234567890,
            body: None,
        };

        assert_eq!(probe_response.status, Some(code));
    }
}

#[test]
fn test_probe_response_durations() {
    let durations = vec![10, 100, 1000, 5000];

    for duration in durations {
        let probe_response = ProbeResponse {
            status: Some(200,),
            duration,
            error: None,
            headers: None,
            probed_by: "peer456".to_string(),
            timestamp: 1234567890,
            body: None,
        };
        assert_eq!(probe_response.duration, duration);
    }
}
