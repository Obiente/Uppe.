use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{Instant, interval_at};
use uuid::Uuid;

use super::checker::CheckType;
use super::executor::MonitoringExecutor;
use super::types::CheckResult;

/// Monitor configuration for scheduling
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorConfig {
    pub id: Uuid,
    pub target: String,
    pub check_type: CheckType,
    pub interval_seconds: u64,
    pub enabled: bool,
    pub timeout_seconds: u64,
    pub allow_private_targets: bool,
    /// How many seconds to wait before the first check.
    /// Used to stagger checks across peers so they don't all hit the
    /// target at the same instant.  0 = start after one full interval
    /// (original behaviour).
    pub phase_offset_secs: u64,
}

/// Monitoring scheduler - coordinates execution of monitoring tasks
pub struct MonitoringScheduler {
    executor: Arc<MonitoringExecutor>,
    result_tx: mpsc::Sender<CheckResult>,
}

impl MonitoringScheduler {
    /// Create a new monitoring scheduler
    pub fn new(executor: Arc<MonitoringExecutor>, result_tx: mpsc::Sender<CheckResult>) -> Self {
        Self { executor, result_tx }
    }

    /// Schedule a single monitor for periodic checking
    pub fn schedule_monitor(&self, config: MonitorConfig) -> tokio::task::JoinHandle<()> {
        let executor = self.executor.clone();
        let result_tx = self.result_tx.clone();

        tokio::spawn(async move {
            if !config.enabled || config.interval_seconds == 0 {
                return;
            }

            let period = Duration::from_secs(config.interval_seconds);

            // When a phase offset is specified, start the first tick at
            // `now + phase_offset` and then tick every `interval`.  This
            // staggers checks across peers so at most one peer hits the
            // target at any given second.
            // With no offset (or offset ≥ interval) fall back to the
            // original "first check after one full interval" behaviour.
            let first_tick = if config.phase_offset_secs > 0
                && config.phase_offset_secs < config.interval_seconds
            {
                Instant::now() + Duration::from_secs(config.phase_offset_secs)
            } else {
                Instant::now() + period
            };
            let mut timer = interval_at(first_tick, period);
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                timer.tick().await;

                let result = executor.execute_config(&config).await;

                // Send result to the result channel
                if let Err(e) = result_tx.send(result).await {
                    tracing::error!("Failed to send check result: {}", e);
                    break;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler() {
        let executor =
            Arc::new(MonitoringExecutor::new("test-peer".to_string(), 10, 1000).unwrap());

        let (tx, mut rx) = mpsc::channel(10);
        let scheduler = MonitoringScheduler::new(executor, tx);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = MonitorConfig {
            id: Uuid::new_v4(),
            target: listener.local_addr().unwrap().to_string(),
            check_type: CheckType::Tcp,
            interval_seconds: 1,
            enabled: true,
            timeout_seconds: 1,
            allow_private_targets: true,
            phase_offset_secs: 0,
        };

        let handle = scheduler.schedule_monitor(config);

        // Wait for at least one result
        let result = tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("Timeout waiting for result")
            .expect("Channel closed");

        assert!(result.latency_ms.is_some());
        handle.abort();
    }
}
