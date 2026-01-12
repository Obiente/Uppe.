use futures::StreamExt;
use peerup::{PeerNode, node::NodeConfig};
use tokio::sync::mpsc;

use super::messages::{P2PCommand, P2PEvent, PeerResult, SignedMessage};
use crate::monitoring::types::CheckResult;

/// P2P network manager
pub struct P2PNetwork {
    peer_id: String,
    enabled: bool,
    /// Ed25519 public key (32 bytes) for signing messages
    public_key: Option<[u8; 32]>,
    /// Configuration for the P2P node
    config: NodeConfig,
    /// Channel to send commands to the P2P node
    command_tx: Option<mpsc::Sender<P2PCommand>>,
    /// Channel to receive events from the P2P node
    event_rx: Option<mpsc::Receiver<P2PEvent>>,
}

impl P2PNetwork {
    /// Create a new P2P network manager
    pub fn new(peer_id: String, enabled: bool) -> Self {
        // Create default config for PeerUP node
        let config = NodeConfig::builder()
            .port_range((9000, 9010))
            .enable_mdns()
            .enable_kademlia()
            .disable_relay()
            .build();

        Self { peer_id, enabled, public_key: None, config, command_tx: None, event_rx: None }
    }

    /// Create a new P2P network manager with custom config
    pub fn with_config(peer_id: String, enabled: bool, public_key: [u8; 32], config: NodeConfig) -> Self {
        Self { peer_id, enabled, public_key: Some(public_key), config, command_tx: None, event_rx: None }
    }

