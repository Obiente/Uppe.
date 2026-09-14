//! Remote claims have bounded, signed receipts, separate from the permanent operator ledger.
use super::models::{Monitor, PeerResult};
use anyhow::Result;
use libsql::{Connection, TransactionBehavior, params};

pub const MAX_RESULTS: usize = 20_000;
pub const MAX_PER_PEER: usize = 1_000;
pub const RETENTION_SECONDS: i64 = 604_800;

#[derive(Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Inserted,
    Duplicate,
    Rejected,
}

pub async fn save(
    conn: &Connection,
    result: &PeerResult,
    target: &str,
    receipt: &str,
) -> Result<SaveOutcome> {
    if !result.verified
        || result.signature.len() != 64
        || result.peer_id.len() != 64
        || target.len() > 2048
        || receipt.len() > 8192
        || receipt.is_empty()
        || result.error_message.as_ref().is_some_and(|s| s.len() > 512)
        || [&result.city, &result.country, &result.region, &result.source_peer_id]
            .iter()
            .any(|s| s.as_ref().is_some_and(|s| s.len() > 128))
    {
        return Ok(SaveOutcome::Rejected);
    }
    let timestamp = Monitor::timestamp_to_i64(result.timestamp);
    let now = chrono::Utc::now().timestamp();
    if !(now - 300..=now + 30).contains(&timestamp) {
        return Ok(SaveOutcome::Rejected);
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).await?;
    // Enforce the local subscription even if a caller forgets its ingress policy.
    let mut rows = tx.query("SELECT 1 FROM monitors WHERE uuid=? AND target=? AND enabled=1 AND visibility!='Internal'", params![result.monitor_uuid.to_string(), target]).await?;
    let subscribed = rows.next().await?.is_some();
    drop(rows);
    if !subscribed {
        return Ok(SaveOutcome::Rejected);
    }
    let mut rows = tx
        .query(
            "SELECT 1 FROM peer_results WHERE peer_id=? AND monitor_uuid=? AND timestamp=?",
            params![result.peer_id.clone(), result.monitor_uuid.to_string(), timestamp],
        )
        .await?;
    let duplicate = rows.next().await?.is_some();
    drop(rows);
    if duplicate {
        return Ok(SaveOutcome::Duplicate);
    }
    // Reject repeated claims for a subscription faster than its configured interval.
    let mut rows = tx.query("SELECT 1 FROM peer_results p JOIN monitors m ON m.uuid=p.monitor_uuid WHERE p.peer_id=? AND p.monitor_uuid=? AND p.timestamp > ?-m.interval_seconds LIMIT 1", params![result.peer_id.clone(), result.monitor_uuid.to_string(), timestamp]).await?;
    let too_soon = rows.next().await?.is_some();
    drop(rows);
    if too_soon {
        return Ok(SaveOutcome::Rejected);
    }
    // Evict oldest receipts transactionally. Neither Sybil churn nor cleanup delays
    // can grow this store beyond its row/field budgets (under 220 MiB of payload).
    tx.execute("DELETE FROM peer_results WHERE peer_id=? AND id IN (SELECT id FROM peer_results WHERE peer_id=? ORDER BY id DESC LIMIT -1 OFFSET ?)", params![result.peer_id.clone(), result.peer_id.clone(), (MAX_PER_PEER-1) as i64]).await?;
    tx.execute("DELETE FROM peer_results WHERE id IN (SELECT id FROM peer_results ORDER BY id DESC LIMIT -1 OFFSET ?)", params![(MAX_RESULTS-1) as i64]).await?;
    tx.execute("INSERT INTO peer_results(monitor_uuid,timestamp,status,latency_ms,status_code,error_message,peer_id,signature,verified,created_at,city,country,region,source_peer_id,synced_from_peer,retention_until,receipt_json) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", params![result.monitor_uuid.to_string(), timestamp, result.status.to_string(), result.latency_ms.map(|v|v as i64), result.status_code.map(|v|v as i64), result.error_message.clone(), result.peer_id.clone(), result.signature.clone(), 1, now, result.city.clone(), result.country.clone(), result.region.clone(), result.source_peer_id.clone(), 0, now+RETENTION_SECONDS, receipt]).await?;
    tx.commit().await?;
    Ok(SaveOutcome::Inserted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{audit, crypto::signing::sign_result, monitoring::types::CheckResult};
    use std::time::SystemTime;
    async fn count(conn: &Connection, table: &str) -> i64 {
        conn.query(&format!("SELECT count(*) FROM {table}"), ())
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap()
            .get(0)
            .unwrap()
    }
    #[tokio::test]
    async fn replay_quotas_and_retention_keep_receipts_bounded() -> Result<()> {
        let db = libsql::Builder::new_local(":memory:").build().await?;
        let conn = db.connect()?;
        super::super::initialize_database(&conn).await?;
        let key = peerup::crypto::generate_keypair();
        let id = uuid::Uuid::new_v4();
        let target = "https://example.invalid";
        let mut check = CheckResult::new(id, target.into(), "http".into(), key.public_key_hex())
            .success(10, Some(200));
        check.signature = Some(sign_result(&check, &key)?);
        let result = PeerResult {
            id: None,
            monitor_uuid: id,
            timestamp: check.timestamp,
            status: check.status,
            latency_ms: check.latency_ms,
            status_code: check.status_code,
            error_message: None,
            peer_id: check.peer_id.clone(),
            signature: check.signature.clone().unwrap(),
            verified: true,
            created_at: SystemTime::now(),
            city: None,
            country: None,
            region: None,
            source_peer_id: None,
            synced_from_peer: false,
            retention_until: None,
        };
        let receipt =
            audit::peer_result_receipt(&key, &key.public_key_hex(), "test", &result, target)?;
        let value: serde_json::Value = serde_json::from_str(&receipt)?;
        assert!(
            serde_json::from_value::<audit::SignedEventEnvelope>(value["event"].clone())?
                .verify()?
        );
        assert!(
            serde_json::from_value::<audit::SignedAuditAttestation>(value["attestation"].clone())?
                .verify()?
        );
        assert_eq!(save(&conn, &result, target, &receipt).await?, SaveOutcome::Rejected);
        conn.execute("INSERT INTO monitors(uuid,name,target,check_type,created_at,updated_at,visibility) VALUES(?,'synthetic',?,'http',0,0,'Public')",params![id.to_string(),target]).await?;
        let audit_before = count(&conn, "audit_outbox").await;
        assert_eq!(save(&conn, &result, target, &receipt).await?, SaveOutcome::Inserted);
        let mut replay = result.clone();
        replay.error_message = Some("different wire bytes".into());
        assert_eq!(save(&conn, &replay, target, &receipt).await?, SaveOutcome::Duplicate);
        assert_eq!(count(&conn, "peer_results").await, 1);
        assert_eq!(count(&conn, "audit_outbox").await, audit_before);
        assert_eq!(count(&conn, "audit_events").await, 0);
        replay.error_message = Some("x".repeat(513));
        assert_eq!(save(&conn, &replay, target, &receipt).await?, SaveOutcome::Rejected);
        assert_eq!(
            save(&conn, &result, "https://wrong.invalid", &receipt).await?,
            SaveOutcome::Rejected
        );
        // Seed a full retained history without sending traffic or producing large payloads.
        conn.execute("DELETE FROM peer_results", ()).await?;
        conn.execute("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<?) INSERT INTO peer_results(monitor_uuid,timestamp,status,peer_id,signature,created_at,retention_until) SELECT 'seed',0,'up',CASE WHEN x<=? THEN ? ELSE 'other-'||x END,x'',0,0 FROM n",params![MAX_RESULTS as i64,MAX_PER_PEER as i64,result.peer_id.clone()]).await?;
        assert_eq!(save(&conn, &result, target, &receipt).await?, SaveOutcome::Inserted);
        assert!(count(&conn, "peer_results").await <= MAX_RESULTS as i64);
        let peers: i64 = conn
            .query(
                "SELECT count(*) FROM peer_results WHERE peer_id=?",
                params![result.peer_id.clone()],
            )
            .await?
            .next()
            .await?
            .unwrap()
            .get(0)?;
        assert_eq!(peers, MAX_PER_PEER as i64);
        // A single sweep removes all expired rows, including more than the old 10k batch.
        conn.execute(
            "DELETE FROM peer_results WHERE retention_until < ?",
            params![chrono::Utc::now().timestamp()],
        )
        .await?;
        assert_eq!(count(&conn, "peer_results").await, 1);
        Ok(())
    }
}
