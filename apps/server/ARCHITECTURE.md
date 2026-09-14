# Go API architecture

The supported runtime uses the shared Rust-owned SQLite schema and ConnectRPC contracts in proto/. The API authenticates operator requests and exposes a separate anonymous status projection. It does not run migrations or create monitor results. Runtime settings are configured in Rust, not through simulated database settings.

See [the root architecture](../../ARCHITECTURE.md), [run instructions](../../README.md), and [implementation status](../../docs/implementation-status.md). The network is open to any peer; joining and executing remote work require explicit installation configuration.
