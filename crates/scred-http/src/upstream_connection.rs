use anyhow::{anyhow, Result};
use rustls::{ClientConfig, ServerName};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{info, warn};

/// Configuration for upstream connection
#[derive(Debug, Clone)]
pub struct UpstreamConnectionConfig {
    /// Target host (for SNI)
    pub host: String,
    /// Target port
    pub port: u16,
    /// Use TLS
    pub use_tls: bool,
    /// Upstream proxy URL (e.g., "http://proxy.example.com:3128")
    pub proxy_url: Option<String>,
}

impl UpstreamConnectionConfig {
    /// Create config for direct HTTPS connection
    pub fn https(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            use_tls: true,
            proxy_url: None,
        }
    }

    /// Create config for direct HTTP connection
    pub fn http(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            use_tls: false,
            proxy_url: None,
        }
    }

    /// Add upstream proxy
    pub fn with_proxy(mut self, proxy_url: &str) -> Self {
        self.proxy_url = Some(proxy_url.to_string());
        self
    }

    /// Check if proxy is configured
    pub fn has_proxy(&self) -> bool {
        self.proxy_url.is_some()
    }
}

/// Connect to upstream server through proxy if configured
///
/// Returns a TCP stream (either direct or through CONNECT tunnel)
pub async fn connect_tcp(config: &UpstreamConnectionConfig) -> Result<TcpStream> {
    let target_addr = format!("{}:{}", config.host, config.port);

    if let Some(ref proxy_url) = config.proxy_url {
        info!(
            "[Upstream] Connecting through proxy: {} -> {}",
            proxy_url, target_addr
        );
        connect_through_proxy(proxy_url, &config.host, config.port).await
    } else {
        info!("[Upstream] Direct TCP connection to {}", target_addr);
        TcpStream::connect(&target_addr)
            .await
            .map_err(|e| anyhow!("Failed to connect to {}: {}", target_addr, e))
    }
}

/// Connect through upstream proxy using CONNECT method
pub async fn connect_through_proxy(
    proxy_url: &str,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    // Parse proxy URL (http://host:port or just host:port)
    let proxy_addr = proxy_url
        .strip_prefix("http://")
        .or_else(|| proxy_url.strip_prefix("https://"))
        .unwrap_or(proxy_url);

    info!("[CONNECT] Connecting to proxy: {}", proxy_addr);

    let mut stream = TcpStream::connect(proxy_addr)
        .await
        .map_err(|e| anyhow!("Failed to connect to proxy {}: {}", proxy_addr, e))?;

    // Send CONNECT request
    let connect_request = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: keep-alive\r\n\r\n",
        target_host, target_port, target_host, target_port
    );
    stream.write_all(connect_request.as_bytes()).await?;

    // Read response (should be 200)
    let mut response_buf = vec![0u8; 1024];
    let n = stream.read(&mut response_buf).await?;
    if n == 0 {
        return Err(anyhow!("Proxy closed connection"));
    }

    let response = String::from_utf8_lossy(&response_buf[..n]);
    if !response.contains("200") {
        let status_line = response.lines().next().unwrap_or("");
        warn!("Proxy rejected CONNECT: {}", status_line);
        return Err(anyhow!("Proxy rejected CONNECT: {}", status_line));
    }

    info!(
        "[CONNECT] Tunnel established to {}:{}",
        target_host, target_port
    );
    Ok(stream)
}

/// Establish TLS connection to upstream server with ALPN configuration
///
/// # Arguments
/// * `tcp_stream` - TCP stream (may be through CONNECT tunnel)
/// * `host` - Hostname for SNI
/// * `advertise_h2` - If true, advertise h2+http/1.1. If false, only http/1.1.
pub async fn establish_tls_with_alpn(
    tcp_stream: TcpStream,
    host: &str,
    advertise_h2: bool,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    // Use standard environment variables for custom CA certificates
    let root_store = crate::tls_roots::build_root_cert_store();
    let mut client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // Configure ALPN based on caller's capability
    client_config.alpn_protocols = if advertise_h2 {
        vec![b"h2".to_vec(), b"http/1.1".to_vec()]
    } else {
        vec![b"http/1.1".to_vec()]
    };

    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));

    let server_name =
        ServerName::try_from(host).map_err(|_| anyhow::anyhow!("Invalid server name: {}", host))?;

    let tls_stream = connector
        .connect(server_name, tcp_stream)
        .await
        .map_err(|e| anyhow!("TLS handshake failed for {}: {}", host, e))?;

    info!(
        "[TLS] Established TLS connection to {} (H2 ALPN: {})",
        host, advertise_h2
    );
    Ok(tls_stream)
}

/// Establish TLS connection (HTTP/1.1 only - convenience wrapper)
pub async fn establish_tls(
    tcp_stream: TcpStream,
    host: &str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    establish_tls_with_alpn(tcp_stream, host, false).await
}

/// Establish TLS connection with H2 support
pub async fn establish_tls_h2(
    tcp_stream: TcpStream,
    host: &str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    establish_tls_with_alpn(tcp_stream, host, true).await
}

/// Connect to upstream with full proxy/TLS handling
///
/// This is the main entry point that handles:
/// 1. Proxy CONNECT tunneling (if configured)
/// 2. TLS establishment (if https)
/// 3. ALPN protocol negotiation
///
/// Returns (tls_stream, alpn_protocol)
pub async fn connect_upstream_tls(
    config: &UpstreamConnectionConfig,
) -> Result<(tokio_rustls::client::TlsStream<TcpStream>, Option<String>)> {
    connect_upstream_tls_with_alpn(config, false).await
}

/// Connect to upstream with H2 ALPN support
pub async fn connect_upstream_tls_h2(
    config: &UpstreamConnectionConfig,
) -> Result<(tokio_rustls::client::TlsStream<TcpStream>, Option<String>)> {
    connect_upstream_tls_with_alpn(config, true).await
}

async fn connect_upstream_tls_with_alpn(
    config: &UpstreamConnectionConfig,
    advertise_h2: bool,
) -> Result<(tokio_rustls::client::TlsStream<TcpStream>, Option<String>)> {
    let tcp_stream = connect_tcp(config).await?;

    if !config.use_tls {
        return Err(anyhow!("connect_upstream_tls called with use_tls=false"));
    }

    let tls_stream = establish_tls_with_alpn(tcp_stream, &config.host, advertise_h2).await?;

    // Extract ALPN protocol
    let alpn = tls_stream
        .get_ref()
        .1
        .alpn_protocol()
        .map(|p| String::from_utf8_lossy(p).to_string());

    info!(
        "[Upstream] TLS established for {} (ALPN: {})",
        config.host,
        alpn.as_deref().unwrap_or("none")
    );

    Ok((tls_stream, alpn))
}

/// Get proxy URL from environment variables
///
/// Respects standard environment variables:
/// - https_proxy / HTTPS_PROXY for HTTPS targets
/// - http_proxy / HTTP_PROXY for HTTP targets
/// - no_proxy / NO_PROXY for bypassing proxy
pub fn get_proxy_url(target_host: &str, is_https: bool) -> Option<String> {
    use crate::proxy_resolver::MitmConfig;

    let resolver = MitmConfig::from_env();
    resolver.get_proxy_for(target_host, is_https)
}

/// Check if a proxy is configured
pub fn has_proxy_configured() -> bool {
    std::env::var("http_proxy")
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTPS_PROXY"))
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}
