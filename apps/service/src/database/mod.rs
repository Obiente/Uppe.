/// Database abstraction layer
///
/// This module provides a unified interface for database operations,
/// supporting both LibSQL (SQLite) and PostgreSQL backends.

pub mod repository;
pub mod migrations;
pub mod models;

pub use repository::{Database, DatabaseImpl};

use anyhow::Result;

/// Initialize database with schema
pub async fn initialize_database(conn: &libsql::Connection) -> Result<()> {
    migrations::run_migrations(conn).await
}
