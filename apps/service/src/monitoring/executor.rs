use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use super::checker::{CheckType, Checker, HttpChecker, TcpChecker, IcmpChecker};
use super::types::CheckResult;

/// Monitoring executor - executes individual monitoring checks
pub struct MonitoringExecutor {
    http_checker: Arc<HttpChecker>,
    tcp_checker: Arc<TcpChecker>,
    icmp_checker: Arc<IcmpChecker>,
    peer_id: String,
    degraded_threshold_ms: u64,
}

impl MonitoringExecutor {
    /// Create a new monitoring executor
    pub fn new(peer_id: String, timeout_seconds: u64, degraded_threshold_ms: u64) -> Result<Self> {
        Ok(Self {
            http_checker: Arc::new(HttpChecker::new(timeout_seconds)?),
            tcp_checker: Arc::new(TcpChecker::new(timeout_seconds)),
            icmp_checker: Arc::new(IcmpChecker::new(timeout_seconds)),
            peer_id,
            degraded_threshold_ms,
        })
    }

    /// Execute a monitoring check
    pub async fn execute_check(
        &self,
        monitor_id: Uuid,
        target: String,
        check_type: CheckType,
    ) -> CheckResult {
        let mut result = CheckResult::new(monitor_id, target.clone(), self.peer_id.clone());

        let checker: &dyn Checker = match check_type {
            CheckType::Http | CheckType::Https => self.http_checker.as_ref(),
            CheckType::Tcp => self.tcp_checker.as_ref(),
            CheckType::Icmp => self.icmp_checker.as_ref(),
        };

        match checker.check(&target).await {
            Ok((latency_ms, status_code)) => {
                if latency_ms > self.degraded_threshold_ms {
                    result = result.degraded(latency_ms, status_code);
                } else {
                    result = result.success(latency_ms, status_code);
                }
            }
            Err(e) => {
                result = result.failure(e.to_string());
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::types::MonitorStatus;

    #[tokio::test]
    async fn test_http_check() {
        let executor = MonitoringExecutor::new(
            "test-peer".to_string(),
            10,
            1000,
        ).unwrap();

        let result = executor.execute_check(
            Uuid::new_v4(),
            "https://example.com".to_string(),
            CheckType::Https,
        ).await;

        // Should succeed for example.com
        assert!(matches!(result.status, MonitorStatus::Up | MonitorStatus::Degraded));
        assert!(result.latency_ms.is_some());
    }
}
