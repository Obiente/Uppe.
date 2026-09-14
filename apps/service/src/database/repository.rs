#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use libsql::params;
use uuid::Uuid;

use super::models::{
    AuditAttestation, AuditEvent, Monitor, MonitorResult, NetworkStats, Peer, PeerResult,
};
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
    async fn save_peer_result(
        &self,
        result: &PeerResult,
        target: &str,
        receipt: &str,
    ) -> Result<super::peer_storage::SaveOutcome>;

    /// Get recent results for a monitor
    async fn get_recent_results(
        &self,
        monitor_uuid: Uuid,
        limit: usize,
    ) -> Result<Vec<MonitorResult>>;

    /// Get peer results for a monitor
    async fn get_peer_results(&self, monitor_uuid: Uuid, limit: usize) -> Result<Vec<PeerResult>>;

    /// Upsert peer metadata
    async fn upsert_peer(&self, peer: &Peer) -> Result<()>;

    /// Mark peer offline
    async fn mark_peer_offline(&self, peer_id: &str, now: std::time::SystemTime) -> Result<()>;

    /// Get peer by ID
    async fn get_peer_by_id(&self, peer_id: &str) -> Result<Option<Peer>>;

    /// List known peers (most recent first)
    async fn list_peers(&self, limit: usize) -> Result<Vec<Peer>>;

    /// Insert network stats snapshot
    async fn insert_network_stats(&self, stats: &NetworkStats) -> Result<i64>;

    /// Get latest network stats
    async fn get_latest_network_stats(&self) -> Result<Option<NetworkStats>>;

    /// Query peer results within a time range
    async fn query_peer_results(
        &self,
        since_timestamp: i64,
        monitor_uuid: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<PeerResult>>;

    /// Mark peer results as synced (for cleanup)
    async fn mark_peer_results_synced(
        &self,
        source_peer_id: &str,
        until_timestamp: i64,
    ) -> Result<()>;

    /// Clean up expired peer results based on retention policy
    async fn cleanup_expired_peer_results(&self) -> Result<u64>;

    /// Delete local history using the monitor visibility policy.
    async fn cleanup_local_results(&self, private_cutoff: i64, public_cutoff: i64) -> Result<u64>;

    // ===== Public Monitor Group Methods =====

    /// Get public monitor group by domain
    async fn get_public_monitor_group(
        &self,
        domain: &str,
    ) -> Result<Option<peerup::distributed::PublicMonitorGroup>>;

    /// Save or update public monitor group
    async fn save_public_monitor_group(
        &self,
        group: &peerup::distributed::PublicMonitorGroup,
    ) -> Result<()>;

    /// Get orchestration votes for a domain
    async fn get_orchestration_votes(
        &self,
        domain: &str,
    ) -> Result<Vec<peerup::distributed::OrchestrationVote>>;

    /// Save orchestration vote
    async fn save_orchestration_vote(
        &self,
        vote: &peerup::distributed::OrchestrationVote,
    ) -> Result<()>;

    /// Get a setting value by key
    async fn get_setting(&self, key: &str) -> Result<Option<String>>;

    /// Set a setting value by key
    async fn set_setting(&self, key: &str, value: &str) -> Result<()>;

    /// Persist a signed audit event.
    async fn save_audit_event(&self, event: &AuditEvent) -> Result<i64>;

    /// Fetch a signed audit event by its UUID.
    async fn get_audit_event(&self, event_uuid: Uuid) -> Result<Option<AuditEvent>>;

    /// List recent signed audit events.
    async fn list_audit_events(&self, limit: usize) -> Result<Vec<AuditEvent>>;

    /// Persist a signed audit attestation.
    async fn save_audit_attestation(&self, attestation: &AuditAttestation) -> Result<i64>;

    /// List attestations associated with an audit event.
    async fn list_audit_attestations(
        &self,
        subject_event_uuid: Uuid,
    ) -> Result<Vec<AuditAttestation>>;
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

    /// Get a connection from the pool
    async fn get_conn(&self) -> Result<deadpool::managed::Object<crate::pool::LibsqlManager>> {
        Ok(self.pool.get().await?)
    }

    fn map_audit_event_row(row: &libsql::Row) -> Result<AuditEvent> {
        let event_uuid: String = row.get(1)?;
        let created_at: i64 = row.get(4)?;
        let parent_event_uuid: Option<String> = row.get(9)?;
        let expires_at: Option<i64> = row.get(14)?;

        Ok(AuditEvent {
            id: Some(row.get(0)?),
            event_uuid: Uuid::parse_str(&event_uuid)?,
            event_type: row.get(2)?,
            schema_version: row.get(3)?,
            created_at: Monitor::i64_to_timestamp(created_at),
            actor_id: row.get(5)?,
            actor_public_key: row.get(6)?,
            resource_type: row.get(7)?,
            resource_id: row.get(8)?,
            parent_event_uuid: parent_event_uuid.as_deref().map(Uuid::parse_str).transpose()?,
            payload_json: row.get(10)?,
            payload_hash: row.get(11)?,
            capability_id: row.get(12)?,
            delegated_by: row.get(13)?,
            expires_at: expires_at.map(Monitor::i64_to_timestamp),
            context_json: row.get(15)?,
            signature: row.get(16)?,
        })
    }

    fn map_audit_attestation_row(row: &libsql::Row) -> Result<AuditAttestation> {
        let attestation_uuid: String = row.get(1)?;
        let subject_event_uuid: String = row.get(2)?;
        let created_at: i64 = row.get(7)?;

        Ok(AuditAttestation {
            id: Some(row.get(0)?),
            attestation_uuid: Uuid::parse_str(&attestation_uuid)?,
            subject_event_uuid: Uuid::parse_str(&subject_event_uuid)?,
            attestor_id: row.get(3)?,
            attestor_public_key: row.get(4)?,
            decision: row.get(5)?,
            reason: row.get(6)?,
            created_at: Monitor::i64_to_timestamp(created_at),
            signature: row.get(8)?,
        })
    }
}

