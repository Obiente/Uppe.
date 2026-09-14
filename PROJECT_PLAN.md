# Uppe Project Plan

Date: 2026-04-04

## Goal

Turn Uppe into a coherent decentralized monitoring platform with:

- a reliable Rust monitoring node
- a clean Go API/query layer
- a typed Astro frontend
- resilient public status pages that can survive owner node downtime
- signed audit/event records replicated and verified across peers

This plan assumes the current direction remains:

- Rust is the execution runtime
- Go is the API/query layer
- Astro is the operator/public UI
- PeerUP is the reusable P2P and replication layer

## Product Vision

Uppe should eventually support three major user experiences:

1. Self-hosted monitoring node
- A user runs a node, creates monitors, sees results, and optionally joins the network.

2. Public decentralized monitoring
- Public monitors are redundantly checked by multiple peers.
- Results are signed and verifiable.

3. Public status pages with network-backed availability
- A user publishes a status page.
- The page remains available even if the owner's node is offline.
- The owner can point a custom domain at the page without self-hosting the full frontend.

## Architecture Principles

### 1. Single ownership per concern

- Rust owns execution, peer protocols, signing, and replication.
- Go owns user-facing APIs, aggregation, and query/mutation workflows.
- Astro owns rendering and UX only.

### 2. One source of truth per contract

- Database schema: Rust migrations only.
- RPC/API schema: protobuf only.
- Frontend generated types: generated from protobuf, not duplicated manually.

### 3. Event-first trust model

- Important actions become signed events.
- Events are append-only.
- Replication and verification happen at the event level.
- UI and APIs query materialized state derived from signed events plus local indexes.

### 4. P2P and application logic stay separate

- P2P transport is not business logic.
- Monitoring logic does not know transport details.
- Trust verification is not mixed into UI-facing handlers.

## Target System Layout

## Rust Service Responsibilities

The Rust service should own:

- monitor scheduling and execution
- local persistence
- key management
- canonical event creation and signing
- peer discovery
- result replication
- audit replication
- trust verification
- status-page bundle publication to the network

Recommended internal split:

- `domain/monitors`
- `domain/results`
- `domain/status_pages`
- `domain/audit`
- `runtime/monitoring`
- `runtime/p2p`
- `runtime/trust`
- `runtime/orchestration`
- `storage/sqlite`

## Go API Responsibilities

The Go server should own:

- monitor CRUD APIs
- result query APIs
- network stats query APIs
- settings APIs
- status page CRUD/query APIs
- audit query APIs
- materialized read models built from shared DB state

It should not own:

- migrations
- peer networking
- cryptographic source-of-truth decisions
- background monitoring execution

## Astro Frontend Responsibilities

The Astro frontend should own:

- operator dashboard
- monitor creation/edit flows
- analytics views
- settings views
- status-page management UX
- public status-page rendering
- audit inspection UI

It should not invent business state locally except for temporary form state.

## PeerUP Responsibilities

PeerUP should be the reusable infrastructure layer for:

- peer discovery
- gossipsub topics
- DHT operations
- content replication
- replication acknowledgements
- peer verification messages

It should not know Uppe-specific dashboard or CRUD semantics.

## Canonical Contracts

## 1. Database Schema

Decision:

- Rust migrations are the only executable schema source.

Rules:

- `apps/service/src/database/migrations.rs` is authoritative.
- `shared/database/schema.sql` becomes generated documentation, not hand-maintained truth.
- old Go migration files are archived or removed from active use.

Required work:

1. Remove schema drift.
2. Add a script that emits schema docs from the Rust migration state.
3. Add CI that fails if shared schema docs drift from the Rust migration output.

## 2. RPC/API Schema

Decision:

- Protobuf is the only API contract source.

Rules:

- Go service implementations must match registered proto services exactly.
- Astro clients must use generated TS clients/types.
- avoid parallel handwritten API DTOs unless they are UI-only view models

Required work:

1. Keep only the services that are actually supported, or implement the missing ones.
2. Add contract tests for generated clients against a live Go server.

## 3. Event Schema

Decision:

- Uppe needs a canonical signed event envelope.

Proposed envelope fields:

- `event_id`
- `event_type`
- `version`
- `timestamp`
- `source_peer_id`
- `subject_id`
- `payload`
- `payload_hash`
- `signature`
- `public_key`
- `previous_event_hash` optional

This envelope will back:

- monitor mutations
- monitoring results
- peer verification attestations
- status page revisions
- audit records
- admin trust updates

## Domain Design

## 1. Monitors

Subtypes:

- public monitors
- private monitors
- internal monitors

Rules:

