//! Conversions from mdns events to PeerUPEvent.

use libp2p::mdns;

use crate::network::events::PeerUPEvent;

impl From<mdns::Event> for PeerUPEvent {
    fn from(event: mdns::Event) -> Self {
        match event {
            mdns::Event::Discovered(list) => {
                if let Some((peer_id, _)) = list.into_iter().next() {
                    PeerUPEvent::PeerDiscovered(peer_id)
                } else {
                    PeerUPEvent::Noop
                }
            }
            mdns::Event::Expired(list) => {
                if let Some((peer_id, _)) = list.into_iter().next() {
                    PeerUPEvent::PeerRemoved(peer_id)
                } else {
                    PeerUPEvent::Noop
                }
            }
        }
    }
}
