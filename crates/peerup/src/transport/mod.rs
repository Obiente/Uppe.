//! Transport setup for PeerUP nodes.
//!
//! This module handles the setup of libp2p transport layer.

use anyhow::Result;
use libp2p::{
    identity::Keypair,
    noise, yamux, tcp, dns,
    Transport,
};

/// Build the transport for a PeerUP node
pub fn build_transport(keypair: &Keypair) -> Result<libp2p::core::transport::Boxed<(libp2p::PeerId, libp2p::core::muxing::StreamMuxerBox)>> {
    // Create noise configuration
    let noise_config = noise::Config::new(keypair)?;
    
    // Set up TCP transport with DNS resolution
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
    let dns_transport = dns::tokio::Transport::system(tcp_transport)?;
    
    // Build the transport stack
    let transport = dns_transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(yamux::Config::default())
        .timeout(std::time::Duration::from_secs(20))
        .boxed();
    
    Ok(transport)
}

/// Create a development transport (simplified, for testing)
pub fn build_dev_transport(keypair: &Keypair) -> Result<libp2p::core::transport::Boxed<(libp2p::PeerId, libp2p::core::muxing::StreamMuxerBox)>> {
    let noise_config = noise::Config::new(keypair)?;
    
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
    
    let transport = tcp_transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(yamux::Config::default())
        .timeout(std::time::Duration::from_secs(10))
        .boxed();
    
    Ok(transport)
}
