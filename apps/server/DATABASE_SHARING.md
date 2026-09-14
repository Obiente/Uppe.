# Database Sharing

## Current Rule

The Rust service owns the database schema.

Canonical schema source:

- [apps/service/src/database/migrations.rs](apps/service/src/database/migrations.rs)

Go API behavior:

- verifies schema compatibility on startup
- reads and writes user-facing data against the Rust-owned schema
- does not run migrations
- does not define a competing executable schema

Schema documentation:

- [shared/database/schema.sql](shared/database/schema.sql)

That file is documentation only. It must not be treated as the executable authority over the Rust migrations.

## Shared Database Model

The intended deployment model is:

- Rust service and Go API point at the same SQLite/LibSQL database
- Rust initializes and migrates the schema
- Go waits for the schema to exist and verifies compatibility
- frontend talks only to the Go API

## Startup Order

1. Start the Rust service first.
2. Let the Rust service create or migrate the schema.
3. Start the Go API.
4. The Go API verifies schema compatibility and serves frontend/API clients.

## Why

This keeps one schema owner and avoids:

- migration drift
- conflicting table definitions
- multiple executable schema sources

## What Not To Do

Do not:

- run migrations from the Go server
- reintroduce an active Go-owned migration directory
- treat an old SQL snapshot as the real schema when Rust migrations differ

## Follow-Up Work

The longer-term cleanup is:

- remove obsolete Go migration helpers entirely
- generate schema docs from Rust migration state or keep them explicitly documentation-only
