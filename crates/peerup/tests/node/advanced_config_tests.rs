//! Advanced tests for node configuration

use peerup::node::{NodeConfig};
use std::time::Duration;

#[test]
fn test_peer_node_config_chaining() {
    let relay_servers = vec![
        "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN".parse().unwrap(),
    ];
    
    let config = NodeConfig::new()
        .with_port(9000)
        .with_bootstrap_interval(Duration::from_secs(45))
        .with_probe_timeout(Duration::from_secs(15))
        .with_relay_servers(relay_servers.clone());
    
    assert_eq!(config.port, 9000);
    assert_eq!(config.bootstrap_interval, Duration::from_secs(45));
    assert_eq!(config.probe_timeout, Duration::from_secs(15));
    assert_eq!(config.relay_servers.len(), 1);
}

#[test]
fn test_peer_node_config_debug() {
    let config = NodeConfig::new()
        .with_port(8080);
    
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("port: 8080"));
}

#[test]
fn test_peer_node_config_clone() {
    let config = NodeConfig::new()
        .with_port(8080)
        .with_bootstrap_interval(Duration::from_secs(60));
    
    let cloned_config = config.clone();
    
    assert_eq!(config.port, cloned_config.port);
    assert_eq!(config.bootstrap_interval, cloned_config.bootstrap_interval);
}

#[test]
fn test_config_validation() {
    // Test that we can create configs with extreme values
    let config = NodeConfig::new()
        .with_port(65535)  // Max port
        .with_bootstrap_interval(Duration::from_secs(1))  // Min reasonable interval
        .with_probe_timeout(Duration::from_secs(300));  // Max reasonable timeout
    
    assert_eq!(config.port, 65535);
    assert_eq!(config.bootstrap_interval, Duration::from_secs(1));
    assert_eq!(config.probe_timeout, Duration::from_secs(300));
}

#[test]
fn test_empty_relay_servers() {
    let config = NodeConfig::new()
        .with_relay_servers(vec![]);
    
    assert!(config.relay_servers.is_empty());
}

#[test]
fn test_multiple_relay_servers() {
    let relay_servers = vec![
        "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN".parse().unwrap(),
        "/ip4/127.0.0.1/tcp/4002/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTO".parse().unwrap(),
    ];
    
    let config = NodeConfig::new()
        .with_relay_servers(relay_servers.clone());
    
    assert_eq!(config.relay_servers.len(), 2);
    assert_eq!(config.relay_servers, relay_servers);
}