- Public monitors participate in shared coordination.
- Private monitors allow peer-assisted encrypted workflows.
- Internal monitors never leave the owner node.

Needed output:

- one clean domain model
- one persistence model
- one API representation

## 2. Results

Results should be split conceptually:

- local check results
- peer-received results
- verification attestations
- aggregated read models

Rules:

- raw signed results are immutable
- verification is a separate attestation, not an in-place mutation of trust

## 3. Status Pages

Status pages need two layers:

1. Authoring layer
- owner defines title, slug, branding, linked monitors, visibility

2. Published bundle layer
- deterministic signed page manifest
- assets
- latest signed summary snapshot
- optional recent incidents summary

Status pages should be renderable without needing the owner node online.

## 4. Audit

Audit must not be an afterthought. It should be a first-class append-only record system.

Core concepts:

- signed event
- peer attestation
- replication proof
- query index

Audit queries should answer:

- what happened
- who claimed it
- who verified it
- what evidence exists
- whether quorum was met

## Public Status Pages: Resilient Hosting Plan

## Problem

Users want public monitor pages on their own domain without having to keep their own node online. The network should preserve availability.

## Practical Design

Do not start with "browser-native P2P hosting". Browsers and DNS constraints make that the wrong first milestone.

Use a layered design:

### Layer 1: Signed page bundle

Each published status page becomes a signed immutable bundle:

- manifest
- branding metadata
- monitor bindings
- rendered assets
- current signed status summary

Bundle fields:

- `bundle_id`
- `owner_peer_id`
- `status_page_id`
- `revision`
- `content_hash`
- `created_at`
- `signature`

### Layer 2: Replication across peers

Peers store and serve:

- page manifest
- immutable assets
- recent signed summaries

This is content replication, not arbitrary app hosting.

### Layer 3: Gateway serving

Gateways serve content by:

- content hash
- page slug
- owner mapping

This gives browser compatibility immediately.

### Layer 4: Custom domain mapping

Users point DNS to an Uppe gateway/resolver.

Options:

- `CNAME status.example.com -> gateway.uppe.dev`
- TXT-based mapping to owner/page identity
- eventually multi-gateway failover

### Layer 5: Federation

Allow multiple gateways or peer-run gateways to serve the same content.

This is the practical path to "network-backed hosting".

## Non-goal for early phases

- Full browser-native libp2p page hosting
- inventing a custom DNS system before the page bundle and gateway model work

## Signed Audit and Verification Plan

## Audit Model

Every important action emits a signed event:

- `monitor.created`
- `monitor.updated`
- `monitor.deleted`
- `result.recorded`
- `result.verified`
- `status_page.published`
- `status_page.replica_stored`
- `peer.attestation`
- `admin_key.rotated`

## Verification Model

Verification is a separate signed record:

- original result is signed by source peer
- verifier peers issue signed attestations
- attestations reference the original event hash

This makes trust composable and inspectable.

## Storage Model

Recommended tables:

- `audit_events`
- `audit_attestations`
- `audit_replication`
- `status_page_revisions`
- `status_page_bundles`
- `status_page_replicas`

## API Model

Add query APIs for:

- audit event timeline
- event detail
- verification status
- peer attestation list
- replication state

## Phased Roadmap

## Phase 0: Stabilize Foundations

Target: 2-3 weeks

Deliverables:

- choose and document final architecture boundaries
- remove obsolete schema paths
- identify canonical model ownership
- replace placeholder/starter docs with accurate docs

Tasks:

1. Mark Rust migrations as the only DB source.
2. Archive/remove old Go migration SQL.
3. Remove duplicate or obsolete Go models.
4. Publish an architecture decision record for:
   - runtime ownership
   - schema ownership
   - API ownership
   - event ownership

Exit criteria:

- no ambiguity about which layer owns what
- no competing schema truth in active code paths

## Phase 1: Contract Discipline

Target: 2-3 weeks

Deliverables:

- generated type flow across Rust, Go, and TS
- compatibility checks in CI
- cleaned-up API contract surface

Tasks:

1. Audit all proto services and classify:
   - implemented
   - planned
   - remove/defer
2. Implement or hide missing services in nav/docs.
3. Add schema compatibility tests.
4. Add generated client usage rules for frontend.

Exit criteria:

- frontend never relies on undocumented handwritten API shapes
- Go and Astro consume generated contracts consistently

## Phase 2: Service Refactor

Target: 3-5 weeks

Deliverables:

- clear domain/runtime module boundaries in Rust
- reduced coupling between monitoring, P2P, and trust logic

Tasks:

