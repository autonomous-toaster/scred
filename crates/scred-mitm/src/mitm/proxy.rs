use tokio::net::{TcpListener, TcpStream};
use std::sync::Arc;
use anyhow::Result;
use tracing::{debug, error, info, warn};
use crate::mitm::tls::CertificateGenerator;
use crate::mitm::config::Config;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct ProxyServer {
    config: Config,
    cert_generator: Arc<CertificateGenerator>,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
}

impl ProxyServer {
    pub fn new(config: &Config) -> Result<Self> {
        let cert_generator = CertificateGenerator::new(
            std::path::Path::new(&config.tls.ca_key),
            std::path::Path::new(&config.tls.ca_cert),
            std::path::Path::new(&config.tls.cert_cache_dir),
        )?;
        
        let redaction_engine = scred_redactor::RedactionEngine::new(
            scred_redactor::RedactionConfig {
                enabled: true,
            },
        );

        Ok(Self {
            config: config.clone(),
            cert_generator: Arc::new(cert_generator),
            redaction_engine: Arc::new(redaction_engine),
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

            let upstream_resolver = Arc::new(scred_http::proxy_resolver::MitmConfig::from_env());

            tokio::spawn(async move {
                if let Err(e) = handle_client(socket, peer_addr, upstream_resolver, cert_gen, redaction, config).await {
                    warn!("Error handling client {}: {}", peer_addr, e);
                }
            });
        }
    }
}

async fn handle_client(
    socket: TcpStream,
    peer_addr: SocketAddr,
    upstream_resolver: Arc<scred_http::proxy_resolver::MitmConfig>,
    cert_generator: Arc<CertificateGenerator>,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
    config: Config,
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
                        let _ = send_error_response(&mut socket_write, 413, "Request Line Too Long").await;
                        return Ok(()); // Exit gracefully on oversized line
                    }
                }
                Err(e) => {
                    debug!("Client connection closed or error: {}", e);
                    return Ok(()); // Exit gracefully
                }
            }
        }
        
        let line = String::from_utf8_lossy(&first_line_buf)
            .trim()
            .to_string();

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

            let (host, port) = scred_http::connect::parse_host_port(parts[1])
                .map_err(|e| anyhow::anyhow!("Failed to parse host:port: {}", e))?;

            info!("CONNECT {}:{} from {}", host, port, peer_addr);

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

            // Determine upstream destination
            let upstream_addr = if let Some(upstream) = upstream_resolver.get_proxy_for(&host, true) {
                debug!("Routing through upstream proxy: {}", upstream);
                upstream
            } else {
                format!("{}:{}", host, port)
            };
            
            info!("[PROXY] CONNECT tunnel: {} -> {} (upstream_addr will be: '{}')", peer_addr, host, upstream_addr);

            // Send 200 Connection Established BEFORE doing TLS!
            // Client is waiting for this before upgrading to TLS
            socket_write.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
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
                config.proxy.redact_responses,
                config.proxy.h2_redact_headers,
            ).await {
                warn!("TLS MITM error: {}", e);
            }
            // After TLS MITM, connection is consumed, exit loop
            return Ok(());
        } else {
            // Handle HTTP proxy requests (non-CONNECT)
            debug!("HTTP proxy request from {}: {}", peer_addr, line);

            if let Err(e) = crate::mitm::http_handler::handle_http_proxy(
                socket_read,
                socket_write,
                &line,
                redaction_engine.clone(),
                upstream_resolver.clone(),
            ).await {
                warn!("HTTP proxy handler error: {}", e);
            }

            return Ok(()); // Connection consumed by HTTP handler
        }
    }
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
    fn test_proxy_server_structure() {
        let size = std::mem::size_of::<ProxyServer>();
        assert!(size > 0, "ProxyServer should have non-zero size");
    }
}
