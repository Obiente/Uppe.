//! Gossipsub event conversions.

use libp2p::gossipsub;

use crate::network::events::PeerUPEvent;

impl From<gossipsub::Event> for PeerUPEvent {
    fn from(event: gossipsub::Event) -> Self {
        match event {
            gossipsub::Event::Message { propagation_source, message_id, message } => {
                PeerUPEvent::GossipsubMessage { peer: propagation_source, message_id, message }
            }
            gossipsub::Event::Subscribed { peer_id, topic } => PeerUPEvent::GossipsubSubscribed {
                peer: peer_id,
                topic: gossipsub::IdentTopic::new(topic.as_str()),
            },
            gossipsub::Event::Unsubscribed { peer_id, topic } => {
                PeerUPEvent::GossipsubUnsubscribed {
                    peer: peer_id,
                    topic: gossipsub::IdentTopic::new(topic.as_str()),
                }
            }
            other => PeerUPEvent::Gossipsub(other),
        }
    }
}
