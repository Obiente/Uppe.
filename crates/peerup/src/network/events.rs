//! Network events for PeerUP.
//!
//! This module defines the events emitted by the PeerUP network behaviour.

use libp2p::{gossipsub, request_response, PeerId};

use crate::protocol::{ProbeRequest, ProbeResponse};

/// Events emitted by the PeerUPBehaviour
#[derive(Debug)]
pub enum PeerUPEvent {
    /// A gossipsub message was received
    GossipsubMessage {
        peer: PeerId,
        message_id: gossipsub::MessageId,
        message: gossipsub::Message,
    },
    /// Successfully subscribed to a gossipsub topic
    GossipsubSubscribed {
        peer: PeerId,
        topic: gossipsub::IdentTopic,
    },
    /// Unsubscribed from a gossipsub topic
    GossipsubUnsubscribed {
        peer: PeerId,
        topic: gossipsub::IdentTopic,
    },
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
    /// Gossipsub event (other)
    Gossipsub(gossipsub::Event),
    /// Relay event
    Relay(libp2p::relay::Event),
    /// Kademlia event
    Kademlia(libp2p::kad::Event),
    /// DHT get record success (first record)
    DhtGetRecordOk {
        key: Vec<u8>,
        record: Vec<u8>,
    },
    /// DHT get record not found
    DhtGetRecordErr {
        key: Vec<u8>,
    },
    /// DHT put record success
    DhtPutRecordOk {
        key: Vec<u8>,
    },
    /// DHT put record failed
    DhtPutRecordErr {
        key: Vec<u8>,
        error: String,
    },
    /// Mdns event
    Mdns(libp2p::mdns::Event),
    /// Request/response event
    RequestResponse(request_response::Event<ProbeRequest, ProbeResponse>),
    /// No-op event — used when a protocol event has no meaningful PeerUP mapping
    Noop,
}
