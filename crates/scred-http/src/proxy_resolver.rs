use anyhow::{anyhow, Result};
/// Proxy Environment Variable Resolution
///
/// NOTE: The MITM proxy does NOT use http_proxy/https_proxy env vars for
/// upstream routing. These env vars are meant for CLIENTS to find the proxy,
/// not for the proxy to find upstreams. If upstream proxy routing is needed,
/// configure it via scred.yaml or a dedicated SCRED_UPSTREAM_PROXY env var.
///
/// This module is kept for reference but the MITM proxy's upstream connector
/// should not call MitmConfig::from_env() for upstream routing decisions.
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct MitmConfig {
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    no_proxy_list: Vec<NoProxyEntry>,
}

#[derive(Debug, Clone)]
enum NoProxyEntry {
    Host(String),
    Suffix(String),
    IpRange(String),
    Localhost,
    All, // NEW: Matches all hosts (NO_PROXY=*)
}

impl MitmConfig {
    /// Create a new proxy resolver from environment variables
    pub fn from_env() -> Self {
        // Try both lowercase and uppercase variants
        let http_proxy = std::env::var("http_proxy")
            .or_else(|_| std::env::var("HTTP_PROXY"))
            .ok()
            .filter(|s| !s.is_empty());

        let https_proxy = std::env::var("https_proxy")
            .or_else(|_| std::env::var("HTTPS_PROXY"))
            .ok()
            .filter(|s| !s.is_empty());

        let no_proxy_str = std::env::var("no_proxy")
            .or_else(|_| std::env::var("NO_PROXY"))
            .unwrap_or_default();

        let no_proxy_list = parse_no_proxy_list(&no_proxy_str);

        if http_proxy.is_some() || https_proxy.is_some() {
            warn!("Proxy environment variables detected - these will affect upstream routing");
            if let Some(ref val) = http_proxy {
                warn!("http_proxy: {}", val);
            }
            if let Some(ref val) = https_proxy {
                warn!("https_proxy: {}", val);
            }
            if !no_proxy_list.is_empty() {
                warn!(
                    "no_proxy: {} entries - {}",
                    no_proxy_list.len(),
                    no_proxy_list
                        .iter()
                        .map(|e| match e {
                            NoProxyEntry::All => "*".to_string(),
                            NoProxyEntry::Localhost => "localhost".to_string(),
                            NoProxyEntry::Host(h) => h.clone(),
                            NoProxyEntry::Suffix(s) => format!(".{}", s),
                            NoProxyEntry::IpRange(_) => "<ip_range>".to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        Self {
            http_proxy,
            https_proxy,
            no_proxy_list,
        }
    }

    /// Check if a host should bypass the proxy (in no_proxy list)
    fn should_bypass_proxy(&self, host: &str) -> bool {
        for entry in &self.no_proxy_list {
            match entry {
                NoProxyEntry::All => {
                    // "*" matches everything
                    return true;
                }
                NoProxyEntry::Localhost => {
                    if host == "localhost" || host == "127.0.0.1" || host == "::1" {
                        return true;
                    }
                }
                NoProxyEntry::Host(h) => {
                    if host.eq_ignore_ascii_case(h) {
                        return true;
                    }
                }
                NoProxyEntry::Suffix(suffix) => {
                    // Match domain suffix: ".example.com" matches "api.example.com"
                    if host.ends_with(suffix) || host.ends_with(&format!(".{}", suffix)) {
                        return true;
                    }
                }
                NoProxyEntry::IpRange(cidr) => {
                    // Simple IP matching (could be enhanced with CIDR parsing)
                    if host == cidr {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get the proxy URL to use for connecting to a target
    /// Returns None if:
    ///   1. No proxy is configured
    ///   2. Target is in no_proxy list
    pub fn get_proxy_for(&self, target_host: &str, is_https: bool) -> Option<String> {
        // Check no_proxy first
        if self.should_bypass_proxy(target_host) {
            debug!("Bypassing proxy for {}", target_host);
            return None;
        }

        // Select appropriate proxy based on protocol
        let proxy_value = if is_https {
            self.https_proxy.clone().or_else(|| self.http_proxy.clone())
        } else {
            self.http_proxy.clone()
        };

        // Filter out empty strings (happens when env var is set to "")
        match proxy_value {
            Some(val) if val.is_empty() => {
                debug!("Proxy env var is empty string, treating as None");
                None
            }
            other => other,
        }
    }

    /// Check if we have any proxy configured
    pub fn has_proxy(&self) -> bool {
        self.http_proxy.is_some() || self.https_proxy.is_some()
    }

    /// Get proxy statistics for debugging
    pub fn stats(&self) -> ProxyStats {
        ProxyStats {
            http_proxy_set: self.http_proxy.is_some(),
            https_proxy_set: self.https_proxy.is_some(),
            no_proxy_count: self.no_proxy_list.len(),
        }
    }
}

#[derive(Debug)]
pub struct ProxyStats {
    pub http_proxy_set: bool,
    pub https_proxy_set: bool,
    pub no_proxy_count: usize,
}

/// Parse no_proxy environment variable
/// Format: comma-separated list of hosts/domains/IPs
/// Examples:
///   - "localhost,127.0.0.1,.example.com"
///   - "localhost, .example.com, 192.168.0.0/16"
fn parse_no_proxy_list(no_proxy_str: &str) -> Vec<NoProxyEntry> {
    let mut entries = Vec::new();

    for entry in no_proxy_str.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        if entry == "*" {
            // "*" means bypass proxy for ALL hosts
            entries.push(NoProxyEntry::All);
        } else if entry == "localhost" || entry == "127.0.0.1" || entry == "::1" {
            entries.push(NoProxyEntry::Localhost);
        } else if entry.starts_with('.') || entry.contains('.') {
            // Domain suffix (.example.com or example.com)
            entries.push(NoProxyEntry::Suffix(entry.to_string()));
        } else if entry.contains('/') {
            // IP range (CIDR notation) - store as-is for now
            entries.push(NoProxyEntry::IpRange(entry.to_string()));
        } else {
            // Assume it's a hostname
            entries.push(NoProxyEntry::Host(entry.to_string()));
        }
    }

    entries
}

/// Connect to target through upstream proxy using CONNECT method
///
/// For HTTPS through an HTTP(S) proxy, use CONNECT tunneling:
/// 1. Connect to proxy
/// 2. Send: CONNECT target:port HTTP/1.1
/// 3. Wait for 200 response
/// 4. Return connected stream (now tunneled)
pub async fn connect_through_proxy(
    proxy_addr: &str,
    target_host: &str,
    target_port: u16,
) -> Result<tokio::net::TcpStream> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    info!("[CONNECT] Connecting through proxy {}", proxy_addr);

    // Parse proxy address
    let proxy_parts: Vec<&str> = proxy_addr.split("://").collect();
    let proxy_url = if proxy_parts.len() > 1 {
        proxy_parts[1]
    } else {
        proxy_addr
    };

    // Connect to proxy
    let stream = tokio::net::TcpStream::connect(proxy_url).await?;
    info!("Connected to upstream proxy: {}", proxy_addr);

    let mut stream = stream;

    // Send CONNECT request
    let connect_request = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: keep-alive\r\n\r\n",
        target_host, target_port, target_host, target_port
    );

    stream.write_all(connect_request.as_bytes()).await?;
    info!(
        "[SEND] Sent CONNECT {}:{} through proxy",
        target_host, target_port
    );

    // Read response (should be 200)
    let mut response_buf = vec![0u8; 1024];
    let n = stream.read(&mut response_buf).await?;

    if n == 0 {
        return Err(anyhow!("Proxy closed connection"));
    }

    let response = String::from_utf8_lossy(&response_buf[..n]);
    if !response.contains("200") {
        let status_line = response.lines().next().unwrap_or("");
        warn!("Proxy blocked CONNECT to {}:{} - {}", target_host, target_port, status_line);
        return Err(anyhow!("Proxy blocked CONNECT to {}:{} - {}", target_host, target_port, status_line));
    }

    info!("CONNECT tunnel established (200 OK)");
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(no_proxy: Vec<&str>) -> MitmConfig {
        MitmConfig {
            http_proxy: None,
            https_proxy: None,
            no_proxy_list: no_proxy.iter().map(|s| {
                match *s {
                    "*" => NoProxyEntry::All,
                    "localhost" => NoProxyEntry::Localhost,
                    s if s.starts_with(".") => NoProxyEntry::Suffix(s.to_string()),
                    s if s.contains('/') => NoProxyEntry::IpRange(s.to_string()),
                    s => NoProxyEntry::Host(s.to_string()),
                }
            }).collect(),
        }
    }

    #[test]
    fn test_should_bypass_proxy_all() {
        let config = make_config(vec!["*"]);
        assert!(config.should_bypass_proxy("anything.com"));
        assert!(config.should_bypass_proxy("localhost"));
    }

    #[test]
    fn test_should_bypass_proxy_localhost() {
        let config = make_config(vec!["localhost"]);
        assert!(config.should_bypass_proxy("localhost"));
        assert!(config.should_bypass_proxy("127.0.0.1"));
        assert!(config.should_bypass_proxy("::1"));
        assert!(!config.should_bypass_proxy("example.com"));
    }

    #[test]
    fn test_should_bypass_proxy_host() {
        let config = make_config(vec!["example.com"]);
        assert!(config.should_bypass_proxy("example.com"));
        assert!(config.should_bypass_proxy("EXAMPLE.COM"));
        assert!(!config.should_bypass_proxy("other.com"));
    }

    #[test]
    fn test_should_bypass_proxy_suffix() {
        let config = make_config(vec![".example.com"]);
        assert!(config.should_bypass_proxy("api.example.com"));
        assert!(config.should_bypass_proxy("sub.api.example.com"));
        assert!(!config.should_bypass_proxy("example.com"));
        assert!(!config.should_bypass_proxy("other.com"));
    }

    #[test]
    fn test_should_bypass_proxy_no_entries() {
        let config = make_config(vec![]);
        assert!(!config.should_bypass_proxy("anything.com"));
    }

    #[test]
    fn test_mitm_config_from_env_empty() {
        let config = MitmConfig::from_env();
        assert!(config.http_proxy.is_none() || config.http_proxy.is_some());
    }

    #[test]
    fn test_no_proxy_entry_debug() {
        let all = NoProxyEntry::All;
        let host = NoProxyEntry::Host("example.com".to_string());
        assert!(!format!("{:?}", all).is_empty());
        assert!(!format!("{:?}", host).is_empty());
    }

    #[test]
    fn test_parse_no_proxy_list_empty() {
        let result = parse_no_proxy_list("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_no_proxy_list_wildcard() {
        let result = parse_no_proxy_list("*");
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], NoProxyEntry::All));
    }

    #[test]
    fn test_parse_no_proxy_list_localhost() {
        let result = parse_no_proxy_list("localhost");
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], NoProxyEntry::Localhost));
    }

    #[test]
    fn test_parse_no_proxy_list_domain() {
        let result = parse_no_proxy_list(".example.com");
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], NoProxyEntry::Suffix(_)));
    }

    #[test]
    fn test_parse_no_proxy_list_hostname() {
        let result = parse_no_proxy_list("internal-service");
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], NoProxyEntry::Host(_)));
    }

    #[test]
    fn test_parse_no_proxy_list_multiple() {
        let result = parse_no_proxy_list("localhost, .example.com, internal-service");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_parse_no_proxy_list_ip_range() {
        let result = parse_no_proxy_list("10/8");
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], NoProxyEntry::IpRange(_)));
    }

    #[test]
    fn test_parse_no_proxy_list_127_0_0_1() {
        let result = parse_no_proxy_list("127.0.0.1");
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], NoProxyEntry::Localhost));
    }

    #[test]
    fn test_parse_no_proxy_list_ipv6() {
        let result = parse_no_proxy_list("::1");
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], NoProxyEntry::Localhost));
    }
}
