#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use libsql::{Connection, params};
use std::sync::Arc;
use uuid::Uuid;

use super::models::{Monitor, MonitorResult, PeerResult};
use crate::monitoring::types::CheckResult;
use crate::pool::LibsqlPool;

/// Database trait for abstracting database operations
#[async_trait]
pub trait Database: Send + Sync {
    /// Get all enabled monitors
    async fn get_enabled_monitors(&self) -> Result<Vec<Monitor>>;

    /// Get a monitor by UUID
    async fn get_monitor_by_uuid(&self, uuid: Uuid) -> Result<Option<Monitor>>;

    /// Save a monitor
    async fn save_monitor(&self, monitor: &Monitor) -> Result<i64>;

    /// Delete a monitor by UUID
    async fn delete_monitor(&self, uuid: Uuid) -> Result<()>;

    /// Save a monitoring result
    async fn save_result(&self, result: &CheckResult) -> Result<i64>;

    /// Save a peer result (result from another peer)
    async fn save_peer_result(&self, result: &PeerResult) -> Result<i64>;

    /// Get recent results for a monitor
    async fn get_recent_results(&self, monitor_uuid: Uuid, limit: usize) -> Result<Vec<MonitorResult>>;

    /// Get peer results for a monitor
    async fn get_peer_results(&self, monitor_uuid: Uuid, limit: usize) -> Result<Vec<PeerResult>>;
}

/// LibSQL database implementation
pub struct DatabaseImpl {
    pool: LibsqlPool,
}

impl DatabaseImpl {
    /// Create a new database instance from a pool
    pub fn new_from_pool(pool: LibsqlPool) -> Self {
        Self { pool }
    }

    /// Create a new database instance (legacy, for backward compatibility)
    #[allow(dead_code)]
    pub fn new(_connection: Arc<Connection>) -> Self {
        // This is a compatibility shim - in practice we should always use new_from_pool
        // For now, this will panic if called - the proper way is to use new_from_pool
        panic!("Use DatabaseImpl::new_from_pool instead");
    }

    /// Get a connection from the pool
    async fn get_conn(&self) -> Result<deadpool::managed::Object<crate::pool::LibsqlManager>> {
        Ok(self.pool.get().await?)
    }
}

#[async_trait]
impl Database for DatabaseImpl {
    async fn get_enabled_monitors(&self) -> Result<Vec<Monitor>> {
        let conn = self.get_conn().await?;
        let mut stmt = conn
            .prepare("SELECT id, uuid, name, target, check_type, interval_seconds, timeout_seconds, enabled, created_at, updated_at FROM monitors WHERE enabled = 1")
            .await?;

        let mut rows = stmt.query(()).await?;
        let mut monitors = Vec::new();

        while let Some(row) = rows.next().await? {
            let uuid_str: String = row.get(1)?;
            let created_at: i64 = row.get(8)?;
            let updated_at: i64 = row.get(9)?;

            monitors.push(Monitor {
                id: Some(row.get(0)?),
                uuid: Uuid::parse_str(&uuid_str)?,
                name: row.get(2)?,
                target: row.get(3)?,
                check_type: row.get(4)?,
                interval_seconds: row.get::<i64>(5)? as u64,
                timeout_seconds: row.get::<i64>(6)? as u64,
                enabled: row.get::<i64>(7)? != 0,
                created_at: Monitor::i64_to_timestamp(created_at),
                updated_at: Monitor::i64_to_timestamp(updated_at),
            });
        }

        Ok(monitors)
    }

