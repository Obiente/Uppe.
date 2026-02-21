//! GossipSub topic management for distributed monitoring.
//!
//! This module defines topic naming conventions and subscription management.

// Note: libp2p types accessed via peerup's swarm
// For now, we define topic strings and let the network layer handle conversion

/// GossipSub topics for Uppe monitoring
///
/// Topic strings that will be converted to IdentTopic by the network layer
pub struct UppeTopic;

impl UppeTopic {
    /// Topic for public monitor consensus (proposals, votes, admissions)
    pub fn public_monitors() -> String {
        "/uppe/public-monitors/v1".to_string()
    }

    /// Topic for orchestration votes (which peer checks which monitor)
    pub fn orchestration() -> String {
        "/uppe/orchestration/v1".to_string()
    }

    /// Topic for encrypted private results (owner-specific)
    ///
    /// Each owner has their own topic: /uppe/private-results/{owner_peer_id}/v1
    /// Only the owner and their assigned helper peers subscribe to this topic
    pub fn private_results(owner_peer_id: &str) -> String {
        format!("/uppe/private-results/{}/v1", owner_peer_id)
    }

    /// Topic for sync completion notifications
    ///
    /// Owner broadcasts when they've synced, helpers can clean up
    pub fn sync_completion(owner_peer_id: &str) -> String {
        format!("/uppe/sync-completion/{}/v1", owner_peer_id)
    }

    /// Topic for DHT replication coordination
    ///
    /// Peers coordinate DHT record publishing to ensure 20-replica redundancy
    pub fn dht_replication() -> String {
        "/uppe/dht-replication/v1".to_string()
    }

    /// Topic for peer reputation updates
    ///
    /// Peers share reputation scores for helper peer selection
    pub fn reputation() -> String {
        "/uppe/reputation/v1".to_string()
    }

    /// Topic for helper assignments (for private monitor orchestration)
    ///
    /// Owners publish helper peer assignments for monitors
    pub fn helper_assignments() -> String {
        "/uppe/helper-assignments/v1".to_string()
    }
}

/// Topic subscription manager
pub struct TopicManager {
    /// Currently subscribed topics
    subscribed: std::sync::Arc<tokio::sync::RwLock<Vec<String>>>,
}

impl TopicManager {
    pub fn new() -> Self {
        Self {
            subscribed: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: String) {
        let mut subscribed = self.subscribed.write().await;
        if !subscribed.contains(&topic) {
            subscribed.push(topic);
        }
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe(&self, topic: &str) {
        let mut subscribed = self.subscribed.write().await;
        subscribed.retain(|t| t != topic);
    }

    /// Check if subscribed to a topic
    pub async fn is_subscribed(&self, topic: &str) -> bool {
        let subscribed = self.subscribed.read().await;
        subscribed.contains(&topic.to_string())
    }

    /// List all subscribed topics
    pub async fn list_subscribed(&self) -> Vec<String> {
        self.subscribed.read().await.clone()
    }
}

impl Default for TopicManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_topic_subscription() {
        let manager = TopicManager::new();

        let topic = UppeTopic::public_monitors();
        assert!(!manager.is_subscribed(&topic).await);

        manager.subscribe(topic.clone()).await;
        assert!(manager.is_subscribed(&topic).await);

        manager.unsubscribe(&topic).await;
        assert!(!manager.is_subscribed(&topic).await);
    }

    #[tokio::test]
    async fn test_private_result_topics() {
        let owner_id = "12D3KooWAbCdEf";
        let topic = UppeTopic::private_results(owner_id);

        assert_eq!(
            topic,
            format!("/uppe/private-results/{}/v1", owner_id)
        );

        // Different owners have different topics
        let owner_id2 = "12D3KooWXyZabc";
        let topic2 = UppeTopic::private_results(owner_id2);

        assert_ne!(topic, topic2);
    }
}
