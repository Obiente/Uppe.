# Decentralized Authentication And Signed Events

This document defines the intended trust model for Uppe:

- identity is cryptographic
- trust-sensitive actions become signed events
- authentication is challenge-response, not password/session centric
- authorization is capability-based

## Goals

Uppe needs:

- stable node identity
- decentralized peer authentication
- operator authentication without a central auth server
- auditable authorization changes
- signed publish and verification flows for status pages and monitoring

## Non-Goals

This design does not require:

- a centralized identity provider
- long-lived bearer sessions as the core trust primitive
- browser-native peer transport for website hosting

## Identity Model

### Node Identity

Each node has a long-lived Ed25519 keypair.

The public key is the root identity.
The node ID is a stable derivation of that public key.

The same root identity should anchor:

- signed event envelopes
- peer authentication
- capability delegation
- status-page publish ownership

### Operator Identity

There are two operator modes:

1. Local operator
2. Delegated operator

Local operator:

- authenticates directly to their node
- proves control via challenge-response signature or local secure mechanism

Delegated operator:

- receives a signed capability grant from the node owner or admin authority
- can act only within the granted scope

### Optional Human-Friendly Layer

Later, WebAuthn can be added as a human-friendly operator login mechanism.
That should be a wrapper around cryptographic identity, not a replacement for it.

## Authentication Model

### Peer Authentication

Peer-to-peer authentication uses:

- transport-level authenticated channels where available
- signed application-layer messages always

Rules:

- transport authentication alone is not sufficient for trust-sensitive state
- trust-sensitive messages must carry signed payloads
- receivers verify signature, key identity, and policy scope

### Operator Authentication

Operator login should use challenge-response:

1. Client requests nonce/challenge.
2. Node returns a short-lived challenge with expiration and audience.
3. Operator signs the challenge using the authorized key.
4. Node verifies the signature and issues a short-lived capability-bound session
   token or signed access token.

Short-lived tokens may exist for convenience, but they are derived from signed
proof and should remain revocable and scope-limited.

## Authorization Model

Authorization should be capability-based.

Examples of capabilities:

- `monitors:create`
- `monitors:update`
- `monitors:delete`
- `status_pages:create`
- `status_pages:update`
- `status_pages:publish`
- `peers:approve`
- `trust:rotate_keys`
- `audit:read`

Each capability grant should be:

- signed
- scoped
- time-bounded when possible
- revocable via later signed event

## Signed Event Envelope

Every trust-sensitive action should become a signed event.

## Envelope Shape

Recommended logical fields:

- `event_id`
- `event_type`
- `schema_version`
- `created_at`
- `actor_id`
- `actor_public_key`
- `resource_type`
- `resource_id`
- `parent_event_id` or `previous_event_hash`
- `payload`
- `payload_hash`
- `signature`

Optional:

- `capability_id`
- `delegated_by`
- `expires_at`
- `context`

## Envelope Rules

Rules:

- payload serialization must be canonical
- signature is over canonical bytes, not ad hoc JSON rendering
- payloads are immutable once signed
- later changes produce new events, not in-place mutation
- verification results are separate attestations, not edits to the source event

## Event Categories

At minimum:

- monitor lifecycle events
- monitor result events
- peer replication events
- peer attestation events
- status page publish/revision events
- capability grant/revoke events
- trust-chain rotation events
- admin policy change events

## Attestations

Attestations are separate signed events that refer to another event.

Examples:

- peer verified result
- peer rejected signature
- peer confirmed status page revision hash
- admin approved capability grant

Recommended fields:

- `attestation_id`
- `subject_event_id`
- `attestor_id`
- `decision`
- `reason`
- `signature`
- `created_at`

## Storage Model

Recommended tables:

- `audit_events`
- `audit_attestations`
- `capability_grants`
- `capability_revocations`
- `identity_keys`
- `identity_rotations`

Important property:

mutable read models can exist, but the event store is the canonical history.

## Key Rotation

Key rotation must be auditable.

Recommended model:

1. Current key signs a rotation event naming the next key.
2. Next key countersigns or becomes valid at a declared point.
3. Peers verify the chain.
4. Revocation status is represented explicitly by later signed events.

This gives:

- inspectable trust lineage
- recovery without hidden state
- compatibility with distributed verification

## Status Pages In This Model

Status pages should also fit the same trust system.

Owner signs:

- page creation
- page revisions
- publish pointers
- domain bindings when supported

Peers can attest:

- revision availability
- revision hash validity
- observed monitor summary agreement

This makes public pages:

- attributable
- verifiable
- replicable

## Recommended Token Model

If API tokens are needed, they should be:

- short-lived
- signed
- audience-bound
- capability-bound

Prefer:

- signed access token derived from challenge-response

Avoid:

- broad permanent bearer tokens without audit linkage

## Backend Implementation Order

1. Define canonical event envelope types in Rust.
2. Define canonical serialization rules.
3. Implement signing and verification helpers.
4. Persist events and attestations.
5. Make trust-sensitive backend actions emit events.
6. Add capability grants and revocations.
7. Add operator challenge-response auth.
8. Add Go APIs that surface read models of identities, grants, and audits.
9. Add Astro flows on top of those APIs.

## API Design Implications

The Go API should eventually expose:

- `AuthService`
- `CapabilityService`
- `AuditService`
- `PublicStatusPageService` or equivalent public read service

These should query Rust-owned state, not invent their own authority.

## Security Constraints

Hard requirements:

- invalid signatures fail closed
- expired challenges are rejected
- capability scope is checked on every write path
- key rotation is chain-validated
- all trust-sensitive changes are auditable

## Practical First Version

Version 1 can be simpler:

- one node root keypair
- challenge-response operator auth
- signed event envelope for monitor/status-page/trust changes
- signed capability grants for operator roles
- peer attestations for replicated results

That is enough to build:

- decentralized operator auth
- inspectable trust history
- verifiable peer coordination

without waiting for a fully federated identity ecosystem.
