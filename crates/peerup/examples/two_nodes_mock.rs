//! Example: Two PeerUP nodes communicating with mock data in the same process.

use std::time::Duration;

use anyhow::Result;
use peerup::{node::NodeConfig, PeerNode, ProbeRequest, ProbeResponse};
use tokio::{sync::mpsc, time::sleep};

#[tokio::main]
async fn main() -> Result<(),> {
    // Channel to simulate probe requests from node2 to node1
    let (probe_tx, mut probe_rx,) = mpsc::channel::<ProbeRequest,>(1,);
    // Channel to simulate probe responses from node1 to node2
    let (resp_tx, mut resp_rx,) = mpsc::channel::<ProbeResponse,>(1,);

    // Node 1 task: receives probe, sends response
    let node1_task = tokio::spawn(async move {
        let config1 = NodeConfig::builder().port_range((4001, 4001,),).build();
        let node1 = PeerNode::with_config(config1,).await.unwrap();
        println!("Node1: {}", node1.peer_id());

        // Simulate event loop
        loop {
            tokio::select! {
                Some(probe) = probe_rx.recv() => {
                    println!("Node1 received probe: {probe:?}");
                    // Simulate processing and send a response
                    let response = ProbeResponse {
                        status: Some(200),
                        duration: 123,
                        error: None,
                        headers: None,
                        body: Some("OK".to_string()),
                        probed_by: node1.peer_id().to_string(),
                        timestamp: 1234567890,
                    };
                    resp_tx.send(response).await.unwrap();
                }
                // In a real node, you would also poll node1.run().await here
                _ = sleep(Duration::from_secs(5)) => {
                    break;
                }
            }
        }
    },);

    // Node 2 task: sends probe, receives response
    let node2_task = tokio::spawn(async move {
        let config2 = NodeConfig::builder().port_range((4002, 4002,),).build();
        let node2 = PeerNode::with_config(config2,).await.unwrap();
        println!("Node2: {}", node2.peer_id());

        // Simulate sending a probe request to node1
        let probe = ProbeRequest {
            target_url: "https://example.com".to_string(),
            method: "GET".to_string(),
            timeout: 5000,
            headers: None,
            body: None,
            requested_by: node2.peer_id().to_string(),
        };
        println!("Node2 sending probe to Node1...");
        probe_tx.send(probe,).await.unwrap();

        // Wait for response
        if let Some(response,) = resp_rx.recv().await {
            println!("Node2 received response: {response:?}");
        }
    },);

    // Wait for both tasks to finish
    let _ = tokio::join!(node1_task, node2_task);

    println!("Done.");
    Ok((),)
}
