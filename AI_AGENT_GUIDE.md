# AI Agent Guide

This guide tells agents how to work in the Uppe repository.

Read this first:

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [PROJECT_PLAN.md](PROJECT_PLAN.md)

## What Uppe Is

Uppe is a decentralized uptime monitoring platform with:

- a Rust monitoring node in [apps/service](apps/service)
- a Go API/query layer in [apps/server](apps/server)
- an Astro frontend in [apps/client](apps/client)
- reusable P2P/replication infrastructure in [crates/peerup](crates/peerup)

Core product goals:

- decentralized public monitoring
- private/internal monitoring modes
- resilient public status pages
- signed audit/event history with peer verification

## Ownership Rules

These rules are not optional.

### Rust owns

- monitor execution
- scheduling
- local result creation
- canonical signing/event generation
- P2P runtime integration
- replication behavior
- trust verification
- status-page bundle publication

### Go owns

- ConnectRPC APIs
- CRUD/query workflows for UI-facing resources
- aggregation/read models
- audit query APIs
- network/settings/status-page APIs

### Astro owns

- dashboard UX
- monitor/settings/status-page management UX
- public page rendering UX
- audit inspection UX

### PeerUP owns

- transport
- peer discovery
- DHT/gossip/request-response primitives
- generic replication mechanics

## Sources of Truth

### Database schema

Canonical source:

- [apps/service/src/database/migrations.rs](apps/service/src/database/migrations.rs)

Rule:

- Rust migrations are the only executable schema source.

Implications:

- Go does not run migrations.
- Obsolete Go migration files must not be treated as active truth.
- [shared/database/schema.sql](shared/database/schema.sql) is documentation, not the executable authority.

### API contracts

Canonical source:

- [apps/server/proto](apps/server/proto)

Rule:

- protobuf is the only API contract source.

### Trust/audit events

Canonical owner:

- Rust trust/domain layer

Rule:

- trust-sensitive actions must be backed by canonical signed events

## GitHub Project and Issue Workflow

Roadmap issues live in:

- milestone-backed GitHub issues
- project board: `Uppe Roadmap`

### Milestones

Use these milestones:

- `M1 Foundations`
- `M2 Service Boundaries`
- `M3 Status Pages MVP`
- `M4 Trust and Audit`

### Labels

Use these labels consistently:

- `roadmap`
- `architecture`
- `api`
- `frontend`
- `p2p`
- `trust`
- `status-pages`
- `audit`
- `backend`
- `rust`
- `go`
- `astro`

### Track field on the project board

Use one of:

- `Platform`
- `Distributed Runtime`
- `Trust and Audit`

Suggested mapping:

- schema/API/frontend integration work -> `Platform`
- replication, peer networking, status-page bundle distribution -> `Distributed Runtime`
- signature verification, trust chains, attestations, audit events -> `Trust and Audit`

### How to work an issue

1. Read the issue body and acceptance criteria.
2. Check the milestone and labels.
3. Confirm the work matches the ownership rules above.
4. Update docs/contracts before or alongside code if the issue changes architecture.
5. Keep implementation aligned with the roadmap rather than inventing parallel paths.

### When opening new issues

New issues should include:

- summary
- scope
- why
- deliverables
- acceptance criteria

Do not open vague TODO issues.

## Rules for Agents

### Do

- keep Rust as the schema owner
- keep protobuf as the API owner
- reduce duplicate models
- remove stale paths when they conflict with active architecture
- prefer tightening boundaries before adding new features

### Do not

- add new executable schema definitions outside Rust migrations
- add frontend pages that pretend a mock-only backend exists
- leave placeholder verification in trust-critical paths
- mix transport logic into monitoring/business logic unnecessarily
- create new "source of truth" documents that contradict the architecture docs

## Current Execution Order

Work should generally follow this sequence:

1. foundations
2. schema/API cleanup
3. service boundary cleanup
4. trust hardening
5. status-page bundle MVP
6. network/settings API completion
7. audit and attestation system

## Immediate Priority

The first implementation priority is:

- make Rust migrations the only active DB schema source

That includes:

- removing or deprecating obsolete Go migration paths
- fixing docs that still imply Go runs migrations
- keeping shared schema docs clearly documented as non-authoritative

## File References

Core docs:

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [PROJECT_PLAN.md](PROJECT_PLAN.md)

Core code:

- [apps/service/src/database/migrations.rs](apps/service/src/database/migrations.rs)
- [apps/server/internal/db/libsql/libsql.go](apps/server/internal/db/libsql/libsql.go)
- [apps/server/proto](apps/server/proto)
- [crates/peerup](crates/peerup)
