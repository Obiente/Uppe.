//! Node configuration types for PeerUP.
//!
//! This module defines the configuration data structures for PeerUP nodes.

use crate::DEFAULT_PORT_RANGE;

/// Configuration options for a PeerUP node
#[derive(Debug, Clone,)]
pub struct NodeConfig {
    /// The port range to listen on
    pub port_range: (u16, u16,),

    /// Path to keypair file (will be generated if it doesn't exist)
    pub keypair_path: Option<String,>,

    /// Bootstrap peers to connect to
    pub bootstrap_peers: Vec<String,>,

    /// Whether to enable mDNS discovery
    pub enable_mdns: bool,

    /// Whether to enable Kademlia discovery
    pub enable_kademlia: bool,

    /// Whether to enable relay support
    pub enable_relay: bool,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            port_range: DEFAULT_PORT_RANGE,
            keypair_path: None,
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            enable_kademlia: true,
            enable_relay: true,
        }
    }
}

impl NodeConfig {
    /// Create a new configuration builder
    pub fn builder() -> NodeConfigBuilder {
        NodeConfigBuilder::default()
    }
}

/// Builder for NodeConfig
#[derive(Default,)]
pub struct NodeConfigBuilder {
    pub(crate) config: NodeConfig,
}
