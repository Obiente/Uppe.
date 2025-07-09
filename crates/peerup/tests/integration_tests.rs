//! Integration tests for PeerUP
//!
//! These tests verify that the modular components work together correctly.

use peerup::{PeerNode, NodeConfig, ProbeRequest, ProbeResponse};
// ...existing code...
use tracing_subscriber;

#[tokio::test]
async fn test_peer_node_creation() {
    let _ = tracing_subscriber::fmt::try_init();

    let result = PeerNode::new().await;
    assert!(result.is_ok(), "Failed to create PeerNode: {:?}", result.err());
}

#[tokio::test]
async fn test_peer_node_with_config() {
    let _ = tracing_subscriber::fmt::try_init();

    let config = NodeConfig::builder().port_range((0, 0)).build(); // Use random port for testing

    let result = PeerNode::with_config(config).await;
    assert!(result.is_ok(), "Failed to create PeerNode with config: {:?}", result.err());
}

#[tokio::test]
async fn test_probe_request_serialization() {
    let probe_request = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        headers: None,
        body: None,
        requested_by: "peer123".to_string(),
    };

    let serialized = serde_json::to_string(&probe_request).unwrap();
    let deserialized: ProbeRequest = serde_json::from_str(&serialized).unwrap();

    assert_eq!(probe_request.target_url, deserialized.target_url);
    assert_eq!(probe_request.method, deserialized.method);
}

#[tokio::test]
async fn test_probe_response_serialization() {
    let probe_response = ProbeResponse {
        status: Some(200),
        duration: 150,
        error: None,
        headers: None,
        body: Some("OK".to_string()),
        probed_by: "peer123".to_string(),
        timestamp: 1234567890,
    };

    let serialized = serde_json::to_string(&probe_response).unwrap();
    let deserialized: ProbeResponse = serde_json::from_str(&serialized).unwrap();

    assert_eq!(probe_response.status, deserialized.status);
    assert_eq!(probe_response.duration, deserialized.duration);
    assert_eq!(probe_response.body, deserialized.body);
}

use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_node_lifecycle() {
    let _ = tracing_subscriber::fmt::try_init();

    let config = NodeConfig::builder().port_range((0, 0)).build(); // Use random port for testing
    let node = PeerNode::with_config(config).await.unwrap();

    // Run the node for a short time to ensure it starts and can be stopped
    let run_future = node.run();
    let result = timeout(Duration::from_secs(2), run_future).await;
    // The node should either run successfully or timeout (which is expected for this test)
    match result {
        Ok(Ok(())) => {
            // Node completed successfully
        },
        Ok(Err(e)) => {
            panic!("Node run returned error: {:?}", e);
        },
        Err(_) => {
            // Timeout occurred, which is expected for a long-running node
        },
    }
}

#[tokio::test]
async fn test_multiple_nodes() {
    let _ = tracing_subscriber::fmt::try_init();

    let config1 = NodeConfig::builder().port_range((0, 0)).build();
    let config2 = NodeConfig::builder().port_range((0, 0)).build();

    let node1 = PeerNode::with_config(config1).await;
    let node2 = PeerNode::with_config(config2).await;

    assert!(node1.is_ok(), "Failed to create first node");
    assert!(node2.is_ok(), "Failed to create second node");
}
