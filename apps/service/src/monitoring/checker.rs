use anyhow::{Result, anyhow};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Type of monitoring check to perform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckType {
    Http,
    Https,
    Tcp,
    Icmp,
}

/// Checker trait for different types of monitoring checks
#[async_trait::async_trait]
pub trait Checker: Send + Sync {
    /// Perform the check and return latency in milliseconds and optional status code
    async fn check(&self, target: &str) -> Result<(u64, Option<u16>)>;
}

/// HTTP/HTTPS checker
pub struct HttpChecker {
    client: reqwest::Client,
}

impl HttpChecker {
    pub fn new(timeout_seconds: u64) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl Checker for HttpChecker {
    async fn check(&self, target: &str) -> Result<(u64, Option<u16>)> {
        let start = Instant::now();

        let response = self
            .client
            .get(target)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        let latency = start.elapsed().as_millis() as u64;
        let status_code = response.status().as_u16();

        // Consider 2xx and 3xx as success
        if response.status().is_success() || response.status().is_redirection() {
            Ok((latency, Some(status_code)))
        } else {
            Err(anyhow!("HTTP check failed with status code: {}", status_code))
        }
    }
}

/// TCP port checker
pub struct TcpChecker {
    timeout_duration: Duration,
}

impl TcpChecker {
    pub fn new(timeout_seconds: u64) -> Self {
        Self { timeout_duration: Duration::from_secs(timeout_seconds) }
    }
}

#[async_trait::async_trait]
impl Checker for TcpChecker {
    async fn check(&self, target: &str) -> Result<(u64, Option<u16>)> {
        let start = Instant::now();

        let connect = tokio::net::TcpStream::connect(target);

        timeout(self.timeout_duration, connect)
            .await
            .map_err(|_| anyhow!("TCP connection timeout"))?
            .map_err(|e| anyhow!("TCP connection failed: {}", e))?;

        let latency = start.elapsed().as_millis() as u64;
        Ok((latency, None))
    }
}

/// ICMP ping checker (placeholder - requires elevated privileges)
pub struct IcmpChecker {
    _timeout_duration: Duration,
}

impl IcmpChecker {
    pub fn new(timeout_seconds: u64) -> Self {
        Self { _timeout_duration: Duration::from_secs(timeout_seconds) }
    }
}

#[async_trait::async_trait]
impl Checker for IcmpChecker {
    async fn check(&self, _target: &str) -> Result<(u64, Option<u16>)> {
        // ICMP requires raw sockets and elevated privileges
        // For now, return a placeholder implementation
        // TODO: Implement proper ICMP ping using surge-ping or similar crate
        Err(anyhow!(
            "ICMP monitoring is not yet implemented. Please use HTTP, HTTPS, or TCP monitoring \
             instead."
        ))
    }
}
