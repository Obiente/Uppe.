use anyhow::{Result, anyhow};
use std::net::{IpAddr, ToSocketAddrs};
use url::Url;

/// Validation results with specific error messages
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub error: Option<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self { is_valid: true, error: None }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self { is_valid: false, error: Some(msg.into()) }
    }

    #[allow(dead_code)] // Public API method
    pub fn to_result(&self) -> Result<()> {
        if self.is_valid {
            Ok(())
        } else {
            Err(anyhow!(self.error.clone().unwrap_or_else(|| "Validation failed".to_string())))
        }
    }
}

/// Validate HTTP/HTTPS URL endpoint
pub fn validate_http_endpoint(target: &str) -> ValidationResult {
    if target.trim().is_empty() {
        return ValidationResult::err("Target cannot be empty");
    }

    // Try to parse as URL
    match Url::parse(target) {
        Ok(url) => {
            let scheme = url.scheme();
            if scheme != "http" && scheme != "https" {
                return ValidationResult::err(format!(
                    "Invalid scheme '{scheme}'. Must be http or https"
                ));
            }

            if url.host_str().is_none() {
                return ValidationResult::err("URL must have a valid host");
            }

            ValidationResult::ok()
        }
        Err(e) => {
            // If it fails to parse, check if it's missing a scheme
            if !target.contains("://") {
                ValidationResult::err("URL must include scheme (http:// or https://)")
            } else {
                ValidationResult::err(format!("Invalid URL: {e}"))
            }
        }
    }
}

/// Validate HTTPS URL endpoint
pub fn validate_https_endpoint(target: &str) -> ValidationResult {
    if target.trim().is_empty() {
        return ValidationResult::err("Target cannot be empty");
    }

    match Url::parse(target) {
        Ok(url) => {
            if url.scheme() != "https" {
                return ValidationResult::err(format!(
                    "Invalid scheme '{}'. Must be https",
                    url.scheme()
                ));
            }

            if url.host_str().is_none() {
                return ValidationResult::err("URL must have a valid host");
            }

            ValidationResult::ok()
        }
        Err(e) => {
            if !target.contains("://") {
                ValidationResult::err("URL must include scheme (https://)")
            } else {
                ValidationResult::err(format!("Invalid URL: {e}"))
            }
        }
    }
}

/// Validate TCP endpoint (host:port format)
pub fn validate_tcp_endpoint(target: &str) -> ValidationResult {
    if target.trim().is_empty() {
        return ValidationResult::err("Target cannot be empty");
    }

    // Try to resolve as socket address
    match target.to_socket_addrs() {
        Ok(_) => ValidationResult::ok(),
        Err(_) => {
            // Check if it has the right format (host:port)
            if !target.contains(':') {
                return ValidationResult::err("TCP target must be in format 'host:port'");
            }

            let parts: Vec<&str> = target.split(':').collect();
            if parts.len() != 2 {
                return ValidationResult::err("TCP target must be in format 'host:port'");
            }

            // Validate port
            match parts[1].parse::<u16>() {
                Ok(port) if port > 0 => ValidationResult::ok(),
                Ok(_) => ValidationResult::err("Port must be between 1 and 65535"),
                Err(_) => ValidationResult::err("Invalid port number"),
            }
        }
    }
}

/// Validate ICMP endpoint (IP address or hostname)
pub fn validate_icmp_endpoint(target: &str) -> ValidationResult {
    if target.trim().is_empty() {
        return ValidationResult::err("Target cannot be empty");
    }

    // Try to parse as IP address
    if target.parse::<IpAddr>().is_ok() {
        return ValidationResult::ok();
    }

    // Check if it looks like a valid hostname
    if target.contains(' ') {
        return ValidationResult::err("Target cannot contain spaces");
    }

    if target.starts_with('-') || target.ends_with('-') {
        return ValidationResult::err("Hostname cannot start or end with hyphen");
    }

    if target.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        ValidationResult::ok()
    } else {
        ValidationResult::err("Invalid hostname. Use IP address or valid hostname")
    }
}

