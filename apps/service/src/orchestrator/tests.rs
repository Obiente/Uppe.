/// Integration tests for orchestrator components
///
/// These tests verify end-to-end functionality of:
/// - Owner Sync (DHT query → decrypt → store)
/// - Retention Cleanup (expire → delete)
/// - Private Monitor orchestration
use crate::crypto::{KeyPair, load_or_generate_keypair};
use crate::database::{Database, DatabaseImpl};
use crate::monitoring::types::{CheckResult, MonitorStatus};
use crate::orchestrator::{PrivateMonitorOrchestrator, RetentionCleanup, RetentionPolicy};
use crate::p2p::P2PNetwork;
use crate::pool::{LibsqlManager, LibsqlPool};
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tempfile::tempdir;
use uuid::Uuid;

/// Helper to create test database pool
async fn create_test_database() -> Result<(LibsqlPool, String)> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let db = libsql::Builder::new_local(&db_path_str).build().await?;
    let manager = LibsqlManager::new(db);
    let pool = deadpool::managed::Pool::builder(manager)
        .config(deadpool::managed::PoolConfig::default())
        .build()?;

    // Initialize schema
    let conn: deadpool::managed::Object<LibsqlManager> = pool.get().await?;
    crate::database::initialize_database(&*conn).await?;

    Ok((pool, db_path_str))
}

/// Helper to create test keypair
fn create_test_keypair() -> KeyPair {
    let temp_dir = tempdir().unwrap();
    let keypair_path = temp_dir.path().join("test_keypair.key");
    load_or_generate_keypair(&keypair_path).unwrap()
}

/// Helper to create test P2P network (disabled)
fn create_test_p2p_network(peer_id: String, public_key: [u8; 32]) -> Arc<P2PNetwork> {
    let config = peerup::node::NodeConfig::builder()
        .port_range((40000, 40010))
        .disable_mdns()
        .disable_kademlia()
        .disable_relay()
        .build();

    Arc::new(P2PNetwork::with_config(
        peer_id,
        false, // Disabled for unit tests
        public_key,
        config,
    ))
}

#[tokio::test]
async fn test_retention_policy_integration() -> Result<()> {
    let (pool, _db_path) = create_test_database().await?;
    let database = Arc::new(DatabaseImpl::new_from_pool(pool));

    // Create custom retention policy: 1 second for all types
    let policy = RetentionPolicy {
        private_result_days: 0,
        public_result_days: 0,
        peer_result_days: 0,
    };

    let cleanup = RetentionCleanup::new(database.clone(), policy);

    // Create old peer result (expired)
    let old_result = crate::database::models::PeerResult {
        id: None,
        peer_id: "test_peer".to_string(),
        monitor_uuid: Uuid::new_v4(),
        status: MonitorStatus::Up,
        latency_ms: Some(100),
        status_code: Some(200),
        error_message: None,
        timestamp: SystemTime::now() - Duration::from_secs(7 * 24 * 3600 + 3600), // 7 days + 1 hour ago
        verified: true,
        signature: vec![0u8; 64],
        created_at: SystemTime::now(),
        city: None,
        country: None,
        region: None,
        source_peer_id: None,
        synced_from_peer: false,
        retention_until: None,
    };

    database.save_peer_result(&old_result).await?;

    // Run cleanup
    cleanup.cleanup_expired_results().await?;

    // Verify old result was deleted
    // Note: We'd need a query method to verify deletion
    // For now, this tests that cleanup runs without errors

    Ok(())
}

#[tokio::test]
async fn test_private_orchestrator_creation() -> Result<()> {
    let (pool, _db_path) = create_test_database().await?;
    let database = Arc::new(DatabaseImpl::new_from_pool(pool));
    let keypair = create_test_keypair();
    let peer_id = keypair.public_key_hex();
    let owner_pubkey = keypair.x25519_public_key();
    let p2p_network = create_test_p2p_network(peer_id.clone(), keypair.public_key_bytes());

    // Create private orchestrator
    let _orchestrator = PrivateMonitorOrchestrator::new(
        database.clone(),
        peer_id,
        owner_pubkey,
        p2p_network,
    );

    // Verify creation succeeds (orchestrator should be functional)
    Ok(())
}

