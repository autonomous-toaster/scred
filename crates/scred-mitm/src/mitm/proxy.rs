use crate::mitm::config::Config;
use crate::mitm::config::TrafficPolicy;
use crate::mitm::tls::CertificateGenerator;
use anyhow::Result;
use scred_policy::PolicyEngine;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

pub struct ProxyServer {
    config: Config,
    cert_generator: Arc<CertificateGenerator>,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
    pool: Arc<scred_http::MultiUpstreamPool>,
    resolver: Arc<scred_http::OptimizedDnsResolver>,
    traffic_policy: Arc<TrafficPolicy>,
    policy: Option<Arc<PolicyEngine>>,
}

impl ProxyServer {
    pub fn new(config: &Config, policy: Option<Arc<PolicyEngine>>) -> Result<Self> {
        // Auto-generate CA certificate if missing
        CertificateGenerator::generate_ca_if_missing(
            std::path::Path::new(&config.tls.ca_key),
            std::path::Path::new(&config.tls.ca_cert),
        )?;

        let cert_generator = CertificateGenerator::new(
            std::path::Path::new(&config.tls.ca_key),
            std::path::Path::new(&config.tls.ca_cert),
            std::path::Path::new(&config.tls.cert_cache_dir),
        )?;

        let redaction_engine =
            scred_redactor::RedactionEngine::new(scred_redactor::RedactionConfig { enabled: true });

        let traffic_policy = config.traffic.into_policy()?;

        if traffic_policy.enabled {
            info!(
                "Traffic filtering enabled: {:?}",
                traffic_policy.allowed_domains
            );
        }

        Ok(Self {
            config: config.clone(),
            cert_generator: Arc::new(cert_generator),
            redaction_engine: Arc::new(redaction_engine),
            pool: Arc::new(scred_http::MultiUpstreamPool::new()),
            resolver: Arc::new(scred_http::OptimizedDnsResolverBuilder::new().build()),
            traffic_policy: Arc::new(traffic_policy),
            policy,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.proxy.listen).await?;
        info!("MITM Proxy listening on: {}", self.config.proxy.listen);

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            debug!("New connection from {}", peer_addr);

            let config = self.config.clone();
            let cert_gen = self.cert_generator.clone();
            let redaction = self.redaction_engine.clone();
            let pool = self.pool.clone();
            let resolver = self.resolver.clone();
            let traffic_policy = self.traffic_policy.clone();
            let policy = self.policy.clone();
            let upstream_resolver = Arc::new(scred_http::proxy_resolver::MitmConfig::from_env());

            tokio::spawn(async move {
                if let Err(e) = handle_client(
                    socket,
                    peer_addr,
                    upstream_resolver,
                    cert_gen,
                    redaction,
                    config,
                    pool,
                    resolver,
                    traffic_policy,
                    policy,
                )
                .await
                {
                    warn!("Error handling client {}: {}", peer_addr, e);
                }
            });
        }
    }
}

/// Read first line from socket (up to newline or 1024 bytes)
async fn read_first_line<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> std::io::Result<Option<String>> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    
    loop {
        match reader.read_exact(&mut byte).await {
            Ok(0) => return Ok(None),
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
                if buf.len() > 1024 {
                    return Ok(None);
                }
            }
            Err(_) => return Ok(None),
        }
    }
    
    let line = String::from_utf8_lossy(&buf).trim().to_string();
    if line.is_empty() {
        return Ok(None);
    }
    Ok(Some(line))
}

/// Consume HTTP headers after CONNECT (read until \r\n\r\n)
async fn consume_connect_headers<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> std::io::Result<()> {
    let mut byte = [0u8; 1];
    let mut buf = [0u8; 4];
    
    loop {
        match reader.read_exact(&mut byte).await {
            Ok(0) => return Ok(()),
            Ok(_) => {
                buf[0] = buf[1];
                buf[1] = buf[2];
                buf[2] = buf[3];
                buf[3] = byte[0];
                if buf[0] == b'\r' && buf[1] == b'\n' && buf[2] == b'\r' && buf[3] == b'\n' {
                    return Ok(());
                }
            }
            Err(e) => return Err(e),
        }
    }
}