/// Validate monitor target based on check type
pub fn validate_monitor_target(target: &str, check_type: &str) -> ValidationResult {
    match check_type.to_lowercase().as_str() {
        "http" => validate_http_endpoint(target),
        "https" => validate_https_endpoint(target),
        "tcp" => validate_tcp_endpoint(target),
        "icmp" => validate_icmp_endpoint(target),
        _ => ValidationResult::err(format!("Unknown check type: {check_type}")),
    }
}

/// Validate monitor name
pub fn validate_monitor_name(name: &str) -> ValidationResult {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return ValidationResult::err("Name cannot be empty");
    }

    if trimmed.len() > 100 {
        return ValidationResult::err("Name too long (max 100 characters)");
    }

    ValidationResult::ok()
}

/// Validate monitor interval
pub fn validate_interval(interval: u64) -> ValidationResult {
    if interval == 0 {
        return ValidationResult::err("Interval must be at least 1 second");
    }

    if interval > 86400 {
        return ValidationResult::err("Interval too long (max 24 hours)");
    }

    ValidationResult::ok()
}

/// Validate monitor timeout
pub fn validate_timeout(timeout: u64, interval: u64) -> ValidationResult {
    if timeout == 0 {
        return ValidationResult::err("Timeout must be at least 1 second");
    }

    if timeout >= interval {
        return ValidationResult::err("Timeout must be less than interval");
    }

    ValidationResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_validation() {
        assert!(validate_http_endpoint("http://example.com").is_valid);
        assert!(validate_http_endpoint("https://example.com").is_valid);
        assert!(validate_http_endpoint("http://192.168.1.1").is_valid);
        assert!(validate_http_endpoint("http://example.com:8080/path").is_valid);

        assert!(!validate_http_endpoint("").is_valid);
        assert!(!validate_http_endpoint("example.com").is_valid);
        assert!(!validate_http_endpoint("ftp://example.com").is_valid);
    }

    #[test]
    fn test_https_validation() {
        assert!(validate_https_endpoint("https://example.com").is_valid);
        assert!(!validate_https_endpoint("http://example.com").is_valid);
        assert!(!validate_https_endpoint("").is_valid);
    }

    #[test]
    fn test_tcp_validation() {
        assert!(validate_tcp_endpoint("localhost:8080").is_valid);
        assert!(validate_tcp_endpoint("192.168.1.1:443").is_valid);
        assert!(validate_tcp_endpoint("example.com:22").is_valid);

        assert!(!validate_tcp_endpoint("").is_valid);
        assert!(!validate_tcp_endpoint("localhost").is_valid);
        assert!(!validate_tcp_endpoint("localhost:").is_valid);
        assert!(!validate_tcp_endpoint("localhost:abc").is_valid);
    }

    #[test]
    fn test_icmp_validation() {
        assert!(validate_icmp_endpoint("192.168.1.1").is_valid);
        assert!(validate_icmp_endpoint("example.com").is_valid);
        assert!(validate_icmp_endpoint("sub.example.com").is_valid);

        assert!(!validate_icmp_endpoint("").is_valid);
        assert!(!validate_icmp_endpoint("invalid hostname").is_valid);
    }

    #[test]
    fn test_name_validation() {
        assert!(validate_monitor_name("My Monitor").is_valid);
        assert!(validate_monitor_name("Test123").is_valid);

        assert!(!validate_monitor_name("").is_valid);
        assert!(!validate_monitor_name("   ").is_valid);
    }

    #[test]
    fn test_timeout_validation() {
        assert!(validate_timeout(5, 10).is_valid);
        assert!(!validate_timeout(10, 10).is_valid);
        assert!(!validate_timeout(15, 10).is_valid);
        assert!(!validate_timeout(0, 10).is_valid);
    }
}