#[async_trait]
impl Database for DatabaseImpl {
    async fn get_enabled_monitors(&self) -> Result<Vec<Monitor>> {
        let conn = self.get_conn().await?;
        let stmt = conn
            .prepare(
                "SELECT id, uuid, name, target, check_type, interval_seconds, timeout_seconds, \
                 enabled, created_at, updated_at, visibility, public_domain, public_display_name, \
                 owner_peer_id FROM monitors WHERE enabled = 1",
            )
            .await?;

        let mut rows = stmt.query(()).await?;
        let mut monitors = Vec::new();

        while let Some(row) = rows.next().await? {
            let uuid_str: String = row.get(1)?;
            let created_at: i64 = row.get(8)?;
            let updated_at: i64 = row.get(9)?;
            let visibility_str: String = row.get(10)?;

            let visibility = match visibility_str.as_str() {
                "Public" => crate::database::models::MonitorVisibility::Public,
                "Internal" => crate::database::models::MonitorVisibility::Internal,
                _ => crate::database::models::MonitorVisibility::Private,
            };

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
                visibility,
                public_domain: row.get(11)?,
                public_display_name: row.get(12)?,
                owner_peer_id: row.get(13)?,
            });
        }

        Ok(monitors)
    }

    async fn get_monitor_by_uuid(&self, uuid: Uuid) -> Result<Option<Monitor>> {
        let conn = self.get_conn().await?;
        let stmt = conn
            .prepare(
                "SELECT id, uuid, name, target, check_type, interval_seconds, timeout_seconds, \
                 enabled, created_at, updated_at, visibility, public_domain, public_display_name, \
                 owner_peer_id FROM monitors WHERE uuid = ?",
            )
            .await?;

        let mut rows = stmt.query(params![uuid.to_string()]).await?;

        if let Some(row) = rows.next().await? {
            let uuid_str: String = row.get(1)?;
            let created_at: i64 = row.get(8)?;
            let updated_at: i64 = row.get(9)?;
            let visibility_str: String = row.get(10)?;

            let visibility = match visibility_str.as_str() {
                "Public" => crate::database::models::MonitorVisibility::Public,
                "Internal" => crate::database::models::MonitorVisibility::Internal,
                _ => crate::database::models::MonitorVisibility::Private,
            };

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
                visibility,
                public_domain: row.get(11)?,
                public_display_name: row.get(12)?,
                owner_peer_id: row.get(13)?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn save_monitor(&self, monitor: &Monitor) -> Result<i64> {
        let conn = self.get_conn().await?;
        let created_at = Monitor::timestamp_to_i64(monitor.created_at);
        let updated_at = Monitor::timestamp_to_i64(monitor.updated_at);

        // Convert visibility to string
        let visibility_str = match monitor.visibility {
            crate::database::models::MonitorVisibility::Public => "Public",
            crate::database::models::MonitorVisibility::Private => "Private",
            crate::database::models::MonitorVisibility::Internal => "Internal",
        };

        if let Some(id) = monitor.id {
            // Update existing monitor
            conn.execute(
                "UPDATE monitors SET name = ?, target = ?, check_type = ?, interval_seconds = ?, \
                 timeout_seconds = ?, enabled = ?, updated_at = ?, visibility = ?, public_domain \
                 = ?, public_display_name = ?, owner_peer_id = ? WHERE id = ?",
                params![
                    monitor.name.clone(),
                    monitor.target.clone(),
                    monitor.check_type.clone(),
                    monitor.interval_seconds as i64,
                    monitor.timeout_seconds as i64,
                    if monitor.enabled { 1 } else { 0 },
                    updated_at,
                    visibility_str,
                    monitor.public_domain.clone(),
                    monitor.public_display_name.clone(),
                    monitor.owner_peer_id.clone(),
                    id
                ],
            )
            .await?;
            Ok(id)
        } else {
            // Insert new monitor
            conn.execute(
                "INSERT INTO monitors (uuid, name, target, check_type, interval_seconds, \
                 timeout_seconds, enabled, created_at, updated_at, visibility, public_domain, \
                 public_display_name, owner_peer_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
                 ?)",
                params![
                    monitor.uuid.to_string(),
                    monitor.name.clone(),
                    monitor.target.clone(),
                    monitor.check_type.clone(),
                    monitor.interval_seconds as i64,
                    monitor.timeout_seconds as i64,
                    if monitor.enabled { 1 } else { 0 },
                    created_at,
                    updated_at,
                    visibility_str,
                    monitor.public_domain.clone(),
                    monitor.public_display_name.clone(),
                    monitor.owner_peer_id.clone()
                ],
            )
            .await?;

            Ok(conn.last_insert_rowid())
        }
    }

    async fn delete_monitor(&self, uuid: Uuid) -> Result<()> {
        let conn = self.get_conn().await?;

        // Delete all results for this monitor first to avoid foreign key constraint
        conn.execute(
            "DELETE FROM monitor_results WHERE monitor_uuid = ?",
            params![uuid.to_string()],
        )
        .await?;

        // Delete peer results as well
        conn.execute("DELETE FROM peer_results WHERE monitor_uuid = ?", params![uuid.to_string()])
            .await?;

        // Now delete the monitor itself
        conn.execute("DELETE FROM monitors WHERE uuid = ?", params![uuid.to_string()])
            .await?;

        Ok(())
    }

    async fn save_result(&self, result: &CheckResult) -> Result<i64> {
        let conn = self.get_conn().await?;
        let timestamp = Monitor::timestamp_to_i64(result.timestamp);
        let created_at = Monitor::timestamp_to_i64(std::time::SystemTime::now());
        let location = crate::location::get_location();

        conn.execute(
            "INSERT INTO monitor_results (monitor_uuid, timestamp, status, latency_ms, \
             status_code, error_message, peer_id, signature, created_at, city, country, region) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        )
        .await?;

        Ok(conn.last_insert_rowid())
    }

    async fn save_peer_result(
        &self,
        result: &PeerResult,
        target: &str,
        receipt: &str,
    ) -> Result<super::peer_storage::SaveOutcome> {
        let conn = self.get_conn().await?;
        super::peer_storage::save(&conn, result, target, receipt).await
    }

    async fn get_recent_results(
        &self,
        monitor_uuid: Uuid,
        limit: usize,
    ) -> Result<Vec<MonitorResult>> {
        let conn = self.get_conn().await?;
        let stmt = conn
            .prepare(
                "SELECT id, monitor_uuid, timestamp, status, latency_ms, status_code, \
                 error_message, peer_id, signature, created_at, city, country, region FROM \
                 monitor_results WHERE monitor_uuid = ? ORDER BY timestamp DESC LIMIT ?",
            )
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
        let stmt = conn
            .prepare(
                "SELECT id, monitor_uuid, timestamp, status, latency_ms, status_code, \
                 error_message, peer_id, signature, verified, created_at FROM peer_results WHERE \
                 monitor_uuid = ? ORDER BY timestamp DESC LIMIT ?",
            )
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
                source_peer_id: row.get(7).ok(),
                synced_from_peer: false,
                retention_until: None,
            });
        }

        Ok(results)
    }

    async fn upsert_peer(&self, peer: &Peer) -> Result<()> {
        let conn = self.get_conn().await?;
        let tx = conn.transaction_with_behavior(libsql::TransactionBehavior::Immediate).await?;
        tx.execute("DELETE FROM peers WHERE peer_id != ? AND peer_id IN (SELECT peer_id FROM peers WHERE peer_id != ? ORDER BY last_seen DESC,peer_id LIMIT -1 OFFSET 4095)", params![peer.peer_id.clone(),peer.peer_id.clone()]).await?;
        let last_seen = Monitor::timestamp_to_i64(peer.last_seen);
        let joined_at = Monitor::timestamp_to_i64(peer.joined_at);

        tx.execute(
            "INSERT INTO peers (peer_id, status, last_seen, joined_at, contribution_score, \
             uptime_percentage, checks_per_day, location_city, location_region, location_country)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(peer_id) DO UPDATE SET status=excluded.status, \
             last_seen=excluded.last_seen",
            params![
                peer.peer_id.clone(),
                peer.status.clone(),
                last_seen,
                joined_at,
                peer.contribution_score,
                peer.uptime_percentage,
                peer.checks_per_day,
                peer.location_city.clone(),
                peer.location_region.clone(),
                peer.location_country.clone()
            ],
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn mark_peer_offline(&self, peer_id: &str, now: std::time::SystemTime) -> Result<()> {
        let conn = self.get_conn().await?;
        let ts = Monitor::timestamp_to_i64(now);

        conn.execute(
            "UPDATE peers SET status = 'offline', last_seen = ? WHERE peer_id = ?",
            params![ts, peer_id],
        )
        .await?;

        Ok(())
    }

    async fn get_peer_by_id(&self, peer_id: &str) -> Result<Option<Peer>> {
        let conn = self.get_conn().await?;

        let mut rows = conn
            .query(
                "SELECT peer_id, status, last_seen, joined_at, contribution_score, \
                 uptime_percentage, checks_per_day, location_city, location_region, \
                 location_country
                 FROM peers WHERE peer_id = ?",
                params![peer_id],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let peer_id: String = row.get(0)?;
            let status: String = row.get(1)?;
            let last_seen_i64: i64 = row.get(2)?;
            let joined_at_i64: i64 = row.get(3)?;
            let contribution_score: f64 = row.get(4)?;
            let uptime_percentage: f64 = row.get(5)?;
            let checks_per_day: i64 = row.get(6)?;
            let location_city: Option<String> = row.get(7)?;
            let location_region: Option<String> = row.get(8)?;
            let location_country: Option<String> = row.get(9)?;

            let last_seen = Monitor::i64_to_timestamp(last_seen_i64);
            let joined_at = Monitor::i64_to_timestamp(joined_at_i64);

            Ok(Some(Peer {
                peer_id,
                status,
                last_seen,
                joined_at,
                contribution_score,
                uptime_percentage,
                checks_per_day,
                location_city,
                location_region,
                location_country,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_peers(&self, limit: usize) -> Result<Vec<Peer>> {
        let conn = self.get_conn().await?;

        let stmt = conn
            .prepare(
                "SELECT peer_id, status, last_seen, joined_at, contribution_score, \
                 uptime_percentage, checks_per_day, location_city, location_region, \
                 location_country FROM peers ORDER BY last_seen DESC LIMIT ?",
            )
            .await?;

        let mut rows = stmt.query(params![limit as i64]).await?;
        let mut peers = Vec::new();

        while let Some(row) = rows.next().await? {
            let peer_id: String = row.get(0)?;
            let status: String = row.get(1)?;
            let last_seen_i64: i64 = row.get(2)?;
            let joined_at_i64: i64 = row.get(3)?;
            let contribution_score: f64 = row.get(4)?;
            let uptime_percentage: f64 = row.get(5)?;
            let checks_per_day: i64 = row.get(6)?;
            let location_city: Option<String> = row.get(7)?;
            let location_region: Option<String> = row.get(8)?;
            let location_country: Option<String> = row.get(9)?;

            peers.push(Peer {
                peer_id,
                status,
                last_seen: Monitor::i64_to_timestamp(last_seen_i64),
                joined_at: Monitor::i64_to_timestamp(joined_at_i64),
                contribution_score,
                uptime_percentage,
                checks_per_day,
                location_city,
                location_region,
                location_country,
            });
        }

        Ok(peers)
    }

    async fn insert_network_stats(&self, stats: &NetworkStats) -> Result<i64> {
        let conn = self.get_conn().await?;
        let ts = Monitor::timestamp_to_i64(stats.timestamp);

        conn.execute(
            "INSERT INTO network_stats (timestamp, total_peers, online_peers, checks_performed, \
             checks_received, bandwidth_used_mb)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                ts,
                stats.total_peers,
                stats.online_peers,
                stats.checks_performed,
                stats.checks_received,
                stats.bandwidth_used_mb
            ],
        )
        .await?;

        Ok(conn.last_insert_rowid())
    }

    async fn get_latest_network_stats(&self) -> Result<Option<NetworkStats>> {
        let conn = self.get_conn().await?;
        let stmt = conn
            .prepare(
                "SELECT timestamp, total_peers, online_peers, checks_performed, checks_received, \
                 bandwidth_used_mb
                 FROM network_stats ORDER BY timestamp DESC LIMIT 1",
            )
            .await?;

        let mut rows = stmt.query(()).await?;

        if let Some(row) = rows.next().await? {
            let ts: i64 = row.get(0)?;
            Ok(Some(NetworkStats {
                timestamp: Monitor::i64_to_timestamp(ts),
                total_peers: row.get(1)?,
                online_peers: row.get(2)?,
                checks_performed: row.get(3)?,
                checks_received: row.get(4)?,
                bandwidth_used_mb: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn query_peer_results(
        &self,
        since_timestamp: i64,
        monitor_uuid: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<PeerResult>> {
        let conn = self.get_conn().await?;

        let (query, _params_list): (String, Vec<String>) = if let Some(uuid) = monitor_uuid {
            (
                format!(
                    "SELECT id, monitor_uuid, timestamp, status, latency_ms, status_code, \
                     error_message, peer_id, signature, verified, created_at, city, country, \
                     region, source_peer_id, synced_from_peer, retention_until FROM peer_results \
                     WHERE timestamp > ? AND monitor_uuid = ? ORDER BY timestamp DESC LIMIT {}",
                    limit
                ),
                vec![since_timestamp.to_string(), uuid.to_string()],
            )
        } else {
            (
                format!(
                    "SELECT id, monitor_uuid, timestamp, status, latency_ms, status_code, \
                     error_message, peer_id, signature, verified, created_at, city, country, \
                     region, source_peer_id, synced_from_peer, retention_until FROM peer_results \
                     WHERE timestamp > ? ORDER BY timestamp DESC LIMIT {}",
                    limit
                ),
                vec![since_timestamp.to_string()],
            )
        };

        let stmt = conn.prepare(&query).await?;

        let mut rows = if let Some(uuid) = monitor_uuid {
            stmt.query(params![since_timestamp, uuid.to_string()]).await?
        } else {
            stmt.query(params![since_timestamp]).await?
        };

        let mut results = Vec::new();

        while let Some(row) = rows.next().await? {
            let monitor_uuid_str: String = row.get(1)?;
            let status_str: String = row.get(3)?;
            let timestamp: i64 = row.get(2)?;
            let created_at: i64 = row.get(10)?;
            let synced: i64 = row.get(15)?;
            let retention: Option<i64> = row.get(16)?;

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
                source_peer_id: row.get(14).unwrap_or_default(),
                synced_from_peer: synced != 0,
                retention_until: retention,
            });
        }

        Ok(results)
    }

    async fn mark_peer_results_synced(
        &self,
        source_peer_id: &str,
        until_timestamp: i64,
    ) -> Result<()> {
        let conn = self.get_conn().await?;

        conn.execute(
            "UPDATE peer_results SET synced_from_peer = 1 WHERE source_peer_id = ? AND timestamp \
             <= ?",
            params![source_peer_id, until_timestamp],
        )
        .await?;

        Ok(())
    }

    async fn cleanup_local_results(&self, private_cutoff: i64, public_cutoff: i64) -> Result<u64> {
        let conn = self.get_conn().await?;
        // Bounded batches keep the shared writer available while catching up after downtime.
        let count = conn.execute("DELETE FROM monitor_results WHERE id IN (SELECT r.id FROM monitor_results r JOIN monitors m ON m.uuid = r.monitor_uuid WHERE r.timestamp < CASE WHEN m.visibility = 'Public' THEN ? ELSE ? END LIMIT 10000)", params![public_cutoff,private_cutoff]).await?;
        Ok(count as u64)
    }

    async fn cleanup_expired_peer_results(&self) -> Result<u64> {
        let conn = self.get_conn().await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs() as i64;

        let result = conn
            .execute(
                "DELETE FROM peer_results WHERE id IN (SELECT id FROM peer_results WHERE COALESCE(retention_until, created_at + 604800) < ?)",
                params![now],
            )
            .await?;

        Ok(result as u64)
    }

    // ===== Public Monitor Group Methods =====

    async fn get_public_monitor_group(
        &self,
        domain: &str,
    ) -> Result<Option<peerup::distributed::PublicMonitorGroup>> {
        let conn = self.get_conn().await?;

        let stmt = conn
            .prepare(
                "SELECT domain, display_name, participating_peers, schedule_json, total_checks, \
                 created_at, last_updated FROM public_monitor_groups WHERE domain = ?",
            )
            .await?;

        let mut rows = stmt.query(params![domain]).await?;

        if let Some(row) = rows.next().await? {
            let participating_peers_json: String = row.get(2)?;
            let schedule_json: String = row.get(3)?;

            let participating_peers: Vec<String> = serde_json::from_str(&participating_peers_json)?;
            let schedule: peerup::distributed::OrchestrationSchedule =
                serde_json::from_str(&schedule_json)?;

            let group = peerup::distributed::PublicMonitorGroup {
                domain: row.get(0)?,
                display_name: row.get(1)?,
                participating_peers,
                schedule,
                total_checks: row.get(4)?,
                created_at: row.get(5)?,
                last_updated: row.get(6)?,
            };

            Ok(Some(group))
        } else {
            Ok(None)
        }
    }

    async fn save_public_monitor_group(
        &self,
        group: &peerup::distributed::PublicMonitorGroup,
    ) -> Result<()> {
        let conn = self.get_conn().await?;

        let participating_peers_json = serde_json::to_string(&group.participating_peers)?;
        let schedule_json = serde_json::to_string(&group.schedule)?;

        conn.execute(
            "INSERT INTO public_monitor_groups (domain, display_name, participating_peers, \
             schedule_json, total_checks, created_at, last_updated) VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(domain) DO UPDATE SET display_name = excluded.display_name, \
             participating_peers = excluded.participating_peers, schedule_json = \
             excluded.schedule_json, total_checks = excluded.total_checks, last_updated = \
             excluded.last_updated",
            params![
                group.domain.clone(),
                group.display_name.clone(),
                participating_peers_json,
                schedule_json,
                group.total_checks,
                group.created_at,
                group.last_updated,
            ],
        )
        .await?;

        Ok(())
    }

    async fn get_orchestration_votes(
        &self,
        domain: &str,
    ) -> Result<Vec<peerup::distributed::OrchestrationVote>> {
        let conn = self.get_conn().await?;

        let stmt = conn
            .prepare(
                "SELECT domain, voter_peer_id, schedule_json, signature, timestamp FROM \
                 orchestration_votes WHERE domain = ? ORDER BY timestamp DESC",
            )
            .await?;

        let mut rows = stmt.query(params![domain]).await?;
        let mut votes = Vec::new();

        while let Some(row) = rows.next().await? {
            let schedule_json: String = row.get(2)?;
            let schedule: peerup::distributed::OrchestrationSchedule =
                serde_json::from_str(&schedule_json)?;

            votes.push(peerup::distributed::OrchestrationVote {
                domain: row.get(0)?,
                voter_peer_id: row.get(1)?,
                schedule,
                signature: row.get(3)?,
                public_key: None, // Not stored in DB; verified at receive time
                timestamp: row.get(4)?,
            });
        }

        Ok(votes)
    }

    async fn save_orchestration_vote(
        &self,
        vote: &peerup::distributed::OrchestrationVote,
    ) -> Result<()> {
        let conn = self.get_conn().await?;

        let schedule_json = serde_json::to_string(&vote.schedule)?;

        conn.execute(
            "INSERT INTO orchestration_votes (domain, voter_peer_id, schedule_json, signature, \
             timestamp) VALUES (?, ?, ?, ?, ?) ON CONFLICT(domain, voter_peer_id) DO UPDATE SET \
             schedule_json = excluded.schedule_json, signature = excluded.signature, timestamp = \
             excluded.timestamp",
            params![
                vote.domain.clone(),
                vote.voter_peer_id.clone(),
                schedule_json,
                vote.signature.clone(),
                vote.timestamp,
            ],
        )
        .await?;

        Ok(())
    }

    async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.get_conn().await?;
        let mut rows = conn.query("SELECT value FROM settings WHERE key = ?", params![key]).await?;

        if let Some(row) = rows.next().await? {
            let value: String = row.get(0)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.get_conn().await?;
        let now = crate::database::models::Monitor::timestamp_to_i64(std::time::SystemTime::now());
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) ON CONFLICT(key) DO \
             UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now],
        )
        .await?;
        Ok(())
    }

    async fn save_audit_event(&self, event: &AuditEvent) -> Result<i64> {
        let conn = self.get_conn().await?;

        conn.execute(
            "INSERT INTO audit_events (
                event_uuid,
                event_type,
                schema_version,
                created_at,
                actor_id,
                actor_public_key,
                resource_type,
                resource_id,
                parent_event_uuid,
                payload_json,
                payload_hash,
                capability_id,
                delegated_by,
                expires_at,
                context_json,
                signature
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                event.event_uuid.to_string(),
                event.event_type.clone(),
                event.schema_version,
                Monitor::timestamp_to_i64(event.created_at),
                event.actor_id.clone(),
                event.actor_public_key.clone(),
                event.resource_type.clone(),
                event.resource_id.clone(),
                event.parent_event_uuid.map(|uuid| uuid.to_string()),
                event.payload_json.clone(),
                event.payload_hash.clone(),
                event.capability_id.clone(),
                event.delegated_by.clone(),
                event.expires_at.map(Monitor::timestamp_to_i64),
                event.context_json.clone(),
                event.signature.clone(),
            ],
        )
        .await?;

        let row_id = conn.last_insert_rowid();
        Ok(row_id)
    }

    async fn get_audit_event(&self, event_uuid: Uuid) -> Result<Option<AuditEvent>> {
        let conn = self.get_conn().await?;
        let mut rows = conn
            .query(
                "SELECT
                    id,
                    event_uuid,
                    event_type,
                    schema_version,
                    created_at,
                    actor_id,
                    actor_public_key,
                    resource_type,
                    resource_id,
                    parent_event_uuid,
                    payload_json,
                    payload_hash,
                    capability_id,
                    delegated_by,
                    expires_at,
                    context_json,
                    signature
                 FROM audit_events
                 WHERE event_uuid = ?",
                params![event_uuid.to_string()],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(Self::map_audit_event_row(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn list_audit_events(&self, limit: usize) -> Result<Vec<AuditEvent>> {
        let conn = self.get_conn().await?;
        let mut rows = conn
            .query(
                "SELECT
                    id,
                    event_uuid,
                    event_type,
                    schema_version,
                    created_at,
                    actor_id,
                    actor_public_key,
                    resource_type,
                    resource_id,
                    parent_event_uuid,
                    payload_json,
                    payload_hash,
                    capability_id,
                    delegated_by,
                    expires_at,
                    context_json,
                    signature
                 FROM audit_events
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?",
                params![limit as i64],
            )
            .await?;

        let mut events = Vec::new();
        while let Some(row) = rows.next().await? {
            events.push(Self::map_audit_event_row(&row)?);
        }

        Ok(events)
    }

    async fn save_audit_attestation(&self, attestation: &AuditAttestation) -> Result<i64> {
        let conn = self.get_conn().await?;

        conn.execute(
            "INSERT INTO audit_attestations (
                attestation_uuid,
                subject_event_uuid,
                attestor_id,
                attestor_public_key,
                decision,
                reason,
                created_at,
                signature
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                attestation.attestation_uuid.to_string(),
                attestation.subject_event_uuid.to_string(),
                attestation.attestor_id.clone(),
                attestation.attestor_public_key.clone(),
                attestation.decision.clone(),
                attestation.reason.clone(),
                Monitor::timestamp_to_i64(attestation.created_at),
                attestation.signature.clone(),
            ],
        )
        .await?;

        Ok(conn.last_insert_rowid())
    }

    async fn list_audit_attestations(
        &self,
        subject_event_uuid: Uuid,
    ) -> Result<Vec<AuditAttestation>> {
        let conn = self.get_conn().await?;
        let mut rows = conn
            .query(
                "SELECT
                    id,
                    attestation_uuid,
                    subject_event_uuid,
                    attestor_id,
                    attestor_public_key,
                    decision,
                    reason,
                    created_at,
                    signature
                 FROM audit_attestations
                 WHERE subject_event_uuid = ?
                 ORDER BY created_at DESC, id DESC",
                params![subject_event_uuid.to_string()],
            )
            .await?;

        let mut attestations = Vec::new();
        while let Some(row) = rows.next().await? {
            attestations.push(Self::map_audit_attestation_row(&row)?);
        }

        Ok(attestations)
    }
}
