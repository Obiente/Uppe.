//! Conversions from relay events to PeerUPEvent.

use libp2p::relay;

use crate::network::events::PeerUPEvent;

impl From<relay::Event> for PeerUPEvent {
    fn from(event: relay::Event) -> Self {
        match event {
            relay::Event::ReservationReqAccepted { src_peer_id, .. } => {
                PeerUPEvent::PeerDiscovered(src_peer_id)
            }
            relay::Event::CircuitReqAccepted { src_peer_id, .. } => {
                PeerUPEvent::PeerDiscovered(src_peer_id)
            }
            _ => PeerUPEvent::Noop,
        }
    }
}
