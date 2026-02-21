//! Gossipsub-related methods for PeerNode.

use anyhow::Result;
use libp2p::gossipsub::{IdentTopic, TopicHash};

use crate::node::core::peer_node::PeerNode;

/// Topic for broadcasting monitoring results
pub const MONITORING_RESULTS_TOPIC: &str = "uppe/monitoring/results/v1";

impl PeerNode {
    /// Subscribe to the monitoring results topic
    pub fn subscribe_to_results(&mut self) -> Result<()> {
        let topic = IdentTopic::new(MONITORING_RESULTS_TOPIC);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to topic: {}", e))?;
        tracing::info!("Subscribed to monitoring results topic");
        Ok(())
    }

    /// Unsubscribe from the monitoring results topic
    pub fn unsubscribe_from_results(&mut self) -> Result<()> {
        let topic = IdentTopic::new(MONITORING_RESULTS_TOPIC);
        let was_subscribed = self.swarm.behaviour_mut().gossipsub.unsubscribe(&topic);

        if was_subscribed {
            tracing::info!("Unsubscribed from monitoring results topic");
        } else {
            tracing::warn!("Was not subscribed to monitoring results topic");
        }
        Ok(())
    }

    /// Publish a monitoring result to the network
    pub fn publish_result(&mut self, result_json: String) -> Result<()> {
        let topic = IdentTopic::new(MONITORING_RESULTS_TOPIC);

        match self.swarm.behaviour_mut().gossipsub.publish(topic, result_json.as_bytes()) {
            Ok(_) => {
                tracing::debug!("Published result to gossipsub network");
                Ok(())
            }
            Err(e) => {
                // Some libp2p versions do not expose a typed InsufficientPeers variant; detect by message content.
                let msg = e.to_string();
                if msg.contains("InsufficientPeers") || msg.to_lowercase().contains("no peers") {
                    tracing::debug!("No peers connected to receive published result (normal during startup or isolation)");
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Failed to publish result: {}", e))
                }
            }
        }
    }

    /// Publish a message to a specific GossipSub topic
    pub fn publish_to_topic(&mut self, topic_name: &str, message: Vec<u8>) -> Result<()> {
        let topic = IdentTopic::new(topic_name);
        
        // Ensure we're subscribed to the topic (required for publishing)
        if !self.swarm.behaviour().gossipsub.topics().any(|t| t.to_string() == topic_name) {
            self.swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&topic)
                .map_err(|e| anyhow::anyhow!("Failed to subscribe to topic {}: {}", topic_name, e))?;
            tracing::debug!("Auto-subscribed to topic {}", topic_name);
        }

        match self.swarm.behaviour_mut().gossipsub.publish(topic, message) {
            Ok(_) => {
                tracing::debug!("Published message to topic {}", topic_name);
                Ok(())
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("InsufficientPeers") || msg.to_lowercase().contains("no peers") {
                    tracing::debug!("No peers connected to receive published message on topic {} (normal during startup or isolation)", topic_name);
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Failed to publish to topic {}: {}", topic_name, e))
                }
            }
        }
    }

    /// Subscribe to a specific GossipSub topic
    pub fn subscribe_to_topic(&mut self, topic_name: &str) -> Result<()> {
        let topic = IdentTopic::new(topic_name);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to topic {}: {}", topic_name, e))?;
        tracing::debug!("Subscribed to topic {}", topic_name);
        Ok(())
    }

    /// Get list of peers subscribed to a topic
    pub fn get_topic_peers(&self, topic: &str) -> Vec<libp2p::PeerId> {
        let topic_hash = TopicHash::from_raw(topic);
        self.swarm.behaviour().gossipsub.mesh_peers(&topic_hash).copied().collect()
    }

    /// Get all subscribed topics
    pub fn get_subscribed_topics(&self) -> Vec<String> {
        self.swarm.behaviour().gossipsub.topics().map(|t| t.to_string()).collect()
    }
}