1. Split orchestrator responsibilities into domain services.
2. Move P2P-specific logic behind explicit interfaces.
3. Move trust/signature verification into dedicated runtime modules.
4. Define message boundaries for:
   - monitor coordination
   - result replication
   - audit replication
   - status-page bundle replication

Exit criteria:

- core workflows can be described without crossing too many modules
- transport concerns are isolated from domain concerns

## Phase 3: Trust Hardening

Target: 2-4 weeks

Deliverables:

- real signature verification
- canonical serialization rules
- admin trust chain no longer uses placeholder checks

Tasks:

1. Replace placeholder verification in PeerUP.
2. Replace placeholder admin trust-chain validation.
3. Add deterministic payload canonicalization for signing.
4. Add unit and property tests around signature validation.

Exit criteria:

- no trust-critical path returns success from placeholders

## Phase 4: Status Page MVP

Target: 4-6 weeks

Deliverables:

- status page CRUD
- signed status page revisions
- replicated bundle storage
- gateway-served public pages

Tasks:

1. Implement `StatusPageService` in Go.
2. Add persistence tables for pages and revisions.
3. Define the signed page bundle format in Rust.
4. Add replication protocol for page bundles.
5. Build public page rendering against bundle data.

Exit criteria:

- a page remains accessible after the owner's node is offline

## Phase 5: Settings and Network APIs

Target: 2-3 weeks

Deliverables:

- `SettingsService`
- `NetworkService`
- live network map/settings pages

Tasks:

1. Implement settings storage/query APIs.
2. Implement peer/network stats APIs from replicated/shared DB state.
3. Replace placeholder frontend pages with live data.

Exit criteria:

- no major operator page is still mock-backed

## Phase 6: Audit MVP

Target: 4-6 weeks

Deliverables:

- signed event log
- peer verification attestations
- audit query APIs
- audit explorer UI

Tasks:

1. Implement canonical event envelope.
2. Persist signed events.
3. Replicate events across peers.
4. Add verification-attestation flows.
5. Build query endpoints and UI inspection views.

Exit criteria:

- an operator can inspect what happened and who verified it

## Immediate Backlog

These are the first concrete work items to start now:

1. Create `ARCHITECTURE.md` at repo root that defines ownership boundaries.
2. Remove or clearly deprecate:
- `apps/server/internal/db/migrations/001_initial_schema.sql`
- obsolete/duplicate Go model paths
3. Add a schema generation/verification script for `shared/database/schema.sql`.
4. Define the canonical event envelope in protobuf or a shared schema document.
5. Decide the status-page replication format:
- bundle manifest
- bundle assets
- summary snapshot
- signature format
6. Implement `SettingsService` and `StatusPageService` before adding more frontend pages.
7. Fix known correctness issues from the audit:
- DB path panic risk
- CORS policy
- incorrect hardcoded frontend API port
- ICMP exposure while unimplemented

## Suggested Repository Restructure

Proposed shape:

- `apps/service/src/domain/`
- `apps/service/src/runtime/`
- `apps/service/src/storage/`
- `apps/service/src/api_contract/` only if needed for shared internal mappings
- `apps/server/internal/services/`
- `apps/server/internal/readmodels/`
- `apps/server/internal/repositories/`
- `crates/peerup/src/transport/`
- `crates/peerup/src/replication/`
- `crates/peerup/src/verification/`

This is not mandatory immediately, but the direction should be toward domain-aligned modules rather than mixed responsibility folders.

## Delivery Strategy

Run the project in three coordinated tracks:

### Track A: Platform

Owns:

- schema discipline
- service boundaries
- Go API completion
- frontend contract consistency

### Track B: Distributed Runtime

Owns:

- PeerUP integration
- result replication
- status-page bundle replication
- network stats plumbing

### Track C: Trust and Audit

Owns:

- signing rules
- event envelope
- attestations
- trust chain hardening
- audit queries

## Success Criteria

The plan is working when:

- one schema source exists
- one API contract source exists
- no critical security path uses placeholders
- public status pages survive owner node downtime
- every important event is signed and inspectable
- frontend pages reflect actual backend capability

## Recommended Order of Execution

Do this in order:

1. architecture freeze
2. schema/API contract cleanup
3. service/module boundary cleanup
4. trust hardening
5. status-page MVP with replicated bundles
6. settings/network API completion
7. signed audit system and verification UX

## Bottom Line

The right move is not "add more features now". The right move is:

- fix ownership
- fix contracts
- fix trust boundaries
- then build resilient public pages and auditing on top of that stable base

If those foundations are done first, the later features you want are realistic. If not, public status pages, peer hosting, and signed audits will all be built on moving ground.
