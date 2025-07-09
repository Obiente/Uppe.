//! Conversions from request_response events to PeerUPEvent.
//!
//! Note: The libp2p request_response API does not expose a request_id in all events. As a result,
//! we use a placeholder value (0) for request_id in ProbeResponseReceived, OutboundProbeFailure,
//! and InboundProbeFailure. If you need to track request IDs, you must do so externally.

use libp2p::request_response;
use crate::protocol::{ProbeRequest, ProbeResponse};
use crate::network::events::PeerUPEvent;

impl From<request_response::Event<ProbeRequest, ProbeResponse>> for PeerUPEvent {
    fn from(event: request_response::Event<ProbeRequest, ProbeResponse>) -> Self {
        match event {
            request_response::Event::Message { peer, message, .. } => {
                match message {
                    request_response::Message::Request { request, channel, .. } => {
                        PeerUPEvent::ProbeRequestReceived { peer, request, channel }
                    }
                    request_response::Message::Response { response, .. } => {
                        // TODO: Track request_id externally if needed
                        let request_id = 0u64; // Placeholder - libp2p does not expose request_id
                        PeerUPEvent::ProbeResponseReceived { peer, request_id, response }
                    }
                }
            }
            request_response::Event::OutboundFailure { peer, error, .. } => {
                let request_id = 0u64; // Placeholder - libp2p does not expose request_id
                PeerUPEvent::OutboundProbeFailure { peer, request_id, error }
            }
            request_response::Event::InboundFailure { peer, error, .. } => {
                let request_id = 0u64; // Placeholder - libp2p does not expose request_id
                PeerUPEvent::InboundProbeFailure { peer, request_id, error }
            }
            request_response::Event::ResponseSent { .. } => {
                // No-op: ResponseSent is not mapped to a PeerUPEvent variant
                // You may log or handle this event elsewhere if needed
                PeerUPEvent::PeerDiscovered(libp2p::PeerId::random())
            }
        }
    }
}
