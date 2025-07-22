//! Basic tests for node configuration

use peerup::node::{NodeConfig};
use std::time::Duration;

#[test]
fn test_peer_node_config_default() {
    let config = NodeConfig::new();
    
    assert_eq!(config.port, 0);
    assert!(config.relay_servers.is_empty());
    assert_eq!(config.bootstrap_interval, Duration::from_secs(30));
    assert_eq!(config.probe_timeout, Duration::from_secs(10));
}

#[test]
fn test_peer_node_config_with_port() {
    let config = NodeConfig::new().with_port(8080);
    
    assert_eq!(config.port, 8080);
}

#[test]
fn test_peer_node_config_with_bootstrap_interval() {
    let interval = Duration::from_secs(60);
    let config = NodeConfig::new().with_bootstrap_interval(interval);
    
    assert_eq!(config.bootstrap_interval, interval);
}

#[test]
fn test_peer_node_config_with_probe_timeout() {
    let timeout = Duration::from_secs(30);
    let config = NodeConfig::new().with_probe_timeout(timeout);
    
    assert_eq!(config.probe_timeout, timeout);
}

#[test]
fn test_peer_node_config_with_relay_servers() {
    let relay_servers = vec![
        "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN".parse().unwrap(),
    ];
    
    let config = NodeConfig::new().with_relay_servers(relay_servers.clone());
    
    assert_eq!(config.relay_servers.len(), 1);
    assert_eq!(config.relay_servers[0], relay_servers[0]);
}
