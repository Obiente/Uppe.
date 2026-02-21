//! Monitor-type specific validation for distributed orchestration.
//!
//! This module provides security validation for different monitor types
//! to prevent abuse and ensure production readiness.

use anyhow::{anyhow, Result};
use std::net::IpAddr;
use url::Url;

/// Validates a monitor target based on its type
pub fn validate_monitor_target(target: &str, check_type: &str) -> Result<()> {
    match check_type {
        "http" | "https" => validate_http_target(target),
        "tcp" => validate_tcp_target(target),
        "icmp" => validate_icmp_target(target),
        _ => Err(anyhow!("Unsupported check type: {}", check_type)),
    }
}

/// Validate HTTP/HTTPS target
fn validate_http_target(target: &str) -> Result<()> {
    // Parse URL
    let url = Url::parse(target).map_err(|e| anyhow!("Invalid URL: {}", e))?;

    // Validate scheme
    match url.scheme() {
        "http" | "https" => {}
        other => return Err(anyhow!("Invalid scheme for HTTP monitor: {}", other)),
    }

    // Prevent localhost/private IP scanning
    if let Some(host) = url.host_str() {
        if is_private_or_local(host) {
            return Err(anyhow!(
                "Private/local addresses not allowed in public monitors: {}",
                host
            ));
        }
    }

    // Validate port (if specified)
    if let Some(port) = url.port() {
        validate_port(port)?;
    }

    Ok(())
}

/// Validate TCP target
fn validate_tcp_target(target: &str) -> Result<()> {
    // Expected format: host:port
    let parts: Vec<&str> = target.split(':').collect();

    if parts.len() != 2 {
        return Err(anyhow!("TCP target must be in format host:port"));
    }

    let host = parts[0];
    let port: u16 = parts[1]
        .parse()
        .map_err(|_| anyhow!("Invalid port number"))?;

    // Prevent private IP scanning
    if is_private_or_local(host) {
        return Err(anyhow!(
            "Private/local addresses not allowed in public monitors"
        ));
    }

    // Validate port
    validate_port(port)?;

    // Blocklist sensitive ports
    validate_port_allowlist(port)?;

    Ok(())
}

/// Validate ICMP target
fn validate_icmp_target(target: &str) -> Result<()> {
    // Can be hostname or IP
    if is_private_or_local(target) {
        return Err(anyhow!(
            "Private/local addresses not allowed in public monitors"
        ));
    }

    Ok(())
}

/// Check if hostname/IP is private or localhost
fn is_private_or_local(host: &str) -> bool {
    // Check for localhost
    if host == "localhost" || host == "127.0.0.1" || host == "::1" {
        return true;
    }

    // Try parsing as IP address
    if let Ok(ip) = host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(ipv4) => {
                // Check private ranges
                ipv4.is_private()
                    || ipv4.is_loopback()
                    || ipv4.is_link_local()
                    || ipv4.is_broadcast()
                    || ipv4.is_documentation()
                    || ipv4.is_unspecified()
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback() || ipv6.is_unspecified() || ipv6.is_multicast()
            }
        }
    } else {
        // Check for special hostnames
        matches!(
            host,
            "localhost"
                | "local"
                | "internal"
                | "private"
                | "*.local"
                | "*.internal"
                | "*.private"
        )
    }
}

/// Validate port is in valid range
fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(anyhow!("Port 0 is not valid"));
    }
    Ok(())
}

/// Validate port is not in sensitive range
fn validate_port_allowlist(port: u16) -> Result<()> {
    // Blocklist sensitive/system ports for public monitors
    let blocked_ports = [
        22,   // SSH
        23,   // Telnet
        25,   // SMTP
        110,  // POP3
        143,  // IMAP
        445,  // SMB
        3389, // RDP
        5432, // PostgreSQL
        5900, // VNC
        6379, // Redis
        27017, // MongoDB
    ];

    if blocked_ports.contains(&port) {
        return Err(anyhow!(
            "Port {} is blocked for security reasons (system/database port)",
            port
        ));
    }

    // Warn about privileged ports (informational)
    if port < 1024 {
        tracing::warn!(
            "Monitor using privileged port {} - ensure this is intentional",
            port
        );
    }

    Ok(())
}

