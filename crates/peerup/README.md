# PeerUP

PeerUP is a standalone Rust crate that enables decentralized uptime monitoring. It communicates with other PeerUP nodes to exchange probe requests, results, and health state using a modular, extensible architecture.

## Features

- **Peer-to-peer uptime monitoring** - Distributed monitoring without central authority
- **Decentralized probe coordination** - Coordinate monitoring tasks across network peers
- **NAT traversal** - Using libp2p relay for connectivity through firewalls
- **LAN discovery** - Automatic peer discovery using mDNS
- **Wide-area discovery** - Kademlia DHT for global peer discovery
- **Custom protocol** - Efficient binary protocol for probe requests and responses
- **Modular architecture** - Well-organized codebase with clear separation of concerns

## Architecture

PeerUP is organized into several key modules:

- **`network`** - Core networking layer with libp2p integration
- **`protocol`** - Custom protocol types and message codec
- **`node`** - Node configuration, lifecycle, and crypto operations
- **`handlers`** - HTTP probe handling and response processing
- **`discovery`** - Peer discovery mechanisms (Kademlia, mDNS)
- **`relay`** - Relay server configuration and management
- **`transport`** - Network transport abstractions

## Usage

### Basic Node Setup

```rust
use peerup::{PeerNode, PeerNodeConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create and start a peer node
    let mut node = PeerNode::new().await?;
    node.run().await?;
    
    Ok(())
}
```

### Custom Configuration

```rust
use peerup::{PeerNode, PeerNodeConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = PeerNodeConfig::new()
        .with_port(8080)
        .with_relay_servers(vec![
            "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN".parse()?,
        ]);
    
    let mut node = PeerNode::with_config(config).await?;
    node.run().await?;
    
    Ok(())
}
```

### Integration with Uppe

PeerUP can be integrated into Uppe's monitoring layer as a dependency, allowing Uppe to:

- **Broadcast monitoring tasks** to trusted peers in the network
- **Aggregate results** back into Uppe's dashboard/status API
- **Provide decentralized uptime monitoring** capabilities
- **Scale monitoring** across multiple geographic regions

```rust
use peerup::{PeerNode, ProbeRequest, ProbeResponse};

// In your Uppe integration:
let mut peer_node = PeerNode::new().await?;

// Send probe requests to the network
let probe_request = ProbeRequest {
    target: "https://example.com".parse()?,
    method: "GET".to_string(),
    headers: Default::default(),
    body: None,
};

peer_node.send_probe_request(probe_request).await?;
```

## Development

### Running Tests

```bash
cargo test
```

### Running Examples

```bash
# Run the modular test example
cargo run --example modular_test

# Run a simple node
cargo run --example simple_node
```

### Building

```bash
cargo build --release
```
