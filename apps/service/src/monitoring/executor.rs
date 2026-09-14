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
            return result.failure("Node check capacity reached".into());
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
