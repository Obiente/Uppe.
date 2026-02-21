use futures::StreamExt;
use peerup::{PeerNode, node::NodeConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};

use super::messages::{P2PCommand, P2PEvent, PeerResult, SignedMessage};
use crate::monitoring::types::CheckResult;

/// Pending DHT query response channel
type DHTResponseChannel = oneshot::Sender<Option<Vec<u8>>>;

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
    /// Pending DHT query responses: key -> response channel
    pending_dht_queries: Arc<RwLock<HashMap<String, DHTResponseChannel>>>,
}

impl P2PNetwork {
    /// Create a new P2P network manager
    #[allow(dead_code)] // Public API
    pub fn new(peer_id: String, enabled: bool) -> Self {
        // Create default config for PeerUP node
        let config = NodeConfig::builder()
            .port_range((9000, 9010))
            .enable_mdns()
            .enable_kademlia()
            .disable_relay()
            .build();

        Self {
            peer_id,
            enabled,
            public_key: None,
            config,
            command_tx: None,
            event_rx: None,
            pending_dht_queries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new P2P network manager with custom config
    pub fn with_config(
        peer_id: String,
        enabled: bool,
        public_key: [u8; 32],
        config: NodeConfig,
    ) -> Self {
        Self {
            peer_id,
            enabled,
            public_key: Some(public_key),
            config,
            command_tx: None,
            event_rx: None,
            pending_dht_queries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize and join the P2P network
    /// 
    /// Returns the event receiver for the orchestrator to use
    pub async fn start(&mut self) -> anyhow::Result<mpsc::Receiver<P2PEvent>> {
        if !self.enabled {
            tracing::info!("P2P network is disabled");
            // Return a dummy receiver that will never receive
            let (_tx, rx) = mpsc::channel::<P2PEvent>(1);
            return Ok(rx);
        }

        tracing::info!("Starting P2P network with peer ID: {}", self.peer_id);

        // Create channels for communication
        let (command_tx, mut command_rx) = mpsc::channel::<P2PCommand>(100);
        let (event_tx, event_rx) = mpsc::channel::<P2PEvent>(100);

        // Store the command sender (receiver is returned to caller)
        self.command_tx = Some(command_tx);

        // Capture public key for the task
        let public_key = self.public_key;

        // Initialize PeerUP node
        let mut node = PeerNode::with_config(self.config.clone()).await?;
        let libp2p_peer_id = node.peer_id();
        let libp2p_peer_id_str = libp2p_peer_id.to_string();

        // Start listening on configured addresses
        node.start_listening()?;

        tracing::info!("PeerUP node started with libp2p peer ID: {}", libp2p_peer_id);
        tracing::info!("Listening on: {:?}", node.listeners());

        // Dial bootstrap peers if configured
        // For local/LAN deployment, mDNS will handle peer discovery automatically
        {
            let bootstrap_peers = node.config().bootstrap_peers.clone();
            if !bootstrap_peers.is_empty() {
                tracing::info!(
                    "Dialing {} configured bootstrap peer(s) for network join",
                    bootstrap_peers.len()
                );
                node.dial_bootstrap_peers(&bootstrap_peers)?;
            } else {
                tracing::info!(
                    "No bootstrap peers configured - relying on mDNS (LAN) and Kademlia (WAN) for \
                     peer discovery"
                );
            }
        }

        // Subscribe to monitoring results topic (public monitors)
        node.subscribe_to_results()?;
        
        // Subscribe to helper assignments topic (for receiving helper requests)
        use crate::p2p::topics::UppeTopic;
        let helper_topic = UppeTopic::helper_assignments();
        node.subscribe_to_topic(&helper_topic)?;

        // Send started event
        tracing::info!(peer_id = %libp2p_peer_id, "P2P network started, emitting Started event");
        let _ = event_tx.send(P2PEvent::Started { peer_id: libp2p_peer_id.to_string() }).await;

        // Capture application peer ID for helper assignment filtering
        let app_peer_id = self.peer_id.clone();
        tracing::info!(
            "Helper assignment filter IDs: app_peer_id={}, libp2p_peer_id={}",
            app_peer_id,
            libp2p_peer_id_str
        );

        // Spawn background task to run the node's event loop
        let pending_dht_queries_clone = Arc::clone(&self.pending_dht_queries);
        
        tokio::task::spawn_local(async move {
            tracing::info!("P2P event loop started");
            let mut last_dht_snapshot = std::time::Instant::now();
            let dht_snapshot_interval = std::time::Duration::from_secs(30);
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
                            P2PCommand::QueryResults(request) => {
                                // Query peer results from database and send to requesting peer
                                tracing::debug!("Received QueryResults request: since={}, limit={}", 
                                    request.since_timestamp, request.limit);

                                // This would be implemented by:
                                // 1. Query database for peer results matching criteria
                                // 2. Serialize results into ResultsQueryResponse
                                // 3. Send response back to requesting peer via request-response protocol
                                
                                // For now, log the request (full implementation requires request-response protocol)
                                tracing::info!(
                                    "QueryResults: since={}, monitor={:?}, limit={}",
                                    request.since_timestamp,
                                    request.monitor_uuid,
                                    request.limit
                                );
                            }
                            P2PCommand::NotifySyncComplete(notification) => {
                                // Notify peer that we've synced their data (for cleanup)
                                tracing::debug!(
                                    "Sending sync completion notification to peer {} for {} results",
                                    notification.syncing_peer_id,
                                    notification.monitor_uuids.len()
                                );

                                // This would be implemented by:
                                // 1. Serialize notification into message
                                // 2. Send to peer via request-response or gossipsub
                                // 3. Peer can then mark results as synced and clean them up

                                // For now, log the notification (full implementation requires messaging protocol)
                                tracing::info!(
                                    "NotifySyncComplete: peer={}, until={}, monitors={}",
                                    notification.syncing_peer_id,
                                    notification.synced_until_timestamp,
                                    notification.monitor_uuids.len()
                                );
                            }
                            P2PCommand::AssignHelper { helper_peer_id, request } => {
                                // Publish helper assignment to GossipSub topic
                                // All peers receive, but only the target helper processes it
                                use crate::p2p::topics::UppeTopic;
                                let topic = UppeTopic::helper_assignments();
                                
                                tracing::info!(
                                    "Publishing helper assignment to topic {} for peer {} (monitor {})",
                                    topic,
                                    helper_peer_id,
                                    request.monitor_uuid
                                );
                                
                                // Subscribe to topic if not already subscribed
                                if let Err(e) = node.subscribe_to_topic(&topic) {
                                    tracing::warn!("Failed to subscribe to helper assignments topic: {}", e);
                                }
                                
                                // Publish assignment request to topic
                                if let Ok(json) = serde_json::to_string(&request) {
                                    match node.publish_to_topic(&topic, json.as_bytes().to_vec()) {
                                        Ok(_) => {
                                            tracing::debug!("Published helper assignment to topic {}", topic);
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to publish helper assignment: {}", e);
                                            let _ = event_tx.send(P2PEvent::Error(e.to_string())).await;
                                        }
                                    }
                                }
                            }
                            P2PCommand::SendHelperResponse(response) => {
                                // Publish helper assignment response to GossipSub topic
                                use crate::p2p::topics::UppeTopic;
                                let topic = UppeTopic::helper_assignments();

                                if let Ok(json) = serde_json::to_string(&response) {
                                    match node.publish_to_topic(&topic, json.as_bytes().to_vec()) {
                                        Ok(_) => {
                                            tracing::debug!("Published helper response to topic {}", topic);
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to publish helper response: {}", e);
                                            let _ = event_tx.send(P2PEvent::Error(e.to_string())).await;
                                        }
                                    }
                                }
                            }
                            P2PCommand::PublishDHTRecord { key, value } => {
                                // Use peerup's DHT API
                                let key_str = String::from_utf8_lossy(&key);
                                match node.dht_put_record_simple(&key_str, value) {
                                    Ok(_) => {
                                        tracing::debug!("DHT record published: key={}", key_str);
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to publish DHT record: {}", e);
                                        let _ = event_tx.send(P2PEvent::Error(e.to_string())).await;
                                    }
                                }
                            }
                            P2PCommand::GetDHTRecord { key } => {
                                // Use peerup's DHT API
                                let key_str = String::from_utf8_lossy(&key);
                                match node.dht_get_record_simple(&key_str) {
                                    Ok(_) => {
                                        tracing::debug!("DHT record retrieved: key={}", key_str);
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to get DHT record: {}", e);
                                        let _ = event_tx.send(P2PEvent::Error(e.to_string())).await;
                                    }
                                }
                            }
                            P2PCommand::PublishToTopic { topic, data } => {
                                // Subscribe to topic if not already subscribed
                                if let Err(e) = node.subscribe_to_topic(&topic) {
                                    tracing::warn!("Failed to subscribe to topic {}: {}", topic, e);
                                }
                                match node.publish_to_topic(&topic, data) {
                                    Ok(_) => {
                                        tracing::debug!("Published message to topic {}", topic);
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to publish to topic {}: {}", topic, e);
                                        let _ = event_tx.send(P2PEvent::Error(e.to_string())).await;
                                    }
                                }
                            }
                            P2PCommand::PublishEncryptedResult(encrypted_result) => {
                                // Publish encrypted result to private topic
                                use crate::p2p::topics::UppeTopic;
                                let topic = UppeTopic::private_results(&encrypted_result.owner_peer_id);
                                
                                if let Ok(json) = serde_json::to_string(&encrypted_result) {
                                    tracing::debug!(
                                        "Publishing encrypted result to private topic {} for owner {}",
                                        topic,
                                        encrypted_result.owner_peer_id
                                    );
                                    
                                    // Subscribe to topic if not already subscribed
                                    if let Err(e) = node.subscribe_to_topic(&topic) {
                                        tracing::warn!("Failed to subscribe to private topic {}: {}", topic, e);
                                    }
                                    
                                    // Publish to private topic
                                    match node.publish_to_topic(&topic, json.as_bytes().to_vec()) {
                                        Ok(_) => {
                                            tracing::debug!("Published encrypted result to private topic {}", topic);
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to publish encrypted result to topic {}: {}", topic, e);
                                            let _ = event_tx.send(P2PEvent::Error(e.to_string())).await;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Handle events from the swarm
                    event = node.swarm.select_next_some() => {
                        use peerup::{swarm::SwarmEvent, PeerUPEvent};

                        match event {
                            SwarmEvent::Behaviour(PeerUPEvent::GossipsubMessage { peer, message, .. }) => {
                                // Check topic to determine message type
                                let topic_str = message.topic.to_string();
                                
                                // Handle helper assignment requests
                                if topic_str == "/uppe/helper-assignments/v1" {
                                    tracing::info!("Received helper assignment message on topic {}", topic_str);
                                    if let Ok(msg_str) = String::from_utf8(message.data.clone()) {
                                        if let Ok(request) = serde_json::from_str::<crate::p2p::messages::HelperAssignmentRequest>(&msg_str) {
                                            tracing::debug!(
                                                "Parsed helper assignment for peer {} (monitor {}). Our app ID: {}, libp2p ID: {}",
                                                request.helper_peer_id,
                                                request.monitor_uuid,
                                                app_peer_id,
                                                libp2p_peer_id_str
                                            );
                                            
                                            // Accept if the assignment matches either our application peer ID or our libp2p peer ID.
                                            let is_for_us = request.helper_peer_id == app_peer_id
                                                || request.helper_peer_id == libp2p_peer_id_str;

                                            if is_for_us {
                                                tracing::info!(
                                                    helper_peer = %request.helper_peer_id,
                                                    app_peer = %app_peer_id,
                                                    libp2p_peer = %libp2p_peer_id_str,
                                                    monitor = %request.monitor_uuid,
                                                    owner = %request.owner_peer_id,
                                                    "Helper assignment targeted to this node"
                                                );
                                                let _ = event_tx.send(P2PEvent::HelperAssignmentRequested {
                                                    from_peer: peer.to_string(),
                                                    request: Box::new(request),
                                                }).await;
                                            } else {
                                                tracing::info!(
                                                    helper_peer = %request.helper_peer_id,
                                                    app_peer = %app_peer_id,
                                                    libp2p_peer = %libp2p_peer_id_str,
                                                    monitor = %request.monitor_uuid,
                                                    "Helper assignment ignored (not for this node)"
                                                );
                                            }
                                            continue; // Skip other message handling
                                        } else {
                                            tracing::warn!("Failed to parse helper assignment message");
                                        }
                                    }
                                }
                                
                                // Handle private encrypted results
                                if topic_str.starts_with("/uppe/private-results/") {
                                    if let Ok(msg_str) = String::from_utf8(message.data.clone()) {
                                        if let Ok(encrypted_result) = serde_json::from_str::<crate::crypto::EncryptedResult>(&msg_str) {
                                            let _ = event_tx.send(P2PEvent::EncryptedResultReceived {
                                                from_peer: peer.to_string(),
                                                result: Box::new(encrypted_result),
                                            }).await;
                                            continue; // Skip general result handling
                                        }
                                    }
                                }
                                
                                // Handle general monitoring results (public monitors only)
                                if let Ok(msg_str) = String::from_utf8(message.data.clone())
                                    && let Ok(signed_msg) = serde_json::from_str::<SignedMessage>(&msg_str)
                                {
                                    // Validate public key size
                                    let pubkey_bytes = signed_msg.public_key;

                                    // Extract signature; must be present
                                    let Some(signature) = signed_msg.result.signature.clone() else {
                                        tracing::warn!(
                                            target: "uppe::audit",
                                            peer = %peer,
                                            monitor = %signed_msg.result.monitor_id,
                                            "Dropped gossipsub result: missing signature"
                                        );
                                        continue;
                                    };

                                    // Verify signature before processing
                                    let is_valid = crate::crypto::verify_result(
                                        &crate::database::models::PeerResult {
                                            id: None,
                                            monitor_uuid: signed_msg.result.monitor_id,
                                            timestamp: signed_msg.result.timestamp,
                                            status: signed_msg.result.status,
                                            latency_ms: signed_msg.result.latency_ms,
                                            status_code: signed_msg.result.status_code,
                                            error_message: signed_msg.result.error_message.clone(),
                                            peer_id: signed_msg.result.peer_id.clone(),
                                            signature: signature.clone(),
                                            verified: false,
                                            created_at: std::time::SystemTime::now(),
                                            city: None,
                                            country: None,
                                            region: None,
                                            source_peer_id: Some(peer.to_string()),
                                            synced_from_peer: false,
                                            retention_until: None,
                                        },
                                        &pubkey_bytes,
                                        &signed_msg.result.target,
                                    ).unwrap_or(false);

                                    if !is_valid {
                                        tracing::warn!(
                                            target: "uppe::audit",
                                            peer = %peer,
                                            monitor = %signed_msg.result.monitor_id,
                                            "Dropped gossipsub result: signature verification failed"
                                        );
                                        continue; // Discard message with invalid signature
                                    }

                                    let peer_result = PeerResult {
                                        result: signed_msg.result.clone(),
                                        signature: Some(signature.clone()),
                                        public_key: Some(pubkey_bytes.to_vec()),
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
                            SwarmEvent::ConnectionEstablished { peer_id: peer, endpoint, .. } => {
                                // When a peer connects, add them to Kademlia routing table
                                if let Some(kademlia) = node.swarm.behaviour_mut().kademlia.as_mut() {
                                    kademlia.add_address(&peer, endpoint.get_remote_address().clone());
                                    tracing::debug!("Added connected peer {} to Kademlia routing table at {}", peer, endpoint.get_remote_address());
                                }
                                
                                let _ = event_tx.send(P2PEvent::PeerConnected(peer.to_string())).await;
                            }
                            SwarmEvent::Behaviour(PeerUPEvent::PeerDiscovered(peer)) => {
                                let _ = event_tx.send(P2PEvent::PeerConnected(peer.to_string())).await;
                            }
                            SwarmEvent::Behaviour(PeerUPEvent::PeerRemoved(peer)) |
                            SwarmEvent::ConnectionClosed { peer_id: peer, .. } => {
                                let _ = event_tx.send(P2PEvent::PeerDisconnected(peer.to_string())).await;
                            }
                            // High-level DHT events from peerup
                            SwarmEvent::Behaviour(PeerUPEvent::DhtGetRecordOk { key, record, .. }) => {
                                let key_str = String::from_utf8_lossy(&key).to_string();
                                
                                // Try to send to pending query handler first
                                let mut queries = pending_dht_queries_clone.write().await;
                                if let Some(response_tx) = queries.remove(&key_str) {
                                    let _ = response_tx.send(Some(record.clone()));
                                    tracing::debug!("DHT record received for key: {}", key_str);
                                } else {
                                    // No pending query, emit event for orchestrator
                                    let _ = event_tx
                                        .send(P2PEvent::DHTRecordReceived { key, record })
                                        .await;
                                }
                            }
                            SwarmEvent::Behaviour(PeerUPEvent::DhtGetRecordErr { key, .. }) => {
                                let key_str = String::from_utf8_lossy(&key).to_string();
                                
                                // Try to send to pending query handler first
                                let mut queries = pending_dht_queries_clone.write().await;
                                if let Some(response_tx) = queries.remove(&key_str) {
                                    let _ = response_tx.send(None);
                                    tracing::debug!("DHT record not found for key: {}", key_str);
                                } else {
                                    // No pending query, emit event for orchestrator
                                    let _ = event_tx
                                        .send(P2PEvent::DHTRecordNotFound { key })
                                        .await;
                                }
                            }
                            SwarmEvent::Behaviour(PeerUPEvent::DhtPutRecordOk { key, .. }) => {
                                let _ = event_tx
                                    .send(P2PEvent::DHTRecordPublished { key })
                                    .await;
                            }
                            SwarmEvent::Behaviour(PeerUPEvent::DhtPutRecordErr { key, error, .. }) => {
                                let _ = event_tx
                                    .send(P2PEvent::DHTRecordPublishFailed { key, error })
                                    .await;
                            }
                            _ => {
                                tracing::trace!("P2P swarm event: {:?}", event);
                            }
                        }
                    }
                    // Periodic: emit DHT snapshot for TUI/DB
                    _ = tokio::time::sleep_until(tokio::time::Instant::from_std(last_dht_snapshot + dht_snapshot_interval)) => {
                        if last_dht_snapshot.elapsed() >= dht_snapshot_interval {
                            // Build snapshot from Kademlia if present
                            if let Some(kademlia) = node.swarm.behaviour_mut().kademlia.as_mut() {
                                // Best-effort reflection of kbuckets
                                let mut buckets = Vec::new();
                                // The kbuckets() API exposes an iterator over buckets
                                for (index, bucket) in kademlia.kbuckets().enumerate() {
                                    let mut peers = Vec::new();
                                    for entry in bucket.iter() {
                                        let peer_id = entry.node.key.preimage().to_string();
                                        let addrs: Vec<String> = entry.node.value.iter().map(|a| a.to_string()).collect();
                                        peers.push(crate::p2p::messages::DhtPeerEntry {
                                            peer_id,
                                            addrs,
                                            state: None,
                                        });
                                    }
                                    // Only include non-empty buckets
                                    if !peers.is_empty() {
                                        buckets.push(crate::p2p::messages::DhtBucket { index, peers });
                                    }
                                }

                                let snapshot = crate::p2p::messages::DhtSnapshot {
                                    local_peer_id: libp2p_peer_id_str.clone(),
                                    buckets,
                                    captured_at: chrono::Utc::now().timestamp(),
                                };
                                let _ = event_tx.send(P2PEvent::DhtSnapshot { snapshot: Box::new(snapshot) }).await;
                            }
                            last_dht_snapshot = std::time::Instant::now();
                        }
                    }
                }
            }

            tracing::info!("P2P event loop stopped");
        });

        // Return the event receiver for the orchestrator
        Ok(event_rx)
    }

    /// Get a DHT record synchronously (with timeout)
    /// 
    /// This method blocks until the DHT query completes or times out.
    /// It's useful for operations that need synchronous access to DHT data.
    pub async fn get_dht_record(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
        if !self.enabled {
            return Ok(None);
        }

        if let Some(tx) = &self.command_tx {
            // Create a response channel for this query
            let (response_tx, response_rx) = oneshot::channel();
            
            // Register pending query
            self.pending_dht_queries
                .write()
                .await
                .insert(key.to_string(), response_tx);

            // Send the DHT query command
            tx.send(P2PCommand::GetDHTRecord {
                key: key.as_bytes().to_vec(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send DHT query: {}", e))?;

            // Wait for response with timeout (30 seconds)
            match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                response_rx,
            ).await {
                Ok(Ok(result)) => Ok(result),
                Ok(Err(_)) => {
                    // Channel closed, query never completed
                    self.pending_dht_queries.write().await.remove(key);
                    Ok(None)
                }
                Err(_) => {
                    // Timeout
                    self.pending_dht_queries.write().await.remove(key);
                    Err(anyhow::anyhow!("DHT query timeout for key: {}", key))
                }
            }
        } else {
            Err(anyhow::anyhow!("P2P node not started"))
        }
    }

    /// Publish a DHT record (non-blocking)
    pub async fn put_dht_record(&self, key: &str, value: Vec<u8>) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(tx) = &self.command_tx {
            tx.send(P2PCommand::PublishDHTRecord {
                key: key.as_bytes().to_vec(),
                value,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send DHT put: {}", e))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("P2P node not started"))
        }
    }

    /// Share a monitoring result with the network
    /// 
    /// **SECURITY**: This method should ONLY be called for PUBLIC monitors.
    /// Private and Internal monitors should NEVER be shared via this method.
    /// 
    /// The caller is responsible for verifying monitor visibility before calling this.
    /// This method does NOT perform visibility checks to avoid async database lookups.
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

    /// Publish an encrypted result from a helper peer back to the owner
    /// 
    /// This is called by helper peers to send encrypted monitoring results
    /// back to the owner peer who assigned them to help monitor.
    pub async fn publish_encrypted_result(&self, encrypted_result: &crate::crypto::EncryptedResult) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(tx) = &self.command_tx {
            tx.send(P2PCommand::PublishEncryptedResult(encrypted_result.clone()))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send encrypted result publish command: {}", e))?;
            tracing::debug!("Sent publish encrypted result command for monitor {}", encrypted_result.monitor_uuid);
        } else {
            tracing::warn!("P2P node not started, cannot publish encrypted result");
        }

        Ok(())
    }

    /// Get the next event from the P2P network
    pub async fn next_event(&mut self) -> Option<P2PEvent> {
        if let Some(rx) = &mut self.event_rx { rx.recv().await } else { None }
    }

    /// Send a command to the P2P node
    #[allow(dead_code)] // Public API
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
