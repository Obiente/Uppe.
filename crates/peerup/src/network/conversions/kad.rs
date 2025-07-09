//! Conversions from Kademlia events to PeerUPEvent.
//!
//! Note: This conversion is a best-effort mapping. Not all Kademlia events map
//! cleanly to PeerUPEvent, and some variants use PeerDiscovered with a random
//! PeerId as a fallback. Adjust as needed for your use case.

use libp2p::{kad, PeerId};

use crate::network::events::PeerUPEvent;

impl From<kad::Event> for PeerUPEvent {
    fn from(event: kad::Event) -> Self {
        use kad::Event::*;
        use kad::QueryResult::*;
        match event {
            OutboundQueryProgressed { result: GetClosestPeers(Ok(peers)), .. } => {
                if let Some(peer_info) = peers.peers.into_iter().next() {
                    PeerUPEvent::PeerDiscovered(peer_info.peer_id)
                } else {
                    PeerUPEvent::PeerDiscovered(PeerId::random())
                }
            }
            OutboundQueryProgressed { .. } => PeerUPEvent::PeerDiscovered(PeerId::random()),
            RoutingUpdated { peer, .. }
            | PendingRoutablePeer { peer, .. } => PeerUPEvent::PeerDiscovered(peer),
            _ => PeerUPEvent::PeerDiscovered(PeerId::random()),
        }
    }
}
