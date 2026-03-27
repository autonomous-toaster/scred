/// Proxy Environment Variable Resolution
/// 
/// Respects standard environment variables:
///   - http_proxy / HTTP_PROXY
///   - https_proxy / HTTPS_PROXY
///   - no_proxy / NO_PROXY
///
/// When SCRED connects to upstream servers, it checks these env vars
/// to determine if it should route through an intermediate proxy.

use tracing::{debug, info, error};
use anyhow::{Result, anyhow};

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
    All,  // NEW: Matches all hosts (NO_PROXY=*)
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
            info!("Proxy environment variables detected");
            if http_proxy.is_some() {
                debug!("http_proxy: (set)");
            }
            if https_proxy.is_some() {
                debug!("https_proxy: (set)");
            }
            if !no_proxy_list.is_empty() {
                debug!("no_proxy: {} entries", no_proxy_list.len());
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
    info!("[SEND] Sent CONNECT {}:{} through proxy", target_host, target_port);

    // Read response (should be 200)
    let mut response_buf = vec![0u8; 1024];
    let n = stream.read(&mut response_buf).await?;

    if n == 0 {
        return Err(anyhow!("Proxy closed connection"));
    }

    let response = String::from_utf8_lossy(&response_buf[..n]);
    if !response.contains("200") {
        error!("Proxy rejected CONNECT: {}", response);
        return Err(anyhow!("Proxy rejected CONNECT (not 200 response)"));
    }

    info!("CONNECT tunnel established (200 OK)");
    Ok(stream)
}