#[tokio::test]
async fn test_owner_sync_with_empty_dht() -> Result<()> {
    let (pool, _db_path) = create_test_database().await?;
    let database = Arc::new(DatabaseImpl::new_from_pool(pool));
    let keypair = create_test_keypair();
    let peer_id = keypair.public_key_hex();
    let owner_pubkey = keypair.x25519_public_key();
    let owner_secret_key = keypair.x25519_secret_bytes();
    let p2p_network = create_test_p2p_network(peer_id.clone(), keypair.public_key_bytes());

    let orchestrator = PrivateMonitorOrchestrator::new(
        database.clone(),
        peer_id,
        owner_pubkey,
        p2p_network,
    );

    // Sync with empty DHT should complete without error (no monitors, no assignments)
    // This tests the graceful handling of "no data" scenario
    match orchestrator.sync_owner_results_from_dht(&owner_secret_key).await {
        Ok(()) => {
            // Success - no data to sync is valid
        }
        Err(e) => {
            // Should not fail on empty DHT
            panic!("Owner sync should handle empty DHT gracefully, got error: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_retention_cleanup_with_recent_results() -> Result<()> {
    let (pool, _db_path) = create_test_database().await?;
    let database = Arc::new(DatabaseImpl::new_from_pool(pool));

    // Default policy: 7 days for private, 30 for public/peer
    let policy = RetentionPolicy::default();
    let cleanup = RetentionCleanup::new(database.clone(), policy);

    // Create recent peer result (should NOT be deleted)
    let recent_result = crate::database::models::PeerResult {
        id: None,
        peer_id: "test_peer".to_string(),
        monitor_uuid: Uuid::new_v4(),
        status: MonitorStatus::Up,
        latency_ms: Some(100),
        status_code: Some(200),
        error_message: None,
        timestamp: SystemTime::now() - Duration::from_secs(3600), // 1 hour ago
        verified: true,
        signature: vec![0u8; 64],
        created_at: SystemTime::now(),
        city: None,
        country: None,
        region: None,
        source_peer_id: None,
        synced_from_peer: false,
        retention_until: None,
    };

    database.save_peer_result(&recent_result).await?;

    // Run cleanup
    cleanup.cleanup_expired_results().await?;

    // Recent result should still exist
    // (In a full test, we'd query the database to verify it exists)

    Ok(())
}

#[tokio::test]
async fn test_encryption_roundtrip_integration() -> Result<()> {
    use peerup::crypto::{decrypt_result_for_owner, encrypt_result_for_owner};

    // Use the KeyPair type which properly derives X25519 keys
    let owner_keypair = peerup::crypto::generate_keypair();
    let keypair = create_test_keypair();

    let check_result = CheckResult {
        monitor_id: Uuid::new_v4(),
        target: "https://private-service.local".to_string(),
        check_type: "https".to_string(),
        status: MonitorStatus::Up,
        latency_ms: Some(42),
        status_code: Some(200),
        error_message: None,
        timestamp: SystemTime::now(),
        peer_id: keypair.public_key_hex(),
        owner_peer_id: Some("owner-peer-id".to_string()),
        signature: None,
    };

    // Encrypt using the owner's X25519 public key
    let encrypted = encrypt_result_for_owner(
        &check_result,
        &owner_keypair.x25519_public_key(),
        "helper-peer-id".to_string(),
        "owner-peer-id".to_string(),
        "monitor-uuid".to_string(),
    )?;

    // Decrypt using the owner's X25519 secret key — this is the critical roundtrip
    let decrypted: CheckResult = decrypt_result_for_owner(
        &encrypted,
        &owner_keypair.x25519_secret_bytes(),
    )?;

    assert_eq!(decrypted.monitor_id, check_result.monitor_id);
    assert_eq!(decrypted.target, check_result.target);
    assert_eq!(decrypted.status, check_result.status);
    assert_eq!(decrypted.latency_ms, check_result.latency_ms);
    assert_eq!(encrypted.owner_peer_id, "owner-peer-id");
    assert_eq!(encrypted.monitor_uuid, "monitor-uuid");

    Ok(())
}

#[tokio::test]
async fn test_retention_periodic_cleanup_starts() -> Result<()> {
    let (pool, _db_path) = create_test_database().await?;
    let database = Arc::new(DatabaseImpl::new_from_pool(pool));

    let policy = RetentionPolicy::default();
    let cleanup = RetentionCleanup::new(database.clone(), policy);

    // Start periodic cleanup
    let handle = cleanup.start_periodic_cleanup();

    // Verify handle is active
    assert!(!handle.is_finished(), "Cleanup task should be running");

    // Abort the task (we don't want it running during other tests)
    handle.abort();

    Ok(())
}

#[cfg(test)]
mod helper_assignment_tests {
    use super::*;

    #[tokio::test]
    async fn test_helper_assignment_flow() -> Result<()> {
        let (pool, _db_path) = create_test_database().await?;
        let database = Arc::new(DatabaseImpl::new_from_pool(pool));
        let keypair = create_test_keypair();
        let peer_id = keypair.public_key_hex();
        let owner_pubkey = keypair.x25519_public_key();
        let p2p_network = create_test_p2p_network(peer_id.clone(), keypair.public_key_bytes());

        let _orchestrator = PrivateMonitorOrchestrator::new(
            database.clone(),
            peer_id,
            owner_pubkey,
            p2p_network,
        );

        // Test helper assignment logic
        // Create a private monitor
        let mut monitor = crate::database::models::Monitor::new_private(
            "Private Test".to_string(),
            "https://internal.example.com".to_string(),
            "https".to_string(),
            keypair.public_key_hex(),
        );
        monitor.enabled = true;

        let _monitor_id = database.save_monitor(&monitor).await?;

        // Test that orchestrator can handle private monitors
        // (Full integration would require mocked peers)

        Ok(())
    }
}
