use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use uuid::Uuid;

use super::executor::MonitoringExecutor;
use super::checker::CheckType;
use super::types::CheckResult;

/// Monitor configuration for scheduling
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub id: Uuid,
    pub target: String,
    pub check_type: CheckType,
    pub interval_seconds: u64,
    pub enabled: bool,
}

/// Monitoring scheduler - coordinates execution of monitoring tasks
pub struct MonitoringScheduler {
    executor: Arc<MonitoringExecutor>,
    result_tx: mpsc::Sender<CheckResult>,
}

impl MonitoringScheduler {
    /// Create a new monitoring scheduler
    pub fn new(executor: Arc<MonitoringExecutor>, result_tx: mpsc::Sender<CheckResult>) -> Self {
        Self {
            executor,
            result_tx,
        }
    }

    /// Schedule a single monitor for periodic checking
    pub fn schedule_monitor(&self, config: MonitorConfig) -> tokio::task::JoinHandle<()> {
        let executor = self.executor.clone();
        let result_tx = self.result_tx.clone();

        tokio::spawn(async move {
            if !config.enabled {
                return;
            }

            let mut timer = interval(Duration::from_secs(config.interval_seconds));

            loop {
                timer.tick().await;

                let result = executor.execute_check(
                    config.id,
                    config.target.clone(),
                    config.check_type,
                ).await;

                // Send result to the result channel
                if let Err(e) = result_tx.send(result).await {
                    tracing::error!("Failed to send check result: {}", e);
                    break;
                }
            }
        })
    }

    /// Schedule multiple monitors
    pub fn schedule_monitors(&self, configs: Vec<MonitorConfig>) -> Vec<tokio::task::JoinHandle<()>> {
        configs
            .into_iter()
            .map(|config| self.schedule_monitor(config))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler() {
        let executor = Arc::new(
            MonitoringExecutor::new("test-peer".to_string(), 10, 1000).unwrap()
        );

        let (tx, mut rx) = mpsc::channel(10);
        let scheduler = MonitoringScheduler::new(executor, tx);

        let config = MonitorConfig {
            id: Uuid::new_v4(),
            target: "https://example.com".to_string(),
            check_type: CheckType::Https,
            interval_seconds: 1,
            enabled: true,
        };

        let _handle = scheduler.schedule_monitor(config);

        // Wait for at least one result
        let result = tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("Timeout waiting for result")
            .expect("Channel closed");

        assert!(result.latency_ms.is_some());
    }
}
