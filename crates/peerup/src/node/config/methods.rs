//! Node configuration methods for PeerUP.
//!
//! This module defines the configuration methods for PeerUP nodes.

use super::types::{NodeConfig, NodeConfigBuilder};

impl NodeConfig {
    /// Enable or disable mDNS discovery
    pub fn with_mdns(mut self, enable: bool) -> Self {
        self.enable_mdns = enable;
        self
    }

    /// Enable or disable Kademlia discovery
    pub fn with_kademlia(mut self, enable: bool) -> Self {
        self.enable_kademlia = enable;
        self
    }

    /// Enable or disable relay support
    pub fn with_relay(mut self, enable: bool) -> Self {
        self.enable_relay = enable;
        self
    }

    /// Set bootstrap peers
    pub fn with_bootstrap_peers(mut self, peers: Vec<String>) -> Self {
        self.bootstrap_peers = peers;
        self
    }

    /// Set keypair path
    pub fn with_keypair_path(mut self, path: String) -> Self {
        self.keypair_path = Some(path);
        self
    }

    /// Set port range
    pub fn with_port_range(mut self, range: (u16, u16)) -> Self {
        self.port_range = range;
        self
    }
}

impl NodeConfigBuilder {
    /// Build the configuration
    pub fn build(self) -> NodeConfig {
        self.config
    }

    /// Set port range
    pub fn port_range(mut self, range: (u16, u16)) -> Self {
        self.config.port_range = range;
        self
    }

    /// Set keypair path
    pub fn keypair_path(mut self, path: String) -> Self {
        self.config.keypair_path = Some(path);
        self
    }

    /// Add bootstrap peer
    pub fn bootstrap_peer(mut self, peer: String) -> Self {
        self.config.bootstrap_peers.push(peer);
        self
    }

    /// Enable mDNS
    pub fn enable_mdns(mut self) -> Self {
        self.config.enable_mdns = true;
        self
    }

    /// Disable mDNS
    pub fn disable_mdns(mut self) -> Self {
        self.config.enable_mdns = false;
        self
    }
}
