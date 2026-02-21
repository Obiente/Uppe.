//! Automatic retention and cleanup of monitoring results.
//!
//! This module manages data lifecycle:
//! - Private monitor results: Cleaned up after 7 days
//! - Public monitor results: Cleaned up after 30 days
//! - Peer results: Cleaned up after 30 days
//!
//! Cleanup runs periodically (every hour) as a background task.

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::database::Database;

/// Retention policy for different result types
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// Days to keep private monitor results
    pub private_result_days: i64,
    /// Days to keep public monitor results
    pub public_result_days: i64,
    /// Days to keep peer results
    pub peer_result_days: i64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            private_result_days: 7,
            public_result_days: 30,
            peer_result_days: 30,
        }
    }
}

impl RetentionPolicy {
    /// Get retention duration in seconds
    fn private_retention_seconds(&self) -> i64 {
        self.private_result_days * 24 * 3600
    }

    fn public_retention_seconds(&self) -> i64 {
        self.public_result_days * 24 * 3600
    }

    fn peer_retention_seconds(&self) -> i64 {
        self.peer_result_days * 24 * 3600
    }
}

/// Cleanup manager for expired results
pub struct RetentionCleanup {
    database: Arc<dyn Database>,
    policy: RetentionPolicy,
}

impl RetentionCleanup {
    /// Create a new retention cleanup manager
    pub fn new(database: Arc<dyn Database>, policy: RetentionPolicy) -> Self {
        Self { database, policy }
    }

    /// Run cleanup for all result types
    pub async fn cleanup_expired_results(&self) -> Result<()> {
        info!("Starting retention cleanup");

        let peer_count = self.database.cleanup_expired_peer_results().await?;

        info!(
            "Retention cleanup completed: {} peer results deleted",
            peer_count
        );

        Ok(())
    }

    /// Clean up local monitoring results (both private and public)
    /// Note: Currently a placeholder for future implementation
    /// Will be fully implemented when we refactor to use a unified result storage approach
    pub async fn cleanup_local_results(&self) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let private_cutoff = now - self.policy.private_retention_seconds();
        let public_cutoff = now - self.policy.public_retention_seconds();

        debug!(
            "Cleaning up private results (older than {} days, cutoff: {})",
            self.policy.private_result_days, private_cutoff
        );
        debug!(
            "Cleaning up public results (older than {} days, cutoff: {})",
            self.policy.public_result_days, public_cutoff
        );

        // TODO: Implement when monitor_results table supports visibility and retention_until
        // For now, this is a placeholder that will be filled when the schema is updated
        info!("Local results cleanup not yet implemented (waiting for schema update)");
        Ok(0)
    }

    /// Start background cleanup task (runs every hour)
    pub fn start_periodic_cleanup(&self) -> tokio::task::JoinHandle<()> {
        let database = Arc::clone(&self.database);
        let policy = self.policy.clone();

        tokio::spawn(async move {
            let cleanup = RetentionCleanup::new(database, policy);
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // 1 hour

            loop {
                interval.tick().await;

                // Cleanup peer results
                match cleanup.cleanup_expired_results().await {
                    Ok(()) => {
                        debug!("Periodic peer results cleanup completed successfully");
                    }
                    Err(e) => {
                        warn!("Periodic peer results cleanup failed: {}", e);
                    }
                }

                // Cleanup local monitor results
                match cleanup.cleanup_local_results().await {
                    Ok(count) => {
                        debug!("Periodic local results cleanup completed: {} deleted", count);
                    }
                    Err(e) => {
                        warn!("Periodic local results cleanup failed: {}", e);
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_policy_defaults() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.private_result_days, 7);
        assert_eq!(policy.public_result_days, 30);
        assert_eq!(policy.peer_result_days, 30);
    }

    #[test]
    fn test_retention_seconds_calculation() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.private_retention_seconds(), 7 * 24 * 3600);
        assert_eq!(policy.public_retention_seconds(), 30 * 24 * 3600);
        assert_eq!(policy.peer_retention_seconds(), 30 * 24 * 3600);
    }

    #[test]
    fn test_custom_retention_policy() {
        let policy = RetentionPolicy {
            private_result_days: 14,
            public_result_days: 60,
            peer_result_days: 14,
        };
        assert_eq!(policy.private_retention_seconds(), 14 * 24 * 3600);
        assert_eq!(policy.public_retention_seconds(), 60 * 24 * 3600);
    }
}
