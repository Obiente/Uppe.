//! PeerNode run loop for driving the swarm and handling events.

use anyhow::Result;
use futures::StreamExt;
use tokio::signal;
use tracing::info;

use super::peer_node::PeerNode;

impl PeerNode {
    /// Run the node event loop, polling the swarm and handling events.
    /// This will block until Ctrl+C is pressed or an error occurs.
    pub async fn run(mut self,) -> Result<(),> {
        info!("PeerNode event loop started. Press Ctrl+C to exit.");
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    info!("Swarm event: {:?}", event);
                    // In the future, handle events more granularly here
                }
                _ = signal::ctrl_c() => {
                    info!("Ctrl+C received, shutting down node.");
                    break;
                }
            }
        }
        Ok((),)
    }
}
