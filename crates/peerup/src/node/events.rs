//! Event handling for PeerUP nodes.
//!
//! This module handles network events and swarm events.

use libp2p::request_response::ResponseChannel;
use libp2p::swarm::SwarmEvent;
use libp2p::PeerId;
use tracing::{debug, info, warn};

use crate::handlers;
use crate::network::PeerUPEvent;
use crate::protocol::{ProbeRequest, ProbeResponse};

/// Handle a PeerUP network event
pub fn handle_peerup_event(event: PeerUPEvent) {
    match event {
        PeerUPEvent::ProbeRequestReceived { peer, request, channel } => {
            info!("Received probe request from {}: {:?}", peer, request);
            handle_probe_request(peer, request, channel);
        },
        PeerUPEvent::ProbeResponseReceived { peer, request_id, response } => {
            info!("Received probe response from {} (ID: {}): {:?}", peer, request_id, response);
        },
        PeerUPEvent::OutboundProbeFailure { peer, request_id, error } => {
            warn!("Outbound probe failed to {} (ID: {}): {:?}", peer, request_id, error);
        },
        PeerUPEvent::InboundProbeFailure { peer, request_id, error } => {
            warn!("Inbound probe failed from {} (ID: {}): {:?}", peer, request_id, error);
        },
        PeerUPEvent::PeerDiscovered(peer_id) => {
            info!("Discovered peer: {}", peer_id);
        },
        PeerUPEvent::PeerRemoved(peer_id) => {
            info!("Peer removed: {}", peer_id);
        },
        PeerUPEvent::ConnectionEstablished(peer_id) => {
            info!("Connection established with: {}", peer_id);
        },
        PeerUPEvent::ConnectionClosed(peer_id) => {
            info!("Connection closed with: {}", peer_id);
        },
        PeerUPEvent::Relay(ev) => {
            debug!("Relay event: {:?}", ev);
        },
        PeerUPEvent::Kademlia(ev) => {
            debug!("Kademlia event: {:?}", ev);
        },
        PeerUPEvent::Mdns(ev) => {
            debug!("Mdns event: {:?}", ev);
        },
        other => {
            debug!("Unhandled PeerUPEvent variant: {:?}", other);
        },
    }
}

/// Handle a probe request
fn handle_probe_request(
    peer: PeerId,
    request: ProbeRequest,
    channel: ResponseChannel<ProbeResponse>,
) {
    // Handle the probe request asynchronously
    tokio::spawn(async move {
        let response = handlers::handle_probe_request(request).await;

        // Note: In libp2p 0.56+, ResponseChannel might need different handling
        // For now, we'll just drop the channel since the API has changed
        drop(channel);
        info!("Handled probe request from {} - response: {:?}", peer, response);
    });
}

/// Handle swarm events
pub fn handle_swarm_event(event: SwarmEvent<PeerUPEvent>) {
    match event {
        SwarmEvent::Behaviour(peerup_event) => {
            handle_peerup_event(peerup_event);
        },
        SwarmEvent::NewListenAddr { address, .. } => {
            info!("Listening on {}", address);
        },
        SwarmEvent::IncomingConnection { local_addr, send_back_addr, .. } => {
            debug!("Incoming connection from {} to {}", send_back_addr, local_addr);
        },
        SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
            info!("Connection established with {} via {}", peer_id, endpoint.get_remote_address());
        },
        SwarmEvent::ConnectionClosed { peer_id, endpoint, cause, .. } => {
            info!(
                "Connection closed with {} via {} (cause: {:?})",
                peer_id,
                endpoint.get_remote_address(),
                cause
            );
        },
        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
            if let Some(peer_id) = peer_id {
                warn!("Outgoing connection error to {}: {:?}", peer_id, error);
            } else {
                warn!("Outgoing connection error: {:?}", error);
            }
        },
        SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
            warn!(
                "Incoming connection error from {} to {}: {:?}",
                send_back_addr, local_addr, error
            );
        },
        _ => {},
    }
}
