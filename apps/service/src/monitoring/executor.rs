use super::types::CheckResult;
use anyhow::Result;

/// Executes checks using each monitor's timeout and destination policy.
pub struct MonitoringExecutor {
    peer_id: String,
    degraded_threshold_ms: u64,
    permits: tokio::sync::Semaphore,
}

impl MonitoringExecutor {
    pub fn new(peer_id: String, _timeout_seconds: u64, degraded_threshold_ms: u64) -> Result<Self> {
        Ok(Self { peer_id, degraded_threshold_ms, permits: tokio::sync::Semaphore::new(64) })
    }
    pub async fn execute_config(&self, config: &super::scheduler::MonitorConfig) -> CheckResult {
        let result = CheckResult::new(
            config.id,
            config.target.clone(),
            format!("{:?}", config.check_type).to_lowercase(),
            self.peer_id.clone(),
        );
        // Skip overloaded ticks instead of growing a queue of overdue checks.
        let Ok(_permit) = self.permits.try_acquire() else {
            return result.unavailable("Node check capacity reached".into());
        };
        match super::checker::check_target(
            &config.target,
            config.check_type,
            config.timeout_seconds,
            config.allow_private_targets,
        )
        .await
        {
            Ok((latency, code)) if latency > self.degraded_threshold_ms => {
                result.degraded(latency, code)
            }
            Ok((latency, code)) => result.success(latency, code),
            Err(error) => result.failure(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn saturation_is_unknown_and_does_not_contact_target() {
        let executor = MonitoringExecutor::new("synthetic-peer".into(), 1, 1000).unwrap();
        let held = executor.permits.acquire_many(64).await.unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = super::super::scheduler::MonitorConfig {
            id: uuid::Uuid::new_v4(),
            target: listener.local_addr().unwrap().to_string(),
            check_type: super::super::checker::CheckType::Tcp,
            interval_seconds: 10,
            enabled: true,
            timeout_seconds: 1,
            allow_private_targets: true,
            phase_offset_secs: 0,
        };
        let result = executor.execute_config(&config).await;
        assert_eq!(result.status, super::super::types::MonitorStatus::Unknown);
        assert!(result.latency_ms.is_none());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), listener.accept())
                .await
                .is_err()
        );
        drop(held);
        assert_eq!(
            executor.execute_config(&config).await.status,
            super::super::types::MonitorStatus::Up
        );
    }
}
