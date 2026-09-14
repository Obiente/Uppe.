# Database Sharing

## Current Rule

The Rust service owns the database schema.

Canonical schema source:

- [Rust migrations](../service/src/database/migrations.rs)

Go API behavior:

- verifies schema compatibility on startup
- reads and writes user-facing data against the Rust-owned schema
- does not run migrations
- does not define a competing executable schema

Schema documentation:

- [Historical SQL snapshot](../../shared/database/schema.sql)

That file is an older documentation snapshot, not the current schema. Rust migrations create schema 7, and Go requires that exact version and its expected tables at startup. The integration tests exercise Go queries against a database created by the real Rust binary.

## Shared Database Model

The intended deployment model is:

- Rust service and Go API point at the same SQLite/LibSQL database
- Rust initializes and migrates the schema
- Go waits for the schema to exist and verifies compatibility
- Astro talks to the Go API; browser mutations use Astro's authenticated same-origin proxy

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

Generate schema documentation from Rust migration state when maintaining a current SQL snapshot becomes useful. Do not edit the historical snapshot to introduce schema changes.
