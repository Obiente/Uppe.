//! Conversions from relay events to PeerUPEvent.
//!
//! Note: This conversion is a best-effort mapping. Not all relay events map
//! cleanly to PeerUPEvent, and some variants use PeerDiscovered with a random
//! PeerId as a fallback. Adjust as needed for your use case.

use libp2p::relay;

use crate::network::events::PeerUPEvent;

impl From<relay::Event,> for PeerUPEvent {
    fn from(event: relay::Event,) -> Self {
        match event {
            relay::Event::ReservationReqAccepted {
                src_peer_id, ..
            } => PeerUPEvent::PeerDiscovered(src_peer_id,),
            relay::Event::CircuitReqAccepted {
                src_peer_id, ..
            } => PeerUPEvent::PeerDiscovered(src_peer_id,),
            _ => PeerUPEvent::PeerDiscovered(libp2p::PeerId::random(),),
        }
    }
}
