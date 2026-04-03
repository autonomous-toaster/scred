use crate::mitm::config::Config;
use crate::mitm::config::TrafficPolicy;
use crate::mitm::tls::CertificateGenerator;
use scred_policy::PolicyEngine;
use anyhow::Result;
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

        let redaction_engine = scred_redactor::RedactionEngine::new(scred_redactor::RedactionConfig {
            enabled: true,
        });

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

    // Keep-alive loop: handle multiple requests on same connection
    loop {
        // Read first line manually WITHOUT buffering
        let mut first_line_buf = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            match socket_read.read_exact(&mut byte).await {
                Ok(0) => return Ok(()), // Connection closed gracefully
                Ok(_) => {
                    first_line_buf.push(byte[0]);
                    if byte[0] == b'\n' {
                        break;
                    }
                    if first_line_buf.len() > 1024 {
                        let _ = send_error_response(&mut socket_write, 413, "Request Line Too Long")
                            .await;
                        return Ok(()); // Exit gracefully on oversized line
                    }
                }
                Err(e) => {
                    debug!("Client connection closed or error: {}", e);
                    return Ok(()); // Exit gracefully
                }
            }
        }

        let line = String::from_utf8_lossy(&first_line_buf).trim().to_string();

        // Skip empty lines (can happen with Connection: close or timing issues)
        if line.is_empty() {
            debug!("Empty request line, closing connection");
            return Ok(());
        }

        if line.starts_with("CONNECT ") {
            debug!("CONNECT request from {}", peer_addr);

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                send_error_response(&mut socket_write, 400, "Bad Request").await?;
                return Err(anyhow::anyhow!("Invalid CONNECT format"));
            }

            let (host, _port) = scred_http::connect::parse_host_port(parts[1])
                .map_err(|e| anyhow::anyhow!("Failed to parse host:port: {}", e))?;

                        if !traffic_policy.is_allowed(&host) {
                info!("Blocked CONNECT to {}: domain not allowed", host);
                send_error_response(&mut socket_write, 403, &traffic_policy.block_message).await?;
                return Ok(());
            }

            debug!("CONNECT {} from {}", parts[1], peer_addr);

            // Read headers until blank line without buffering
            // We need to consume the \r\n\r\n sequence that terminates HTTP headers
            let mut buf = [0u8; 4];
            buf[0] = 0;
            buf[1] = 0;
            buf[2] = 0;
            buf[3] = 0;

            // Keep sliding window of last 4 bytes to detect \r\n\r\n
            loop {
                match socket_read.read_exact(&mut byte).await {
                    Ok(0) => break,
                    Ok(_) => {
                        buf[0] = buf[1];
                        buf[1] = buf[2];
                        buf[2] = buf[3];
                        buf[3] = byte[0];

                        // Check if we have \r\n\r\n
                        if buf[0] == b'\r' && buf[1] == b'\n' && buf[2] == b'\r' && buf[3] == b'\n' {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to read headers: {}", e);
                        return Err(e.into());
                    }
                }
            }

            // Parse host:port and determine upstream
            let (host, port) = scred_http::connect::parse_host_port(parts[1])
                .map_err(|e| anyhow::anyhow!("Failed to parse host:port: {}", e))?;

            // Determine upstream destination
            let upstream_addr = if let Some(upstream) = upstream_resolver.get_proxy_for(&host, true) {
                debug!("Routing through upstream proxy: {}", upstream);
                upstream
            } else {
                format!("{}:{}", host, port)
            };

            debug!(
                "[PROXY] CONNECT tunnel: {} -> {} (upstream_addr will be: '{}')",
                peer_addr, host, upstream_addr
            );

            // Send 200 Connection Established BEFORE doing TLS!
            // Client is waiting for this before upgrading to TLS
            socket_write
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .await?;
            socket_write.flush().await?;

            // Now do TLS MITM interception (decrypt, redact, re-encrypt)
            // This consumes the socket and doesn't return for keep-alive
            if let Err(e) = crate::mitm::tls_mitm::handle_tls_mitm(
                socket_read,
                socket_write,
                &host,
                port,
                &upstream_addr,
                cert_generator.clone(),
                redaction_engine.clone(),
                config.proxy.redaction_mode,
                config.proxy.h2_redact_headers,
                config.proxy.detect_patterns.clone(),
                config.proxy.redact_patterns.clone(),
                policy,
            )
            .await
            {
                warn!("TLS MITM error: {}", e);
            }

            // After TLS MITM, connection is consumed, exit loop
            return Ok(());
        } else {
            // Handle HTTP proxy requests (non-CONNECT)
            debug!("HTTP proxy request from {}: {}", peer_addr, line);

            // Extract host from HTTP request for traffic filtering
                        {
                // Parse the host from the request line or headers
                // For simplicity, we check common patterns
                let host = extract_host_from_request(&line);
                if let Some(host) = host {
                    if !traffic_policy.is_allowed(&host) {
                        info!("Blocked HTTP request to {}: domain not allowed", host);
                        send_error_response(&mut socket_write, 403, &traffic_policy.block_message)
                            .await?;
                        return Ok(());
                    }
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
            )
            .await
            {
                warn!("HTTP proxy handler error: {}", e);
            }
            return Ok(()); // Connection consumed by HTTP handler
        }
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
