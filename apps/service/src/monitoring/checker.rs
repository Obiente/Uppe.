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

/// Resolve once, reject non-public destinations for remote work, and pin the
/// validated addresses to the connection so DNS changes cannot bypass policy.
pub async fn check_target(
    target: &str,
    kind: CheckType,
    seconds: u64,
    allow_private: bool,
) -> Result<(u64, Option<u16>)> {
    super::validation::validate_timeout(seconds)?;
    timeout(Duration::from_secs(seconds.clamp(1, 300)), async {
        let start = Instant::now();
        let url = match kind {
            CheckType::Http | CheckType::Https => reqwest::Url::parse(target)?,
            CheckType::Tcp => reqwest::Url::parse(&format!("tcp://{target}"))?,
            CheckType::Icmp => return Err(anyhow!("ICMP monitoring is not supported")),
        };
        if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
            return Err(anyhow!("Credentials and fragments are not allowed in targets"));
        }
        if matches!(kind, CheckType::Http | CheckType::Https)
            && !matches!(url.scheme(), "http" | "https")
        {
            return Err(anyhow!("Only HTTP and HTTPS URLs are supported"));
        }
        let host = url.host_str().ok_or_else(|| anyhow!("Missing host"))?.trim_matches(['[', ']']);
        let port = url.port_or_known_default().ok_or_else(|| anyhow!("Missing port"))?;
        let addresses: Vec<_> = tokio::net::lookup_host((host, port)).await?.take(16).collect();
        if addresses.is_empty()
            || (!allow_private && addresses.iter().any(|a| !public_address(a.ip())))
        {
            return Err(anyhow!("Target does not resolve exclusively to public addresses"));
        }
        let code = if kind == CheckType::Tcp {
            tokio::net::TcpStream::connect(addresses.as_slice()).await?;
            None
        } else {
            let response = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(seconds.clamp(1, 300)))
                .resolve_to_addrs(host, &addresses)
                .build()?
                .get(url)
                .send()
                .await?;
            let code = response.status();
            if !(code.is_success() || code.is_redirection()) {
                return Err(anyhow!("HTTP status {}", code.as_u16()));
            }
            Some(code.as_u16())
        };
        Ok((start.elapsed().as_millis() as u64, code))
    })
    .await
    .map_err(|_| anyhow!("Check timed out"))?
}

fn public_address(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip) => {
            let o = ip.octets();
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
                || ip.is_multicast()
                || o[0] == 0
                || o[0] >= 240
                || (o[0] == 100 && (64..128).contains(&o[1]))
                || (o[0] == 198 && (18..20).contains(&o[1]))
                || (o[0] == 192 && o[1] == 0 && o[2] == 0))
        }
        std::net::IpAddr::V6(ip) => {
            // Restrict remote work to native global unicast. Excludes mapped IPv4,
            // local/unique-local/link-local, multicast and transition mechanisms.
            let s = ip.segments();
            s[0] & 0xe000 == 0x2000
                && s[0] != 0x2002
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    #[test]
    fn excludes_local_metadata_and_transition_addresses() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "::1",
            "::ffff:127.0.0.1",
            "2002:7f00:1::",
            "2001:db8::1",
        ] {
            assert!(!public_address(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_address("8.8.8.8".parse().unwrap()));
        assert!(public_address("2606:4700:4700::1111".parse().unwrap()));
    }
    #[tokio::test]
    async fn remote_work_cannot_reach_local_service() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = listener.local_addr().unwrap().to_string();
        assert!(check_target(&target, CheckType::Tcp, 1, false).await.is_err());
        assert!(check_target(&target, CheckType::Tcp, 1, true).await.is_ok());
    }
}
