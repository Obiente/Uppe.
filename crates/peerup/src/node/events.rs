//! Event handling for PeerUP nodes.
//!
//! This module handles network events and swarm events.

use libp2p::swarm::SwarmEvent;
use tracing::{debug, info, warn};

use crate::network::PeerUPEvent;

/// Handle a PeerUP network event.
///
/// Note: `ProbeRequestReceived` events carry a `ResponseChannel` that must be
/// used via `swarm.behaviour_mut().request_response.send_response(channel, resp)`.
/// Since this standalone handler has no swarm access, probe requests are logged
/// but the response channel is returned to the caller. Consumers that need full
/// probe handling (like the Uppe service) should match on `PeerUPEvent` directly
/// in their own event loop.
pub fn handle_peerup_event(event: PeerUPEvent) {
    match event {
        PeerUPEvent::ProbeRequestReceived { peer, request, channel } => {
            // We cannot send a response here because we don't have swarm access.
            // Log the request and drop the channel — the peer will see an inbound
            // failure.  Production consumers should handle this variant in their
            // own swarm event loop where they have mutable access to the behaviour.
            warn!(
                "ProbeRequestReceived from {} ({:?}) — dropping channel \
                 (handle in your own event loop to respond)",
                peer, request
            );
            drop(channel);
        }
        PeerUPEvent::ProbeResponseReceived { peer, request_id, response } => {
            info!("Received probe response from {} (ID: {}): {:?}", peer, request_id, response);
        }
        PeerUPEvent::OutboundProbeFailure { peer, request_id, error } => {
            warn!("Outbound probe failed to {} (ID: {}): {:?}", peer, request_id, error);
        }
        PeerUPEvent::InboundProbeFailure { peer, request_id, error } => {
            warn!("Inbound probe failed from {} (ID: {}): {:?}", peer, request_id, error);
        }
        PeerUPEvent::PeerDiscovered(peer_id) => {
            info!("Discovered peer: {}", peer_id);
        }
        PeerUPEvent::PeerRemoved(peer_id) => {
            info!("Peer removed: {}", peer_id);
        }
        PeerUPEvent::ConnectionEstablished(peer_id) => {
            info!("Connection established with: {}", peer_id);
        }
        PeerUPEvent::ConnectionClosed(peer_id) => {
            info!("Connection closed with: {}", peer_id);
        }
        PeerUPEvent::Relay(ev) => {
            debug!("Relay event: {:?}", ev);
        }
        PeerUPEvent::Kademlia(ev) => {
            debug!("Kademlia event: {:?}", ev);
        }
        PeerUPEvent::Mdns(ev) => {
            debug!("Mdns event: {:?}", ev);
        }
        PeerUPEvent::Noop => {}
        other => {
            debug!("Unhandled PeerUPEvent variant: {:?}", other);
        }
    }
}

/// Handle swarm events
pub fn handle_swarm_event(event: SwarmEvent<PeerUPEvent>) {
    match event {
        SwarmEvent::Behaviour(peerup_event) => {
            handle_peerup_event(peerup_event);
        }
        SwarmEvent::NewListenAddr { address, .. } => {
            info!("Listening on {}", address);
        }
        SwarmEvent::IncomingConnection { local_addr, send_back_addr, .. } => {
            debug!("Incoming connection from {} to {}", send_back_addr, local_addr);
        }
        SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
            info!("Connection established with {} via {}", peer_id, endpoint.get_remote_address());
        }
        SwarmEvent::ConnectionClosed { peer_id, endpoint, cause, .. } => {
            info!(
                "Connection closed with {} via {} (cause: {:?})",
                peer_id,
                endpoint.get_remote_address(),
                cause
            );
        }
        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
            if let Some(peer_id) = peer_id {
                warn!("Outgoing connection error to {}: {:?}", peer_id, error);
            } else {
                warn!("Outgoing connection error: {:?}", error);
            }
        }
        SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
            warn!(
                "Incoming connection error from {} to {}: {:?}",
                send_back_addr, local_addr, error
            );
        }
        _ => {}
    }
}
