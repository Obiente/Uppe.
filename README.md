# Uppe.

Self-hosted uptime monitoring with a Rust monitoring service, Go API, and Astro dashboard. One installation owns its monitors and SQLite database. Public peers can exchange signed observations; those observations do not decide the health shown on your status pages.

## Run locally

Install Node.js 24 LTS, pnpm 10.12.1, Go 1.26.8 or newer, and Rust through rustup. The repository pins Rust 1.98.1. Building Rust also requires a C/C++ toolchain: Visual Studio Build Tools with Desktop development with C++ on Windows, or a compiler and linker on Linux.

```sh
pnpm install --frozen-lockfile
pnpm build --debug
pnpm start --debug
```

Open http://127.0.0.1:4321. Sign in with the generated key in `.uppe/access.key`. Create an HTTP or TCP monitor, wait for its first check, then create and publish a status page. New status pages stay hidden until explicitly published. Pause and edit monitors from their detail pages. Changes reach the scheduler within 30 seconds.

For optimized binaries, use `pnpm build` and `pnpm start` without `--debug`. The launcher migrates the database before starting Rust, Go, and Astro, and stops the installation if a required process exits. Ctrl+C stops the processes. For frontend iteration, run `pnpm dev:client` against an already running API with the same access key and `UPPE_API_URL`.

Local HTTP monitors may reach your private services. Checks use each monitor's timeout, accept HTTP 200-399, and do not follow redirects. ICMP and custom HTTP bodies, headers, or accepted status codes are not currently supported; the API rejects these options. Disabled or stale checks show as unknown or paused, never as healthy by default.

## Configuration and operation

Default runtime state is in the ignored `.uppe` directory:

| File | Purpose |
| --- | --- |
| config.toml | Rust runtime and network configuration; restart after edits |
| uppe.db | SQLite database, with adjacent WAL files while running |
| node.key | Persistent node signing and encryption identity |
| access.key | Operator login secret |
| bin/uppe-api (.exe on Windows) | Built Go executable |

Set `UPPE_DATA_DIR` to relocate this directory; use the same value for build and start. `UPPE_DATABASE_PATH` and `UPPE_KEYPAIR_PATH` override individual paths. `UPPE_OPERATOR_TOKEN` supplies an external secret of at least 32 characters instead of generating an access key. Use a random secret and restart all components when rotating it; existing sessions then become invalid.

Astro listens on `HOST`/`PORT` (default 127.0.0.1:4321). Go listens on `UPPE_API_ADDRESS` (default 127.0.0.1:8080), and Astro connects through `UPPE_API_URL`. Expose only Astro through an HTTPS reverse proxy, preserving the public Host and scheme. Keep Go private. Operator requests require authentication; only the narrow public-status projection and liveness endpoint are anonymous. The browser never receives the backend bearer token. Public status pages expose names and aggregate health, not targets or peer identities.

Stop the installation before copying the complete data directory for a backup. Retain the identity key together with the database. SQLite is local to this installation; network filesystems and multiple Rust writers are not supported. Raw internal/private checks are retained for 7 days and public checks for 30 days. Peer observations and their signed receipts expire after 7 days and may be evicted earlier under storage budgets. Peer cleanup removes all expired observations hourly. The permanent audit history of operator changes and local checks is append-only and is not automatically pruned; plan disk capacity and backups accordingly.

Public status projections are cached for up to five seconds. Publication is checked on every read, and API management changes invalidate cached projections. Public requests have a global budget of 30 requests/second with a burst of 60, at most 16 concurrent requests, and one database refresh at a time. Excess requests receive 429 or a resource-exhausted response. Visit counts are approximate, batched every 30 seconds, and can lose the last batch on shutdown.

## Open public network

Anyone can run a peer. There is no membership allowlist. Network participation is an explicit installation choice because it exposes your node to other machines. In config.toml, set `preferences.use_peerup_layer = true`. Configure reachable `peerup.bootstrap_peers` as libp2p multiaddresses; mDNS can discover peers on the same LAN. No managed public bootstrap service is bundled.

Set `preferences.accept_remote_checks = true` when this node should execute public-network requests. Remote checks are limited to public destinations after DNS resolution, with pinned addresses, no proxy inheritance, no redirects, at most ten helper leases, and ten-minute leases. The executor allows at most 64 concurrent checks. The transport caps established connections at 128 (at most two per peer). Received gossip has message-size and ingestion limits. Location sharing defaults to disabled.

Checks skipped because the executor is full are recorded as unknown, excluded from target uptime calculations, and never reported as a target outage. Peer identity mappings expire after five idle minutes and evict the least recently used mapping at capacity, so churn cannot permanently close admission.

Only observations for enabled local subscriptions and matching targets are retained; public gossip requires a public subscription, while helper results require an active assignment. Insertion deduplicates peer/monitor/timestamp identities and limits each peer to the configured check interval per monitor. Signed receipts live with the observations in `peer_results.receipt_json`, outside the permanent operator ledger. Limits are 20,000 observations overall, 1,000 per peer, 8 KiB per receipt and 512 bytes per error text; the oldest observations are evicted transactionally to admit fresh evidence. These are payload budgets, not a total database-file size guarantee. Peer metadata retains at most 4,096 recent identities. Multiple public identities do not imply independent witnesses.

Schema version 8 moves new peer evidence to this bounded store and discards pending unsigned legacy peer-outbox entries. Previously signed audit history remains intact. Rust and Go must be upgraded together; the launcher applies the migration before starting them.

`enable_distributed_monitoring` enables experimental helper coordination. Signatures bind observations to stable identities, but multiple keys do not prove independent operators. Peer observations remain separate from local health. Helper assignment targets are currently visible to network participants even though results are encrypted for the owner. Do not place credentials or sensitive target details in shared monitors.

Automatic offline history recovery, Sybil-resistant consensus, replicated status-page hosting, capability delegation, and operational admin-key distribution remain unfinished. They are not required for local monitoring and status pages. See [implementation status](docs/implementation-status.md) and [architecture](ARCHITECTURE.md).

## Development

`pnpm check` runs Rust formatting, Clippy and tests; builds Rust; tests Go against the actual Rust-created schema; checks protobuf generation; and checks, tests, and builds Astro. CI runs it on Linux and Windows. `pnpm proto` regenerates committed bindings from apps/server/proto; edit schemas, not generated files.

Rust owns migrations and signed storage events. Go writes monitor/status-page configuration in transactions but does not write check results or run migrations. SQLite triggers enqueue changes in the same transaction; Rust signs them through a durable outbox. Audit records attest to what this installation stored, not to the truth of a peer's claim.
