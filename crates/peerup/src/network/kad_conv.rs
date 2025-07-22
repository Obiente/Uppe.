//! Conversions from Kademlia events to PeerUPEvent.

use libp2p::kad;
use libp2p::PeerId;
use crate::network::events::PeerUPEvent;

impl From<kad::Event> for PeerUPEvent {
    fn from(event: kad::Event) -> Self {
        match event {
            kad::Event::OutboundQueryProgressed { result, .. } => {
                match result {
                    kad::QueryResult::GetClosestPeers(Ok(peers)) => {
                        if let Some(peer_info) = peers.peers.into_iter().next() {
                            PeerUPEvent::PeerDiscovered(peer_info.peer_id)
                        } else {
                            PeerUPEvent::PeerDiscovered(PeerId::random())
                        }
                    }
                    _ => PeerUPEvent::PeerDiscovered(PeerId::random()),
                }
            }
            kad::Event::RoutingUpdated { peer, .. } => {
                PeerUPEvent::PeerDiscovered(peer)
            }
            kad::Event::PendingRoutablePeer { peer, .. } => {
                PeerUPEvent::PeerDiscovered(peer)
            }
            _ => PeerUPEvent::PeerDiscovered(PeerId::random()),
        }
    }
}
