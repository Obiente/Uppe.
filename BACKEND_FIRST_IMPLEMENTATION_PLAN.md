# Backend-First Implementation Plan

This document defines the next implementation order for Uppe after the current
API/frontend stabilization work. The goal is to stop building outward from UI
assumptions and instead build upward from a stable backend domain model.

## Core Rule

Implementation order is:

1. Rust domain/runtime
2. Rust-owned schema and signed event model
3. Go API/read models
4. Astro frontend

No new frontend feature should be added unless the Rust runtime and the API
contract already support it.

## Ownership

Rust owns:

- monitor execution
- P2P protocols and replication
- trust and signature verification
- audit/event emission
- canonical writes to the shared database
- status-page publish/revision generation

Go owns:

- query APIs
- CRUD APIs for operator workflows
- read models and projections derived from Rust-owned state
- public read endpoints for dashboards and status pages

Astro owns:

- operator UX
- public status page UX
- rendering and interaction only

## Immediate Program

### Milestone 1: Canonical Signed Event Envelope

Deliverables:

- one canonical event envelope for trust-sensitive actions
- deterministic serialization rules
- event signing and verification helpers
- append-only event storage in Rust

Events that must use the envelope:

- monitor created
- monitor updated
- monitor deleted
- monitor result recorded
- peer result replicated
- peer attestation recorded
- status page published
- status page revised
- trust-chain updated
- capability granted or revoked

Exit criteria:

- no trust-sensitive mutation is represented only as mutable row state
- all new trust-sensitive writes can emit an event envelope

### Milestone 2: Trust Hardening

Deliverables:

- remove placeholder signature-verification paths
- define attestation semantics clearly
- define verification quorum semantics
- split trust validation from orchestration and UI concerns

Implementation focus:

- `apps/service/src/orchestrator/admin_trust.rs`
- `crates/peerup/src/trust`
- any message verification path that currently accepts unverifiable data

Exit criteria:

- no production trust path succeeds via placeholder checks
- invalid signatures fail closed
- trust decisions become inspectable from signed data

### Milestone 3: Public Status Page Backend Model

The public page should stop deriving rich UI state from a CRUD message.

Deliverables:

- dedicated backend read model for public status pages
- status page revision/publish model in Rust
- monitor summaries for public pages
- recent incident summaries
- uptime history and global ping samples

Recommended model split:

- `status_pages`:
  operator-owned page identity and config
- `status_page_revisions`:
  immutable published page bundles or metadata pointers
- `status_page_monitors`:
  monitor inclusion and ordering
- `status_page_read_models`:
  query-optimized public projection

Exit criteria:

- public page routes consume a public read model, not CRUD state plus guesses
- status page data can be replicated later without redesigning the model

### Milestone 4: Runtime Separation

Rust should be split by runtime responsibility, not by mixed orchestration files.

Target shape:

- `runtime/monitoring`
- `runtime/p2p`
- `runtime/trust`
- `runtime/status_pages`
- `domain/monitors`
- `domain/results`
- `domain/audit`
- `domain/status_pages`

Rules:

- monitoring execution must not own trust policy
- trust runtime must not own UI-facing read models
- P2P transport must not be mixed into monitor scheduling code

Exit criteria:

- module boundaries match domain responsibilities
- transport, monitoring, and trust can evolve independently

### Milestone 5: Decentralized Authentication Foundation

Authentication must be built as an extension of trust and identity, not as a
centralized web login bolted on later.

Deliverables:

- node identity model
- challenge-response auth design
- signed capability grants
- key rotation model
- operator auth strategy for UI/API access

Detailed spec is in
[DECENTRALIZED_AUTH_AND_SIGNED_EVENTS.md](DECENTRALIZED_AUTH_AND_SIGNED_EVENTS.md).

Exit criteria:

- operator auth and peer auth share the same cryptographic foundations
- capabilities are delegable without central session state

## Data Flow Rules

### Monitoring

1. Rust executes checks.
2. Rust writes canonical result rows.
3. Rust emits signed result events.
4. Rust replicates result events and/or derived records to peers.
5. Go exposes read/query APIs over canonical and projected state.
6. Astro renders from Go contracts only.

### Status Pages

1. Operator configures page through API.
2. Rust creates or updates publishable page state.
3. Rust emits signed publish/revision events.
4. Go exposes operator CRUD and public read models.
5. Astro renders the operator page and public page from those read models.

### Trust and Audit

1. Rust receives or creates signed events.
2. Rust verifies signatures and policy.
3. Rust stores event envelopes and attestations.
4. Go exposes query APIs for audits, trust state, and verification summaries.
5. Astro renders trust/audit views from those APIs.

## Rules For New Work

Before adding any feature:

1. Is the Rust domain model defined?
2. Is the schema owned by Rust migrations?
3. Is there a signed event if the action is trust-sensitive?
4. Is the API contract generated from a canonical source?
5. Is the frontend only consuming backend-supported data?

If any answer is no, the feature is not ready for frontend-first work.

## Next Tickets To Execute

Recommended next sequence:

1. Define `SignedEventEnvelope` in Rust.
2. Add canonical serialization and signature helpers.
3. Add audit event persistence tables.
4. Refactor placeholder trust verification out of production paths.
5. Define `PublicStatusPageView` API contract.
6. Implement Rust-side status page revision generation.
7. Expose Go read model endpoints for public status pages.
8. Only then expand frontend status-page UX further.

## What To Avoid

Do not:

- add new frontend pages that fabricate missing backend state
- hand-maintain parallel models across Rust, Go, and Astro
- treat trust verification as a UI concern
- treat decentralized auth as a later unrelated subsystem
- keep extending CRUD `StatusPage` to behave like a public page read model

## Definition Of Success

The backend-first direction is working when:

- Rust is the clear execution and trust authority
- Go is a thin, reliable query and operator API layer
- Astro no longer needs mock-derived business logic
- public pages, audits, and auth all sit on the same identity and event model
