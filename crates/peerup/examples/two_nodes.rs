//! Example: Two PeerUP nodes communicating in the same process.

use std::time::Duration;

use anyhow::Result;
use peerup::{node::NodeConfig, PeerNode, ProbeRequest};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(),> {
    // Node 1 config: fixed port
    let config1 = NodeConfig::builder().port_range((4001, 4001,),).build();
    let node1 = PeerNode::with_config(config1,).await?;
    let addr1 = node1.swarm.listeners().next().cloned();

    // Node 2 config: fixed port
    let config2 = NodeConfig::builder().port_range((4002, 4002,),).build();
    let mut node2 = PeerNode::with_config(config2,).await?;
    let addr2 = node2.swarm.listeners().next().cloned();

    // Print peer IDs and addresses
    println!("Node1: {} {:?}", node1.peer_id(), addr1);
    println!("Node2: {} {:?}", node2.peer_id(), addr2);

    // Node2 dials Node1
    if let Some(addr1,) = addr1 {
        node2.swarm.dial(addr1.clone(),).expect("Dial failed",);
        println!("Node2 dialing Node1 at {addr1:?}");
    }

    // Let the nodes discover each other
    sleep(Duration::from_secs(1,),).await;

    // Example: Node2 sends a probe request to Node1 (stub, see note)
    let probe = ProbeRequest {
        target_url: "https://example.com".to_string(),
        method: "GET".to_string(),
        timeout: 5000,
        headers: None,
        body: None,
        requested_by: node2.peer_id().to_string(),
    };

    // NOTE: You need a real API to send a probe request from node2 to node1.
    // If you have a method like node2.send_probe(peer_id, probe), call it here.
    // For now, just print what would happen:
    println!("Node2 would send probe to Node1: {probe:?} -> {}", node1.peer_id());

    // Run both nodes for a short time to process events
    let _n1 = tokio::spawn(async move { node1.run().await },);
    let _n2 = tokio::spawn(async move { node2.run().await },);

    sleep(Duration::from_secs(5,),).await;

    // In a real test, you would shut down nodes gracefully
    println!("Done.");
    Ok((),)
}
