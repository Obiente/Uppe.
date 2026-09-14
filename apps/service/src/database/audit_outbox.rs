//! Durable changes captured by SQLite triggers and signed by the Rust node.
//!
//! Mutation and enqueue share the writer's transaction, including Go API writes.
//! Event insertion and dequeue share a second transaction; a failed signer or a
//! process interruption leaves the pending change available on restart.

use crate::audit::{NewSignedEvent, SignedEventEnvelope};
use crate::crypto::KeyPair;
use anyhow::Result;
use libsql::{Connection, params};
use serde_json::Value;

pub async fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_peer_result_identity ON peer_results(peer_id, monitor_uuid, timestamp);
        CREATE TABLE IF NOT EXISTS audit_outbox (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        event_type TEXT NOT NULL,
        resource_type TEXT NOT NULL,
        resource_id TEXT NOT NULL,
        payload_json TEXT NOT NULL
    );
    CREATE TRIGGER IF NOT EXISTS audit_events_no_update BEFORE UPDATE ON audit_events
    BEGIN SELECT RAISE(ABORT, 'audit events are immutable'); END;
    CREATE TRIGGER IF NOT EXISTS audit_events_no_delete BEFORE DELETE ON audit_events
    BEGIN SELECT RAISE(ABORT, 'audit events are immutable'); END;
    CREATE TRIGGER IF NOT EXISTS audit_attestations_no_update BEFORE UPDATE ON audit_attestations
    BEGIN SELECT RAISE(ABORT, 'audit attestations are immutable'); END;
    CREATE TRIGGER IF NOT EXISTS audit_attestations_no_delete BEFORE DELETE ON audit_attestations
    BEGIN SELECT RAISE(ABORT, 'audit attestations are immutable'); END;",
    )
    .await?;

    for (table, resource, key, columns, actions) in [
        (
            "monitors",
            "monitor",
            "uuid",
            "'name', {row}.name, 'target', {row}.target, 'check_type', {row}.check_type, 'enabled', {row}.enabled, 'interval_seconds', {row}.interval_seconds, 'timeout_seconds', {row}.timeout_seconds, 'visibility', {row}.visibility",
            "INSERT UPDATE DELETE",
        ),
        (
            "monitor_results",
            "result",
            "id",
            "'monitor_uuid', {row}.monitor_uuid, 'timestamp', {row}.timestamp, 'status', {row}.status, 'latency_ms', {row}.latency_ms, 'peer_id', {row}.peer_id, 'signature', hex({row}.signature)",
            "INSERT",
        ),
        (
            "peer_results",
            "peer_result",
            "id",
            "'monitor_uuid', {row}.monitor_uuid, 'timestamp', {row}.timestamp, 'status', {row}.status, 'peer_id', {row}.peer_id, 'signature', hex({row}.signature), 'verified', {row}.verified",
            "INSERT",
        ),
        (
            "status_pages",
            "status_page",
            "uuid",
            "'title', {row}.title, 'slug', {row}.slug, 'is_active', {row}.is_active",
            "INSERT UPDATE DELETE",
        ),
        (
            "status_page_monitors",
            "status_page_binding",
            "status_page_id",
            "'monitor_uuid', {row}.monitor_uuid",
            "INSERT DELETE",
        ),
    ] {
        for action in actions.split_whitespace() {
            let row = if action == "DELETE" { "OLD" } else { "NEW" };
            let event = match action {
                "INSERT" => "created",
                "UPDATE" => "updated",
                _ => "deleted",
            };
            let columns = columns.replace("{row}", row);
            let action_clause = if action == "UPDATE" && table == "status_pages" {
                "UPDATE OF title, slug, description, logo_url, primary_color, is_active"
            } else {
                action
            };
            // Identifiers above are fixed schema constants, never request input.
            let sql = format!(
                "CREATE TRIGGER IF NOT EXISTS audit_{table}_{action}
                AFTER {action_clause} ON {table} BEGIN
                INSERT INTO audit_outbox(event_type, resource_type, resource_id, payload_json)
                VALUES ('storage.{resource}.{event}', '{resource}', CAST({row}.{key} AS TEXT),
                    json_object('observed_at', unixepoch(), {columns})); END;"
            );
            conn.execute_batch(&sql).await?;
        }
    }
    Ok(())
}

pub async fn drain(conn: &Connection, key: &KeyPair) -> Result<usize> {
    let mut completed = 0;
    for _ in 0..100 {
        let tx = conn.transaction_with_behavior(libsql::TransactionBehavior::Immediate).await?;
        let mut rows = tx.query("SELECT id, event_type, resource_type, resource_id, payload_json FROM audit_outbox ORDER BY id LIMIT 1", ()).await?;
        let Some(row) = rows.next().await? else {
            tx.commit().await?;
            break;
        };
        let id: i64 = row.get(0)?;
        let event_type: String = row.get(1)?;
        let resource_type: String = row.get(2)?;
        let resource_id: String = row.get(3)?;
        let payload: Value = serde_json::from_str(&row.get::<String>(4)?)?;
        drop(rows);
        let actor = key.public_key_hex();
        let event = SignedEventEnvelope::sign(
            NewSignedEvent::<_, Value> {
                event_type: &event_type,
                actor_id: &actor,
                resource_type: &resource_type,
                resource_id: &resource_id,
                parent_event_id: None,
                payload: &payload,
                capability_id: None,
                delegated_by: None,
                expires_at: None,
                context: None,
            },
            key,
        )?;
        tx.execute(
            "INSERT INTO audit_events (event_uuid,event_type,schema_version,created_at,
            actor_id,actor_public_key,resource_type,resource_id,payload_json,payload_hash,signature)
            VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            params![
                event.event_id.to_string(),
                event.event_type,
                event.schema_version,
                event.created_at.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                event.actor_id,
                event.actor_public_key,
                event.resource_type,
                event.resource_id,
                event.payload_json,
                event.payload_hash,
                event.signature,
            ],
        )
        .await?;
        tx.execute("DELETE FROM audit_outbox WHERE id = ?", [id]).await?;
        tx.commit().await?;
        completed += 1;
    }
    Ok(completed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mutation_is_durable_and_audit_is_append_only() {
        let db = libsql::Builder::new_local(":memory:").build().await.unwrap();
        let conn = db.connect().unwrap();
        crate::database::migrations::run_migrations(&conn).await.unwrap();
        conn.execute("INSERT INTO monitors(uuid,name,target,check_type,interval_seconds,timeout_seconds,enabled,created_at,updated_at) VALUES('synthetic-monitor','Test','https://example.com','http',60,10,1,1,1)",()).await.unwrap();
        let key = peerup::crypto::generate_keypair();
        assert_eq!(drain(&conn, &key).await.unwrap(), 1);
        assert_eq!(drain(&conn, &key).await.unwrap(), 0);
        assert!(conn.execute("UPDATE audit_events SET event_type='forged'", ()).await.is_err());
        assert!(conn.execute("DELETE FROM audit_events", ()).await.is_err());
        // A failed event insertion must retain the original pending change.
        conn.execute_batch("CREATE TRIGGER reject_test_event BEFORE INSERT ON audit_events BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").await.unwrap();
        conn.execute("UPDATE monitors SET name='Updated' WHERE uuid='synthetic-monitor'", ())
            .await
            .unwrap();
        assert!(drain(&conn, &key).await.is_err());
        conn.execute_batch("DROP TRIGGER reject_test_event;").await.unwrap();
        assert_eq!(drain(&conn, &key).await.unwrap(), 1);
    }
}
