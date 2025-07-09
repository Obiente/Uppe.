//! Simple example node for testing PeerUP functionality.
//!
//! This example demonstrates how to create and run a basic PeerUP node.

use anyhow::Result;
use peerup::{PeerNode, NodeConfig};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting PeerUP example node...");

    // Create a configuration
    let config = NodeConfig::default()
        .with_mdns(true)
        .with_kademlia(true)
        .with_relay(false); // Disable relay for this simple example

    // Create the node
    let node = PeerNode::with_config(config).await?;
    
    info!("Node created with peer ID: {}", node.peer_id());
    info!("Node configuration: {:?}", node.config());

    info!("Example completed successfully!");
    Ok(())
}
