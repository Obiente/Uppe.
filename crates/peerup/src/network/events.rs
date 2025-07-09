//! Network events for PeerUP.
//!
//! This module defines the events emitted by the PeerUP network behaviour.

use libp2p::{
    request_response,
    PeerId,
};

use crate::protocol::{ProbeRequest, ProbeResponse};

/// Events emitted by the PeerUPBehaviour
#[derive(Debug)]
pub enum PeerUPEvent {
    /// Probe request was received
    ProbeRequestReceived {
        peer: PeerId,
        request: ProbeRequest,
        channel: request_response::ResponseChannel<ProbeResponse>,
    },
    /// Probe response was received
    ProbeResponseReceived {
        peer: PeerId,
        request_id: u64,
        response: ProbeResponse,
    },
    /// Outbound probe request failed
    OutboundProbeFailure {
        peer: PeerId,
        request_id: u64,
        error: request_response::OutboundFailure,
    },
    /// Inbound probe request failed
    InboundProbeFailure {
        peer: PeerId,
        request_id: u64,
        error: request_response::InboundFailure,
    },
    /// A peer was discovered
    PeerDiscovered(PeerId),
    /// A peer was removed from the network
    PeerRemoved(PeerId),
    /// The local node received a connection from a peer
    ConnectionEstablished(PeerId),
    /// The local node's connection to a peer was closed
    ConnectionClosed(PeerId),
    /// Relay event
    Relay(libp2p::relay::Event),
    /// Kademlia event
    Kademlia(libp2p::kad::Event),
    /// Mdns event
    Mdns(libp2p::mdns::Event),
    /// Request/response event
    RequestResponse(request_response::Event<ProbeRequest, ProbeResponse>),
}
