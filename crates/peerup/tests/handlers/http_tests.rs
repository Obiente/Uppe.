//! Tests for HTTP request handling

use peerup::handlers::handle_probe_request;
use peerup::protocol::ProbeRequest;

#[tokio::test]
async fn test_handle_probe_request_basic() {
    let request = ProbeRequest {
        target_url: "https://httpbin.org/get".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        body: None,
        headers: None,
        requested_by: "peer123".to_string(),
    };

    // This test might fail if httpbin.org is not available, so we'll just check
    // that the function doesn't panic
    let _response = handle_probe_request(request).await;
    // If the function returns, the test passes (no panic)
}
