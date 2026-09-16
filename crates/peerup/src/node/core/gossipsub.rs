//! Generic GossipSub helpers for PeerNode.

use anyhow::Result;
use libp2p::gossipsub::{IdentTopic, TopicHash};

use crate::node::core::peer_node::PeerNode;

impl PeerNode {
    /// Publish a message to a specific GossipSub topic.
    pub fn publish_to_topic(&mut self, topic_name: &str, message: Vec<u8>) -> Result<()> {
        let topic = IdentTopic::new(topic_name);
        if !self.swarm.behaviour().gossipsub.topics().any(|t| t.to_string() == topic_name) {
            self.swarm.behaviour_mut().gossipsub.subscribe(&topic).map_err(|e| {
                anyhow::anyhow!("Failed to subscribe to topic {}: {}", topic_name, e)
            })?;
            tracing::debug!("Auto-subscribed to topic {}", topic_name);
        }
        match self.swarm.behaviour_mut().gossipsub.publish(topic, message) {
            Ok(_) => {
                tracing::debug!("Published message to topic {}", topic_name);
                Ok(())
            }
            Err(e) => {
                let message = e.to_string();
                if message.contains("InsufficientPeers")
                    || message.to_lowercase().contains("no peers")
                {
                    tracing::debug!("No peers connected for topic {}", topic_name);
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Failed to publish to topic {}: {}", topic_name, e))
                }
            }
        }
    }

    /// Subscribe to a specific GossipSub topic.
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

    /// Unsubscribe from a specific GossipSub topic.
    pub fn unsubscribe_from_topic(&mut self, topic_name: &str) -> Result<()> {
        let topic = IdentTopic::new(topic_name);
        self.swarm.behaviour_mut().gossipsub.unsubscribe(&topic);
        Ok(())
    }

    /// Get peers subscribed to a topic.
    pub fn get_topic_peers(&self, topic: &str) -> Vec<libp2p::PeerId> {
        let topic_hash = TopicHash::from_raw(topic);
        self.swarm.behaviour().gossipsub.mesh_peers(&topic_hash).copied().collect()
    }

    /// Get all subscribed topics.
    pub fn get_subscribed_topics(&self) -> Vec<String> {
        self.swarm.behaviour().gossipsub.topics().map(|t| t.to_string()).collect()
    }
}