/// Handle CONNECT request: establish TLS MITM tunnel
#[allow(clippy::too_many_arguments)]
async fn handle_connect_request(
    host: &str,
    port: u16,
    upstream_addr: &str,
    mut socket_read: tokio::net::tcp::OwnedReadHalf,
    mut socket_write: tokio::net::tcp::OwnedWriteHalf,
    cert_generator: Arc<CertificateGenerator>,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
    config: &Config,
    policy: Option<Arc<PolicyEngine>>,
) -> Result<()> {
    debug!(
        "[PROXY] CONNECT tunnel: {} -> {} (upstream_addr: '{}')",
        host, port, upstream_addr
    );
    
    socket_write
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;
    socket_write.flush().await?;
    
    if let Err(e) = crate::mitm::tls_mitm::handle_tls_mitm(
        socket_read,
        socket_write,
        host,
        port,
        upstream_addr,
        cert_generator,
        redaction_engine,
        config.proxy.redaction_mode,
        config.proxy.h2_redact_headers,
        config.proxy.detect_patterns.clone(),
        config.proxy.redact_patterns.clone(),
        policy,
    ).await {
        warn!("TLS MITM error: {}", e);
    }
    
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_client(
    socket: TcpStream,
    peer_addr: SocketAddr,
    upstream_resolver: Arc<scred_http::proxy_resolver::MitmConfig>,
    cert_generator: Arc<CertificateGenerator>,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
    config: Config,
    _pool: Arc<scred_http::MultiUpstreamPool>,
    resolver: Arc<scred_http::OptimizedDnsResolver>,
    traffic_policy: Arc<TrafficPolicy>,
    policy: Option<Arc<PolicyEngine>>,
) -> Result<()> {
    let (mut socket_read, mut socket_write) = socket.into_split();

    // Read first line
    let line = match read_first_line(&mut socket_read).await {
        Ok(Some(line)) => line,
        _ => return Ok(()),
    };

    if line.starts_with("CONNECT ") {
        debug!("CONNECT request from {}", peer_addr);

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            send_error_response(&mut socket_write, 400, "Bad Request").await?;
            return Err(anyhow::anyhow!("Invalid CONNECT format"));
        }

        let (host, port) = scred_http::connect::parse_host_port(parts[1])
            .map_err(|e| anyhow::anyhow!("Failed to parse host:port: {}", e))?;

        if !traffic_policy.is_allowed(&host) {
            info!("Blocked CONNECT to {}: domain not allowed", host);
            send_error_response(&mut socket_write, 403, &traffic_policy.block_message).await?;
            return Ok(());
        }

        // Consume headers after CONNECT
        consume_connect_headers(&mut socket_read).await?;

        // Determine upstream destination
        let upstream_addr = if let Some(upstream) = upstream_resolver.get_proxy_for(&host, true) {
            debug!("Routing through upstream proxy: {}", upstream);
            upstream
        } else {
            format!("{}:{}", host, port)
        };

        // Handle CONNECT tunnel (consumes socket)
        return handle_connect_request(
            &host,
            port,
            &upstream_addr,
            socket_read,
            socket_write,
            cert_generator,
            redaction_engine,
            &config,
            policy,
        ).await
    } else {
        // Handle HTTP proxy requests (non-CONNECT)
        debug!("HTTP proxy request from {}: {}", peer_addr, line);

        // Extract host from request for traffic filtering
        if let Some(host) = extract_host_from_request(&line) {
            if !traffic_policy.is_allowed(&host) {
                info!("Blocked HTTP request to {}: domain not allowed", host);
                send_error_response(&mut socket_write, 403, &traffic_policy.block_message).await?;
                return Ok(());
            }
        }

        if let Err(e) = crate::mitm::http_handler::handle_http_proxy(
            socket_read,
            socket_write,
            &line,
            redaction_engine.clone(),
            upstream_resolver.clone(),
            Some(config.proxy.redact_patterns.clone()),
            resolver.clone(),
        ).await {
            warn!("HTTP proxy handler error: {}", e);
        }
        Ok(()) // Connection consumed by HTTP handler
    }
}

fn extract_host_from_request(request_line: &str) -> Option<String> {
    // HTTP request format: METHOD http://host/path HTTP/1.1
    // or: METHOD /path HTTP/1.1 (requires Host header, not parsed here)
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() >= 2 {
        let url = parts[1];
        // Check if it's a full URL
        if url.starts_with("http://") || url.starts_with("https://") {
            // Parse URL to extract host
            let url = url.strip_prefix("http://").unwrap_or(url);
            let url = url.strip_prefix("https://").unwrap_or(url);
            let host_port = url.split('/').next()?;
            let host = host_port.split(':').next()?;
            return Some(host.to_string());
        }
    }
    None
}

async fn send_error_response(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    code: u16,
    reason: &str,
) -> Result<()> {
    let response = format!("HTTP/1.1 {} {}\r\nContent-Length: 0\r\n\r\n", code, reason);
    writer.write_all(response.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_host_from_request_full_url_http() {
        let result = extract_host_from_request("GET http://example.com/path HTTP/1.1");
        assert_eq!(result, Some("example.com".to_string()));
    }

    #[test]
    fn test_extract_host_from_request_full_url_https() {
        let result = extract_host_from_request("GET https://example.com:8443/path HTTP/1.1");
        assert_eq!(result, Some("example.com".to_string()));
    }

    #[test]
    fn test_extract_host_from_request_relative_path() {
        let result = extract_host_from_request("GET /path HTTP/1.1");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_host_from_request_empty() {
        let result = extract_host_from_request("");
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_read_first_line_basic() {
        let data = b"GET /path HTTP/1.1\r\n";
        let mut reader = &data[..];
        let line = read_first_line(&mut reader).await.unwrap();
        assert_eq!(line, Some("GET /path HTTP/1.1".to_string()));
    }

    #[tokio::test]
    async fn test_read_first_line_empty() {
        let data = b"";
        let mut reader = &data[..];
        let line = read_first_line(&mut reader).await.unwrap();
        assert_eq!(line, None);
    }

    #[tokio::test]
    async fn test_read_first_line_too_long() {
        let data = vec![b'a'; 2000];
        let mut reader = &data[..];
        let line = read_first_line(&mut reader).await.unwrap();
        assert_eq!(line, None);
    }
}