    async fn get_monitor_by_uuid(&self, uuid: Uuid) -> Result<Option<Monitor>> {
        let conn = self.get_conn().await?;
        let mut stmt = conn
            .prepare("SELECT id, uuid, name, target, check_type, interval_seconds, timeout_seconds, enabled, created_at, updated_at FROM monitors WHERE uuid = ?")
            .await?;

        let mut rows = stmt.query(params![uuid.to_string()]).await?;

        if let Some(row) = rows.next().await? {
            let uuid_str: String = row.get(1)?;
            let created_at: i64 = row.get(8)?;
            let updated_at: i64 = row.get(9)?;

            Ok(Some(Monitor {
                id: Some(row.get(0)?),
                uuid: Uuid::parse_str(&uuid_str)?,
                name: row.get(2)?,
                target: row.get(3)?,
                check_type: row.get(4)?,
                interval_seconds: row.get::<i64>(5)? as u64,
                timeout_seconds: row.get::<i64>(6)? as u64,
                enabled: row.get::<i64>(7)? != 0,
                created_at: Monitor::i64_to_timestamp(created_at),
                updated_at: Monitor::i64_to_timestamp(updated_at),
            }))
        } else {
            Ok(None)
        }
    }

    async fn save_monitor(&self, monitor: &Monitor) -> Result<i64> {
        let conn = self.get_conn().await?;
        let created_at = Monitor::timestamp_to_i64(monitor.created_at);
        let updated_at = Monitor::timestamp_to_i64(monitor.updated_at);

        if let Some(id) = monitor.id {
            // Update existing monitor
            conn.execute(
                "UPDATE monitors SET name = ?, target = ?, check_type = ?, interval_seconds = ?, timeout_seconds = ?, enabled = ?, updated_at = ? WHERE id = ?",
                params![
                    monitor.name.clone(),
                    monitor.target.clone(),
                    monitor.check_type.clone(),
                    monitor.interval_seconds as i64,
                    monitor.timeout_seconds as i64,
                    if monitor.enabled { 1 } else { 0 },
                    updated_at,
                    id
                ],
            ).await?;
            Ok(id)
        } else {
            // Insert new monitor
            conn.execute(
                "INSERT INTO monitors (uuid, name, target, check_type, interval_seconds, timeout_seconds, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    monitor.uuid.to_string(),
                    monitor.name.clone(),
                    monitor.target.clone(),
                    monitor.check_type.clone(),
                    monitor.interval_seconds as i64,
                    monitor.timeout_seconds as i64,
                    if monitor.enabled { 1 } else { 0 },
                    created_at,
                    updated_at
                ],
            ).await?;

            Ok(conn.last_insert_rowid())
        }
    }

    async fn delete_monitor(&self, uuid: Uuid) -> Result<()> {
        let conn = self.get_conn().await?;

        // Delete the monitor; related rows will be removed via ON DELETE CASCADE
        conn.execute(
            "DELETE FROM monitors WHERE uuid = ?",
            params![uuid.to_string()],
        ).await?;
        Ok(())
    }

    async fn save_result(&self, result: &CheckResult) -> Result<i64> {
        let conn = self.get_conn().await?;
        let timestamp = Monitor::timestamp_to_i64(result.timestamp);
        let created_at = Monitor::timestamp_to_i64(std::time::SystemTime::now());
        let location = crate::location::get_location();

        conn.execute(
            "INSERT INTO monitor_results (monitor_uuid, timestamp, status, latency_ms, status_code, error_message, peer_id, signature, created_at, city, country, region) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                result.monitor_id.to_string(),
                timestamp,
                result.status.to_string(),
                result.latency_ms.map(|v| v as i64),
                result.status_code.map(|v| v as i64),
                result.error_message.clone(),
                result.peer_id.clone(),
                result.signature.clone(),
                created_at,
                location.city,
                location.country,
                location.region
            ],
        ).await?;

        Ok(conn.last_insert_rowid())
    }

    async fn save_peer_result(&self, result: &PeerResult) -> Result<i64> {
        let conn = self.get_conn().await?;
        let timestamp = Monitor::timestamp_to_i64(result.timestamp);
        let created_at = Monitor::timestamp_to_i64(result.created_at);

        conn.execute(
            "INSERT INTO peer_results (monitor_uuid, timestamp, status, latency_ms, status_code, error_message, peer_id, signature, verified, created_at, city, country, region) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                result.monitor_uuid.to_string(),
                timestamp,
                result.status.to_string(),
                result.latency_ms.map(|v| v as i64),
                result.status_code.map(|v| v as i64),
                result.error_message.clone(),
                result.peer_id.clone(),
                result.signature.clone(),
                if result.verified { 1 } else { 0 },
                created_at,
                result.city.clone(),
                result.country.clone(),
                result.region.clone()
            ],
        ).await?;

        Ok(conn.last_insert_rowid())
    }

    async fn get_recent_results(&self, monitor_uuid: Uuid, limit: usize) -> Result<Vec<MonitorResult>> {
        let conn = self.get_conn().await?;
        let mut stmt = conn
            .prepare("SELECT id, monitor_uuid, timestamp, status, latency_ms, status_code, error_message, peer_id, signature, created_at, city, country, region FROM monitor_results WHERE monitor_uuid = ? ORDER BY timestamp DESC LIMIT ?")
            .await?;

        let mut rows = stmt.query(params![monitor_uuid.to_string(), limit as i64]).await?;
        let mut results = Vec::new();

        while let Some(row) = rows.next().await? {
            let monitor_uuid_str: String = row.get(1)?;
            let status_str: String = row.get(3)?;
            let timestamp: i64 = row.get(2)?;
            let created_at: i64 = row.get(9)?;

            results.push(MonitorResult {
                id: Some(row.get(0)?),
                monitor_uuid: Uuid::parse_str(&monitor_uuid_str)?,
                timestamp: Monitor::i64_to_timestamp(timestamp),
                status: match status_str.as_str() {
                    "up" => crate::monitoring::types::MonitorStatus::Up,
                    "down" => crate::monitoring::types::MonitorStatus::Down,
                    "degraded" => crate::monitoring::types::MonitorStatus::Degraded,
                    _ => crate::monitoring::types::MonitorStatus::Unknown,
                },
                latency_ms: row.get::<Option<i64>>(4)?.map(|v| v as u64),
                status_code: row.get::<Option<i64>>(5)?.map(|v| v as u16),
                error_message: row.get(6)?,
                peer_id: row.get(7)?,
                signature: row.get(8)?,
                created_at: Monitor::i64_to_timestamp(created_at),
                city: row.get(10)?,
                country: row.get(11)?,
                region: row.get(12)?,
            });
        }

        Ok(results)
    }

    async fn get_peer_results(&self, monitor_uuid: Uuid, limit: usize) -> Result<Vec<PeerResult>> {
        let conn = self.get_conn().await?;
        let mut stmt = conn
            .prepare("SELECT id, monitor_uuid, timestamp, status, latency_ms, status_code, error_message, peer_id, signature, verified, created_at, city, country, region FROM peer_results WHERE monitor_uuid = ? ORDER BY timestamp DESC LIMIT ?")
            .await?;

        let mut rows = stmt.query(params![monitor_uuid.to_string(), limit as i64]).await?;
        let mut results = Vec::new();

        while let Some(row) = rows.next().await? {
            let monitor_uuid_str: String = row.get(1)?;
            let status_str: String = row.get(3)?;
            let timestamp: i64 = row.get(2)?;
            let created_at: i64 = row.get(10)?;

            results.push(PeerResult {
                id: Some(row.get(0)?),
                monitor_uuid: Uuid::parse_str(&monitor_uuid_str)?,
                timestamp: Monitor::i64_to_timestamp(timestamp),
                status: match status_str.as_str() {
                    "up" => crate::monitoring::types::MonitorStatus::Up,
                    "down" => crate::monitoring::types::MonitorStatus::Down,
                    "degraded" => crate::monitoring::types::MonitorStatus::Degraded,
                    _ => crate::monitoring::types::MonitorStatus::Unknown,
                },
                latency_ms: row.get::<Option<i64>>(4)?.map(|v| v as u64),
                status_code: row.get::<Option<i64>>(5)?.map(|v| v as u16),
                error_message: row.get(6)?,
                peer_id: row.get(7)?,
                signature: row.get(8)?,
                verified: row.get::<i64>(9)? != 0,
                created_at: Monitor::i64_to_timestamp(created_at),
                city: row.get(11).ok(),
                country: row.get(12).ok(),
                region: row.get(13).ok(),
            });
        }

        Ok(results)
    }
}
