//! A simple PeerUP node example.
//!
//! This example demonstrates how to create and run a basic PeerUP node.

use anyhow::Result;
use peerup::{PeerNode, node::NodeConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    // Create a node configuration
    let mut config = NodeConfig::default();
    
    // Allow overriding the port range via environment variables
    let min_port = env::var("PEERUP_MIN_PORT").ok().and_then(|s| s.parse::<u16>().ok());
    let max_port = env::var("PEERUP_MAX_PORT").ok().and_then(|s| s.parse::<u16>().ok());
    if let (Some(min_port), Some(max_port)) = (min_port, max_port) {
        config.port_range = (min_port, max_port);
    }
    
    // Allow specifying a keypair path
    if let Ok(keypair_path) = env::var("PEERUP_KEYPAIR_PATH") {
        config.keypair_path = Some(keypair_path);
    }
    
    // Allow specifying bootstrap peers
    if let Ok(bootstrap_peers) = env::var("PEERUP_BOOTSTRAP_PEERS") {
        config.bootstrap_peers = bootstrap_peers.split(',').map(String::from).collect();
    }
    
    println!("Starting PeerUP node with configuration:");
    println!("  Port range: {:?}", config.port_range);
    println!("  Keypair path: {:?}", config.keypair_path);
    println!("  Bootstrap peers: {:?}", config.bootstrap_peers);
    println!("  mDNS enabled: {}", config.enable_mdns);
    println!("  Kademlia enabled: {}", config.enable_kademlia);
    println!("  Relay enabled: {}", config.enable_relay);
    
    // Create and start the node
    let node = PeerNode::with_config(config).await?;

    println!("Node started with peer ID: {}", node.peer_id());
    println!("Press Ctrl+C to exit.");

    // Run the node until interrupted
    if let Err(e) = node.run().await {
        eprintln!("Node exited with error: {e}");
    }
    Ok(())
}
