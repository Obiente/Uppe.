//! Conversions from mdns events to PeerUPEvent.
//!
//! Note: This conversion is a best-effort mapping. Not all mDNS events map
//! cleanly to PeerUPEvent, and some variants use PeerDiscovered or PeerRemoved
//! with a random PeerId as a fallback. Adjust as needed for your use case.

use libp2p::{mdns, PeerId};

use crate::network::events::PeerUPEvent;

impl From<mdns::Event> for PeerUPEvent {
    fn from(event: mdns::Event) -> Self {
        match event {
            mdns::Event::Discovered(list) => {
                if let Some((peer_id, _)) = list.into_iter().next() {
                    PeerUPEvent::PeerDiscovered(peer_id)
                } else {
                    PeerUPEvent::PeerDiscovered(PeerId::random())
                }
            },
            mdns::Event::Expired(list) => {
                if let Some((peer_id, _)) = list.into_iter().next() {
                    PeerUPEvent::PeerRemoved(peer_id)
                } else {
                    PeerUPEvent::PeerRemoved(PeerId::random())
                }
            },
        }
    }
}