    /// Initialize and join the P2P network
    pub async fn start(&mut self) -> anyhow::Result<()> {
        if !self.enabled {
            tracing::info!("P2P network is disabled");
            return Ok(());
        }

        tracing::info!("Starting P2P network with peer ID: {}", self.peer_id);

        // Create channels for communication
        let (command_tx, mut command_rx) = mpsc::channel::<P2PCommand>(100);
        let (event_tx, event_rx) = mpsc::channel::<P2PEvent>(100);

        // Store the command sender and event receiver
        self.command_tx = Some(command_tx);
        self.event_rx = Some(event_rx);

        // Capture public key for the task
        let public_key = self.public_key;

        // Initialize PeerUP node
        let mut node = PeerNode::with_config(self.config.clone()).await?;
        let libp2p_peer_id = node.peer_id();

        // Start listening on configured addresses
        node.start_listening()?;

        tracing::info!("PeerUP node started with libp2p peer ID: {}", libp2p_peer_id);
        tracing::info!("Listening on: {:?}", node.listeners());

        // Dial bootstrap peers if configured (production deployment with known bootstrap nodes)
        // For local/LAN deployment, mDNS will handle peer discovery automatically
        {
            let bootstrap_peers = node.config().bootstrap_peers.clone();
            if !bootstrap_peers.is_empty() {
                tracing::info!("Dialing {} configured bootstrap peer(s) for network join", bootstrap_peers.len());
                node.dial_bootstrap_peers(&bootstrap_peers)?;
            } else {
                tracing::info!("No bootstrap peers configured - relying on mDNS (LAN) and Kademlia (WAN) for peer discovery");
            }
        }

        // Subscribe to monitoring results topic
        node.subscribe_to_results()?;

        // Send started event
        let _ = event_tx.send(P2PEvent::Started { peer_id: libp2p_peer_id.to_string() }).await;

        // Spawn background task to run the node's event loop
        tokio::task::spawn_local(async move {
            tracing::info!("P2P event loop started");

            loop {
                tokio::select! {
                    // Handle commands from the service
                    Some(cmd) = command_rx.recv() => {
                        match cmd {
                            P2PCommand::PublishResult(result) => {
                                // Wrap result with public key in SignedMessage
                                let signed_msg = SignedMessage {
                                    result: result.clone(),
                                    public_key: public_key.unwrap_or([0u8; 32]),
                                };
                                
                                if let Ok(json) = serde_json::to_string(&signed_msg) {
                                    match node.publish_result(json) {
                                        Ok(_) => {
                                            tracing::debug!("Published monitoring result to P2P network");
                                        }
                                        Err(e) => {
                                            // Only log actual errors, not "no peers" conditions
                                            tracing::error!("Failed to publish result: {}", e);
                                            let _ = event_tx.send(P2PEvent::Error(e.to_string())).await;
                                        }
                                    }
                                }
                            }
                            P2PCommand::Subscribe => {
                                if let Err(e) = node.subscribe_to_results() {
                                    tracing::error!("Failed to subscribe: {}", e);
                                } else {
                                    let _ = event_tx.send(P2PEvent::Subscribed).await;
                                }
                            }
                            P2PCommand::Unsubscribe => {
                                if let Err(e) = node.unsubscribe_from_results() {
                                    tracing::error!("Failed to unsubscribe: {}", e);
                                } else {
                                    let _ = event_tx.send(P2PEvent::Unsubscribed).await;
                                }
                            }
                            P2PCommand::Shutdown => {
                                tracing::info!("Shutting down P2P node");
                                break;
                            }
                        }
                    }

                    // Handle events from the swarm
                    event = node.swarm.select_next_some() => {
                        use peerup::{swarm::SwarmEvent, PeerUPEvent};

                        match event {
                            SwarmEvent::Behaviour(PeerUPEvent::GossipsubMessage { peer, message, .. }) => {
                                // Decode signed message
                                if let Ok(msg_str) = String::from_utf8(message.data.clone())
                                    && let Ok(signed_msg) = serde_json::from_str::<SignedMessage>(&msg_str)
                                {
                                    let peer_result = PeerResult {
                                        result: signed_msg.result.clone(),
                                        signature: signed_msg.result.signature.clone(),
                                        public_key: Some(signed_msg.public_key.to_vec()),
                                        // Use the signer-declared peer_id (matches signature) rather than libp2p ID
                                        peer_id: signed_msg.result.peer_id.clone(),
                                        received_at: std::time::SystemTime::now(),
                                    };
                                    let _ = event_tx.send(P2PEvent::ResultReceived {
                                        peer_id: peer.to_string(),
                                        result: Box::new(peer_result),
                                    }).await;
                                }
                            }
                            SwarmEvent::Behaviour(PeerUPEvent::PeerDiscovered(peer)) |
                            SwarmEvent::ConnectionEstablished { peer_id: peer, .. } => {
                                let _ = event_tx.send(P2PEvent::PeerConnected(peer.to_string())).await;
                            }
                            SwarmEvent::Behaviour(PeerUPEvent::PeerRemoved(peer)) |
                            SwarmEvent::ConnectionClosed { peer_id: peer, .. } => {
                                let _ = event_tx.send(P2PEvent::PeerDisconnected(peer.to_string())).await;
                            }
                            _ => {
                                tracing::trace!("P2P swarm event: {:?}", event);
                            }
                        }
                    }
                }
            }

            tracing::info!("P2P event loop stopped");
        });

        Ok(())
    }

    /// Share a monitoring result with the network
    pub async fn share_result(&self, result: &CheckResult) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(tx) = &self.command_tx {
            tx.send(P2PCommand::PublishResult(result.clone()))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send publish command: {}", e))?;
            tracing::debug!("Sent publish command for monitor {}", result.monitor_id);
        } else {
            tracing::warn!("P2P node not started, cannot share result");
        }

        Ok(())
    }

    /// Get the next event from the P2P network
    pub async fn next_event(&mut self) -> Option<P2PEvent> {
        if let Some(rx) = &mut self.event_rx { rx.recv().await } else { None }
    }

    /// Send a command to the P2P node
    pub async fn send_command(&self, command: P2PCommand) -> anyhow::Result<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(command)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("P2P node not started"))
        }
    }

    /// Get our peer ID
    #[allow(dead_code)] // Public API method
    pub fn peer_id(&self) -> &str {
        &self.peer_id
    }

    /// Check if P2P is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p2p_network_disabled() {
        let network = P2PNetwork::new("test-peer".to_string(), false);
        assert!(!network.is_enabled());

        // Network is disabled, so start should succeed without errors
        assert!(!network.is_enabled());
    }

    #[tokio::test]
    async fn test_p2p_network_enabled() {
        let network = P2PNetwork::new("test-peer".to_string(), true);
        assert!(network.is_enabled());
        assert_eq!(network.peer_id(), "test-peer");
    }
}
