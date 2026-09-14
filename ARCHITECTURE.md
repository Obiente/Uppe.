# Uppe Architecture

Updated: 2026-09-14

This document describes intended ownership, including future capabilities. For implemented behavior and remaining work, see [implementation status](docs/implementation-status.md). Runtime settings currently belong to config.toml; replicated bundles and capability operations are not yet implemented.

## Purpose

This document freezes the intended ownership boundaries for Uppe so future implementation work has a stable target.

It is the root architecture reference for:

- runtime ownership
- database schema ownership
- RPC/API ownership
- frontend ownership
- P2P ownership
- trust and audit ownership

This document should be read together with:

- [PROJECT_PLAN.md](PROJECT_PLAN.md)

## System Overview

Uppe is a decentralized monitoring platform made of four major systems:

1. Rust monitoring node
2. Go API/query layer
3. Astro frontend
4. Peer verification, replication, and audit network

Those systems must cooperate, but they must not overlap in responsibility.

## Ownership Boundaries

## Rust Service

Location:

- [apps/service](apps/service)

Rust owns:

- monitor execution
- scheduling
- local result creation
- key management
- canonical event creation and signing
- P2P runtime integration
- replication behavior
- trust verification
- status-page bundle publication

Rust does not own:

- browser-facing query APIs
- frontend rendering
- ad hoc CRUD workflows meant only for UI consumption

## Go API

Location:

- [apps/server](apps/server)

Go owns:

- RPC/query APIs for the frontend and external clients
- CRUD APIs for monitors, status pages, and settings
- aggregation and read models
- query-oriented access to audit and network state

Go does not own:

- database migrations
- monitoring execution
- peer networking
- trust source-of-truth decisions

## Astro Frontend

Location:

- [apps/client](apps/client)

Astro owns:

- operator dashboard UX
- monitor management UX
- analytics UX
- status-page management UX
- public status-page rendering UX
- audit inspection UX

Astro does not own:

- long-lived business state
- handwritten API contract truth
- trust verification logic

## PeerUP

Location:

- [crates/peerup](crates/peerup)

PeerUP owns reusable infrastructure for:

- peer discovery
- gossip topics
- DHT operations
- transport behavior
- generic replication primitives
- peer verification message transport

PeerUP does not own Uppe-specific dashboard or CRUD rules.

## Sources of Truth

## Database Schema

Canonical source:

- Rust migrations in [apps/service/src/database/migrations.rs](apps/service/src/database/migrations.rs)

Rule:

- Rust migrations are the only executable database schema source.

Implications:

- Go must adapt to the schema created by Rust.
- shared schema docs must be generated from Rust-owned migration state or treated as documentation only.
- obsolete migration files outside Rust must not remain active sources of truth.

## RPC/API Contracts

Canonical source:

- protobuf definitions in [apps/server/proto](apps/server/proto)

Rule:

- Protobuf is the only API contract source.

Implications:

- Go service implementations must match active protos.
- Astro uses generated TS clients and types.
- handwritten parallel API DTOs should be minimized.

## Signed Event Format

Canonical owner:

- Rust service domain/trust layer

Rule:

- Uppe uses a canonical signed event envelope for trust-sensitive actions.

This envelope will back:

- monitor changes
- monitoring results
- peer attestations
- status-page publish events
- audit records
- trust-chain updates

## Runtime Architecture

## Monitoring Runtime

The monitoring runtime is responsible for:

- scheduling checks
- running checkers
- generating local results
- writing local results

It should not know transport details.

## P2P Runtime

The P2P runtime is responsible for:

- joining the network
- discovering peers
- publishing and receiving protocol messages
- replication transport
- DHT fetch/store behavior

It should not decide product semantics by itself.

## Trust Runtime

The trust runtime is responsible for:

- signing
- signature verification
- attestation verification
- trust-chain validation
- audit integrity checks

It should be isolated from frontend or query concerns.

## Orchestration Layer

The orchestration layer composes:

- monitoring runtime
- P2P runtime
- trust runtime
- storage

It is allowed to coordinate, but not to become the home for all business logic indefinitely.

## Domain Model

## Monitors

Monitor visibility modes:

- Public
- Private
- Internal

Rules:

- Public monitors participate in network coordination.
- Private monitors allow assisted workflows with privacy guarantees.
- Internal monitors never leave the owner node.

## Results

Result classes:

- local results
- peer results
- verification attestations
- derived aggregations

Rules:

- raw results are immutable evidence
- verification is an additional record, not an in-place mutation of truth

## Status Pages

Status pages have two layers:

1. Authoring layer
- page identity
- branding
- monitor bindings
- publication state

2. Published bundle layer
- immutable signed bundle
- page assets
- summary snapshot
- replication metadata

## Audit

Audit is a first-class system, not a log appendix.

Audit tracks:

- what happened
- who signed it
- who verified it
- where it replicated
- whether it met verification thresholds

## Public Status Page Hosting Model

The intended hosting model is:

1. Owner publishes a signed status-page bundle.
2. Peers replicate the bundle and recent signed summary data.
3. A browser-compatible gateway serves the bundle even if the owner node is offline.
4. Custom domains map to the gateway/resolver layer.

This means early resilient hosting is gateway-backed, network-replicated hosting.

It is explicitly not:

- arbitrary peer-hosted dynamic frontend execution
- browser-native libp2p page hosting as the first milestone

## Interface Boundaries

## Rust -> Go

Primary interface:

- shared database state
- optional future internal contracts only when necessary

Rule:

- Go reads and mutates user-facing state, but does not become the execution engine.

## Go -> Astro

Primary interface:

- ConnectRPC/protobuf-generated contracts

Rule:

- Astro should not reach into DB assumptions directly.

## Rust -> PeerUP

Primary interface:

- explicit transport/replication abstractions

Rule:

- Uppe domain code should depend on protocol boundaries, not raw libp2p behavior whenever possible.

## Immediate Rules for Ongoing Work

1. No new schema source may be introduced outside Rust migrations.
2. No new API surface should be documented as active unless it is implemented or clearly marked experimental.
3. No trust-critical placeholder verification may remain in paths that claim production trust guarantees.
4. No new major frontend page should ship as if complete while its backend is still mock-only.
5. New work should attach to one of these workstreams:
- Platform
- Distributed Runtime
- Trust and Audit

## Near-Term Architecture Deliverables

1. Remove schema drift between Rust, shared docs, and old Go migration files.
2. Align active proto services with actual Go implementations.
3. Split Rust service responsibilities into clearer domain/runtime modules.
4. Define the canonical signed event envelope.
5. Implement status-page bundles and replicated gateway serving.
6. Implement audit events and peer attestations.

## Decision Summary

The final architecture direction is:

- Rust executes and signs
- Go serves and aggregates
- Astro renders and manages UX
- PeerUP transports and replicates
- Rust migrations define the database
- protobuf defines the API
- signed events define trust and audit

Any implementation that crosses those boundaries needs explicit justification.