/// Validate check interval for security
pub fn validate_check_interval(interval_seconds: u64) -> Result<()> {
    const MIN_INTERVAL: u64 = 10; // 10 seconds
    const MAX_INTERVAL: u64 = 86400; // 24 hours

    if interval_seconds < MIN_INTERVAL {
        return Err(anyhow!(
            "Check interval too short: {} seconds (minimum: {})",
            interval_seconds,
            MIN_INTERVAL
        ));
    }

    if interval_seconds > MAX_INTERVAL {
        return Err(anyhow!(
            "Check interval too long: {} seconds (maximum: {})",
            interval_seconds,
            MAX_INTERVAL
        ));
    }

    Ok(())
}

/// Validate timeout is reasonable
pub fn validate_timeout(timeout_seconds: u64) -> Result<()> {
    const MIN_TIMEOUT: u64 = 1;
    const MAX_TIMEOUT: u64 = 300; // 5 minutes

    if timeout_seconds < MIN_TIMEOUT {
        return Err(anyhow!(
            "Timeout too short: {} seconds (minimum: {})",
            timeout_seconds,
            MIN_TIMEOUT
        ));
    }

    if timeout_seconds > MAX_TIMEOUT {
        return Err(anyhow!(
            "Timeout too long: {} seconds (maximum: {})",
            timeout_seconds,
            MAX_TIMEOUT
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_http_target() {
        // Valid
        assert!(validate_http_target("https://example.com").is_ok());
        assert!(validate_http_target("http://example.com:8080").is_ok());

        // Invalid - private IPs
        assert!(validate_http_target("http://localhost").is_err());
        assert!(validate_http_target("http://127.0.0.1").is_err());
        assert!(validate_http_target("http://192.168.1.1").is_err());
        assert!(validate_http_target("http://10.0.0.1").is_err());

        // Invalid - wrong scheme
        assert!(validate_http_target("ftp://example.com").is_err());
    }

    #[test]
    fn test_validate_tcp_target() {
        // Valid
        assert!(validate_tcp_target("example.com:80").is_ok());
        assert!(validate_tcp_target("google.com:443").is_ok());

        // Invalid - private IPs
        assert!(validate_tcp_target("127.0.0.1:80").is_err());
        assert!(validate_tcp_target("192.168.1.1:80").is_err());

        // Invalid - blocked ports
        assert!(validate_tcp_target("example.com:22").is_err()); // SSH
        assert!(validate_tcp_target("example.com:3389").is_err()); // RDP

        // Invalid - format
        assert!(validate_tcp_target("example.com").is_err());
        assert!(validate_tcp_target("example.com:").is_err());
    }

    #[test]
    fn test_validate_check_interval() {
        assert!(validate_check_interval(10).is_ok()); // Min
        assert!(validate_check_interval(60).is_ok()); // Normal
        assert!(validate_check_interval(86400).is_ok()); // Max

        assert!(validate_check_interval(5).is_err()); // Too short
        assert!(validate_check_interval(100000).is_err()); // Too long
    }

    #[test]
    fn test_private_ip_detection() {
        assert!(is_private_or_local("localhost"));
        assert!(is_private_or_local("127.0.0.1"));
        assert!(is_private_or_local("192.168.1.1"));
        assert!(is_private_or_local("10.0.0.1"));
        assert!(is_private_or_local("172.16.0.1"));

        assert!(!is_private_or_local("8.8.8.8"));
        assert!(!is_private_or_local("1.1.1.1"));
        assert!(!is_private_or_local("example.com"));
    }
}
