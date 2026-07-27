#![allow(clippy::all)]
/// TLS MITM Bridge - REAL TLS Client Acceptance (Phase 6: Full Streaming)
///
/// This is the Phase 6 implementation with full streaming support:
/// 1. Accepts TLS FROM the client (using generated certificate)
/// 2. Decrypts HTTP request to plain text
/// 3. Streams request body directly to upstream (no buffering)
/// 4. Applies SCRED redaction per-chunk
/// 5. Streams response back to client
/// 6. Supports HTTP/1.1 keep-alive (multiple requests per connection)
///
/// Phase 6: Streaming-first architecture with unlimited request/response sizes

use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use std::sync::Arc;
use tracing::{debug, info, warn, error};
use rustls::{ServerConfig, Certificate, PrivateKey};
use std::io::Cursor;

use super::tls::CertificateGenerator;
use scred_http::dns_resolver::DnsResolver;
use scred_http::duplex::DuplexSocket;
use scred_http::http_line_reader::{read_request_line, read_response_line};
use scred_http::proxy_resolver::connect_through_proxy;
use scred_http::streaming_request::{stream_request_to_upstream, StreamingRequestConfig};
use rustls::{ClientConfig, RootCertStore, ServerName};
use tokio_rustls::TlsConnector;
use scred_http::h2::alpn::HttpProtocol;
use scred_http::upstream_h2_client::{extract_upstream_protocol, UpstreamConnectionInfo};
use scred_redactor::StreamingRedactor;
use scred_http::streaming_response::{stream_response_to_client, StreamingResponseConfig};

/// Load and parse TLS certificate and private key from PEM data
fn load_tls_certificate(cert_pem: &[u8], key_pem: &[u8]) -> Result<(Vec<Certificate>, PrivateKey)> {
    let mut cert_reader = Cursor::new(cert_pem);
    let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow!("Failed to parse certificate: {}", e))?;

    if certs.is_empty() {
        return Err(anyhow!("No certificates found in PEM"));
    }

    let cert_chain: Vec<Certificate> = certs
        .into_iter()
        .map(|c| Certificate(c.as_ref().to_vec()))
        .collect();

    let mut key_reader = Cursor::new(key_pem);
    let parsed_keys: Vec<_> = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;

    if parsed_keys.is_empty() {
        return Err(anyhow!("No private keys found in PEM"));
    }

    let private_key = PrivateKey(parsed_keys[0].secret_pkcs8_der().to_vec());
    Ok((cert_chain, private_key))
}

/// Build TLS ServerConfig with ALPN protocols
fn build_tls_server_config(
    cert_chain: Vec<Certificate>,
    private_key: PrivateKey,
) -> Result<ServerConfig> {
    let mut server_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .map_err(|e| anyhow!("Failed to build TLS config: {}", e))?;

    use scred_http::h2::alpn::alpn_protocols;
    server_config.alpn_protocols = alpn_protocols();
    Ok(server_config)
}

/// Accept TLS from client and extract negotiated ALPN protocol
async fn accept_client_tls(
    acceptor: &tokio_rustls::TlsAcceptor,
    duplex: DuplexSocket<tokio::net::tcp::OwnedReadHalf, tokio::net::tcp::OwnedWriteHalf>,
) -> Result<(impl AsyncReadExt + AsyncWriteExt + Unpin, HttpProtocol)> {
    debug!("Accepting TLS connection from client...");
    let client_tls = acceptor.accept(duplex).await
        .map_err(|e| {
            error!("Client TLS handshake failed: {}", e);
            anyhow!("Client TLS handshake failed: {}", e)
        })?;

    let negotiated_protocol = client_tls.get_ref().1.alpn_protocol()
        .and_then(|proto| HttpProtocol::from_bytes(proto))
        .unwrap_or(HttpProtocol::Http11);

    info!(
        "Client TLS handshake successful, HTTP decrypted, protocol: {}",
        negotiated_protocol
    );

    Ok((client_tls, negotiated_protocol))
}

/// Execute REAL TLS MITM with full streaming support (Phase 6)
///
/// This function implements the complete man-in-the-middle with streaming:
/// 1. Accept client TLS with generated certificate
/// 2. Stream HTTP requests directly (no buffering)
/// 3. Apply per-chunk redaction
/// 4. Forward to upstream
/// 5. Stream responses back to client
pub async fn handle_tls_mitm(
    client_read: tokio::net::tcp::OwnedReadHalf,
    client_write: tokio::net::tcp::OwnedWriteHalf,
    host: &str,
    _port: u16,
    upstream_addr: &str,
    cert_generator: Arc<CertificateGenerator>,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
    redaction_mode: crate::mitm::config::RedactionMode,
    h2_redact_headers: bool,
    detect_patterns: scred_http::PatternSelector,
    _redact_patterns: scred_http::PatternSelector,
    _policy: Option<Arc<scred_policy::PolicyEngine>>,
) -> Result<()> {
    
    
    info!("TLS MITM tunnel starting for: {}", host);

    // Step 1: Get or generate certificate for this domain
    let (cert_pem, key_pem) = cert_generator.get_or_generate_cert(host).await?;
    debug!("Certificate loaded/generated for: {}", host);

    // Step 2: Parse certificate and key for rustls
    let (cert_chain, private_key) = load_tls_certificate(&cert_pem, &key_pem)?;

    // Step 3: Build TLS ServerConfig
    let server_config = build_tls_server_config(cert_chain, private_key)?;
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

    // Step 4: Combine split socket halves using DuplexSocket
    let duplex = DuplexSocket::new(client_read, client_write);

    // Step 5: Accept TLS FROM client
    let (mut client_tls, negotiated_protocol) = accept_client_tls(&acceptor, duplex).await?;

    // Smart Routing: Handle HTTP/2 upstream based on client protocol and upstream type
    if negotiated_protocol.is_h2() {
        info!("H2 Client: Using H2 transcoding");
        return handle_h2_client_transcoding(
            client_tls,
            host,
            upstream_addr,
            redaction_engine.clone(),
            h2_redact_headers,
            redaction_mode,
            &detect_patterns,
            &_redact_patterns,
            _policy.clone(),
        ).await;
    }
    
    // Scenarios 1-2: HTTP/1.1 client (or H2 client via proxy)
    // Use existing transcoding path via H2UpstreamClient
    info!("HTTP/1.1 client path: Using H2UpstreamClient for any H2 upstream transcoding");
    
    // Phase 1 Fallback: Log streaming mode active
    log_redaction_mode();

    // Phase 6: Keep-alive loop - process multiple requests per connection
    'keep_alive: loop {
        debug!("Processing request in keep-alive loop");
        
        // Handle single request with full streaming support
        match handle_single_request(
            &mut client_tls,
            host,
            upstream_addr,
            redaction_engine.clone(),
            redaction_mode.should_redact(),
        ).await {
            Ok(should_close) => {
                if should_close {
                    debug!("Response requested connection close; ending MITM keep-alive loop");
                    break 'keep_alive;
                }
            }
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => {
                        debug!("Client closed connection (EOF received)");
                        break 'keep_alive;
                    }
                    _ => {
                        warn!("Request handling error: {}", e);
                        return Err(anyhow!("Request handling failed: {}", e));
                    }
                }
            }
        }
        
        debug!("Request complete, looping for next request");
    }

    info!("TLS MITM tunnel complete: all requests processed, connection closed by client");
    Ok(())
}


/// Handle a single HTTP request with full streaming support (Phase 6 Step 2+3)
///
/// This helper processes one complete request/response cycle with streaming:
/// 1. Read request line from client
/// 2. Stream request body directly to upstream (no buffering)
/// 3. Read response line from upstream
/// 4. Stream response body directly to client (no buffering)
/// 5. Apply per-chunk redaction with pattern detection
///
/// Returns Err with UnexpectedEof when client closes connection

/// Handle HTTP/2 connection preface downgrade
/// Detects H2 preface and skips SETTINGS frame, returning the actual HTTP/1.1 request line
async fn handle_h2_downgrade<RW: AsyncReadExt + AsyncWriteExt + Unpin>(
    client_tls: &mut RW,
    request_line: &str,
) -> std::io::Result<Option<String>> {
    if !request_line.starts_with("PRI * HTTP/2.0") {
        return Ok(None);
    }
    
    warn!(
        "Client sent HTTP/2 preface; initiating transparent downgrade to HTTP/1.1 (RFC 7540 Section 3.4)"
    );
    
    // Read and skip the SETTINGS frame
    let mut frame_header = [0u8; 9];
    match client_tls.read(&mut frame_header).await {
        Ok(n) if n == 9 => {
            let frame_len = ((frame_header[0] as u32) << 16) 
                          | ((frame_header[1] as u32) << 8) 
                          | (frame_header[2] as u32);
            if frame_len > 0 {
                let mut payload = vec![0u8; frame_len as usize];
                let _ = client_tls.read_exact(&mut payload).await;
            }
            debug!("Skipped HTTP/2 preface + SETTINGS frame ({} bytes payload)", frame_len);
        }
        Ok(n) => warn!("Only read {} bytes of frame header; continuing anyway", n),
        Err(e) => warn!("Failed to read h2 SETTINGS frame: {}; continuing anyway", e),
    }
    
    // Read the actual HTTP/1.1 request line
    let actual_line = read_request_line(client_tls).await?;
    if actual_line.is_empty() {
        warn!("No HTTP/1.1 request after h2 preface; closing connection");
        return Ok(Some(String::new()));
    }
    
    warn!("HTTP/2 downgrade successful; continuing with HTTP/1.1");
    Ok(Some(actual_line))
}

/// Connect to upstream server with TLS
async fn connect_upstream_tls(
    upstream_addr: &str,
    target_host: &str,
) -> std::io::Result<tokio_rustls::client::TlsStream<tokio::net::TcpStream>> {
    let is_upstream_proxy = upstream_addr.contains("://");
    
    let upstream_tcp = match if is_upstream_proxy {
        connect_through_proxy(upstream_addr, target_host, 443).await
    } else {
        DnsResolver::connect_with_retry(&format!("{}:443", target_host)).await
    } {
        Ok(stream) => {
            info!("Connected to upstream {}", upstream_addr);
            stream
        }
        Err(e) => {
            error!("Failed to connect to upstream {}: {}", upstream_addr, e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };
    
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));
    let mut client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    
    use scred_http::h2::alpn::alpn_protocols;
    client_config.alpn_protocols = alpn_protocols();
    
    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name = ServerName::try_from(target_host)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid upstream host"))?;
    
    info!("[TLS] Starting upstream TLS handshake with server_name={}", target_host);
    connector
        .connect(server_name, upstream_tcp)
        .await
        .map_err(|e| {
            error!("[TLS] Upstream TLS handshake FAILED: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, format!("upstream TLS failed: {}", e))
        })
}

/// Forward response body without redaction
async fn forward_response_no_redaction<U: AsyncReadExt + AsyncWriteExt + Unpin, C: AsyncReadExt + AsyncWriteExt + Unpin>(
    upstream: &mut U,
    client_tls: &mut C,
    response_line: &str,
) -> std::io::Result<()> {
    let mut upstream_buf_reader = BufReader::new(&mut *upstream);
    
    let headers = scred_http::http_headers::parse_http_headers(&mut upstream_buf_reader, true)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    client_tls.write_all(format!("{}\r\n", response_line).as_bytes()).await?;
    client_tls.write_all(headers.raw_headers.as_bytes()).await?;
    client_tls.write_all(b"\r\n").await?;
    
    let mut buffer = vec![0u8; 65536];
    loop {
        match upstream_buf_reader.get_mut().read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => client_tls.write_all(&buffer[..n]).await?,
            Err(e) => {
                warn!("Error reading response body: {}", e);
                return Err(e);
            }
        }
    }
    
    client_tls.flush().await
}

/// Handle HTTP/2 upstream request forwarding
/// Builds an H2 request from H1.1 request line and forwards via h2::client
async fn handle_h2_upstream_request<RW: AsyncWriteExt + Unpin>(
    client_tls: &mut RW,
    request_line: &str,
    target_host: &str,
    upstream_addr: &str,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
    redact_responses: bool,
) -> std::io::Result<bool> {
    use http::Request;
    use bytes::Bytes;
    use crate::mitm::config::RedactionMode;
    use crate::mitm::h2_upstream_forwarder::handle_upstream_h2_connection;
    
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", target_host)
        .body(Bytes::new())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
    let mode = if redact_responses {
        RedactionMode::Redact
    } else {
        RedactionMode::Passthrough
    };
    let detect_patterns = scred_http::PatternSelector::default();
    let redact_patterns = scred_http::PatternSelector::default();
    
    match handle_upstream_h2_connection(
        request,
        redaction_engine.clone(),
        upstream_addr.to_string(),
        target_host,
        mode,
        detect_patterns,
        redact_patterns,
    ).await {
        Ok(response_body) => {
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", 
                response_body.len(), String::from_utf8_lossy(&response_body));
            client_tls.write_all(response.as_bytes()).await?;
            client_tls.flush().await?;
            Ok(true)
        }
        Err(e) => {
            error!("[HTTP/2 Upstream] Failed to forward request: {}", e);
            client_tls.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
            client_tls.flush().await?;
            Ok(true)
        }
    }
}

/// Stream request to upstream with redaction
async fn stream_request_with_redaction<RW: AsyncReadExt + AsyncWriteExt + Unpin>(
    client_tls: &mut RW,
    upstream: &mut (impl AsyncReadExt + AsyncWriteExt + Unpin),
    request_line: &str,
    redactor: Arc<StreamingRedactor>,
) -> std::io::Result<()> {
    let request_config = StreamingRequestConfig::default();
    let mut client_buf_reader = BufReader::new(&mut *client_tls);
    
    stream_request_to_upstream(
        &mut client_buf_reader,
        upstream,
        request_line,
        redactor,
        request_config,
    ).await.map(|_| ()).map_err(|e| {
        warn!("Failed to stream request to upstream: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })
}

/// Stream response to client with redaction
async fn stream_response_with_redaction<RW: AsyncReadExt + AsyncWriteExt + Unpin>(
    upstream: &mut (impl AsyncReadExt + AsyncWriteExt + Unpin),
    client_tls: &mut RW,
    response_line: &str,
    redactor: Arc<StreamingRedactor>,
) -> std::io::Result<()> {
    let response_config = StreamingResponseConfig::default();
    let mut upstream_buf_reader = BufReader::new(&mut *upstream);
    
    stream_response_to_client(
        &mut upstream_buf_reader,
        client_tls,
        response_line,
        redactor,
        response_config,
        None,
        None,
        Some("https"),
    ).await.map(|_| ()).map_err(|e| {
        error!("Failed to stream response to client with redaction: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })
}

async fn handle_single_request<RW>(
    client_tls: &mut RW,
    target_host: &str,
    upstream_addr: &str,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
    redact_responses: bool,
) -> std::io::Result<bool>
where
    RW: AsyncReadExt + AsyncWriteExt + Unpin,
{
    use scred_redactor::StreamingRedactor;
    use scred_http::streaming_response::{stream_response_to_client, StreamingResponseConfig};
    
    // Step 1: Read request line from client
    let mut request_line = read_request_line(client_tls).await?;
    if request_line.is_empty() {
        debug!("Empty request line received, skipping");
        return Ok(false);
    }
    
    // HTTP/2 Downgrade: Skip H2 preface and continue with HTTP/1.1
    if let Some(downgraded_line) = handle_h2_downgrade(client_tls, &request_line).await? {
        if downgraded_line.is_empty() {
            return Ok(true);
        }
        request_line = downgraded_line;
        warn!("HTTP/2 downgrade successful; continuing with HTTP/1.1");
    }
    
    debug!("[streaming] Request line: {}", request_line);
    
    // Step 2: Connect to upstream server
    let mut upstream = match connect_upstream_tls(upstream_addr, target_host).await {
        Ok(stream) => stream,
        Err(e) => {
            let error_response = "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n";
            let _ = client_tls.write_all(error_response.as_bytes()).await;
            return Err(e);
        }
    };
    
    // Extract and log upstream protocol negotiation
    let upstream_alpn = upstream.get_ref().1.alpn_protocol();
    let (upstream_protocol, _upstream_info) = handle_upstream_protocol_selection(
        upstream_alpn,
        target_host,
    ).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Check upstream protocol - if HTTP/2, we need different handling
    if matches!(upstream_protocol, HttpProtocol::Http2) {
        info!("[HTTP/2 Upstream] HTTP/2 upstream detected - forwarding via h2::client");
        return handle_h2_upstream_request(
            client_tls,
            &request_line,
            target_host,
            upstream_addr,
            redaction_engine,
            redact_responses,
        ).await;
    }


    // Step 3: Create redactor for streaming
    let redactor = Arc::new(StreamingRedactor::with_defaults(redaction_engine));
    
    // Step 4: Stream request to upstream with redaction
    info!("[Request] About to stream request line: {}", request_line);
    stream_request_with_redaction(client_tls, &mut upstream, &request_line, redactor.clone()).await?;
    
    info!("[streaming] About to read response line from upstream");
    let response_line = read_response_line(&mut upstream).await?;
    if response_line.is_empty() {
        debug!("Empty response line received, closing connection");
        return Ok(true);
    }
    
    debug!("[streaming] Response line: {}", response_line);
    
    let mut upstream_buf_reader = BufReader::new(&mut upstream);
    
    if redact_responses {
        info!("[streaming] Streaming response WITH redaction enabled");
        stream_response_with_redaction(&mut upstream, client_tls, &response_line, redactor.clone()).await?;
    }
    else {
        // Stream response without redaction
        info!("Response redaction DISABLED - forwarding as-is");
        forward_response_no_redaction(&mut upstream, client_tls, &response_line).await?;
    }
    
    Ok(true)
}

/// Helper to show which mode is in use
fn log_redaction_mode() {
    debug!("[Phase 6] Using STREAMING mode - full streaming architecture active");
}

/// Helper to handle upstream protocol detection and logging
///
/// This function encapsulates the logic for:
/// 1. Extracting protocol from upstream ALPN
/// 2. Creating UpstreamConnectionInfo
/// 3. Logging protocol selection
/// 4. Returning handler selector
fn handle_upstream_protocol_selection(
    upstream_alpn: Option<&[u8]>,
    target_host: &str,
) -> Result<(HttpProtocol, UpstreamConnectionInfo)> {
    let protocol = extract_upstream_protocol(upstream_alpn)?;
    
    let connection_info = UpstreamConnectionInfo {
        protocol: protocol.clone(),
        server_addr: target_host.to_string(),
    };

    match protocol {
        HttpProtocol::Http2 => {
            info!(
                "Upstream server {} negotiated HTTP/2, will transcode to HTTP/1.1 for downstream \
                 (transparent downgrade vs native H2 multiplexing)",
                target_host
            );
        }
        HttpProtocol::Http11 => {
            debug!("Upstream server {} negotiated HTTP/1.1, using existing streaming path", target_host);
        }
    }

    Ok((protocol, connection_info))
}

/// Handle HTTP/2 multiplexed connection
/// 
/// This is called when client negotiates HTTP/2 via ALPN.
/// Implements full HTTP/2 multiplexing with per-stream redaction and upstream forwarding.
async fn handle_h2_client_transcoding<S>(
    client_conn: S,
    host: &str,
    upstream_addr: &str,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
    _h2_redact_headers: bool,
    redaction_mode: crate::mitm::config::RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
    redact_patterns: &scred_http::PatternSelector,
    policy: Option<Arc<scred_policy::PolicyEngine>>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use crate::mitm::h2_mitm_handler::{H2MitmHandler, H2MitmConfig};
    
    let config = H2MitmConfig {
        redaction_mode,
        detect_patterns: detect_patterns.clone(),
        redact_patterns: redact_patterns.clone(),
        ..Default::default()
    };
    
    let handler = H2MitmHandler::new(
        redaction_engine,
        upstream_addr.to_string(),
        config,
        policy,
    );
    
    handler.handle_connection(client_conn, host).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_handle_h2_downgrade_no_preface() {
        // Test that non-H2 preface returns None
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut client, mut server) = tokio::io::duplex(1024);
            
            // Write a normal HTTP request line
            server.write_all(b"GET / HTTP/1.1\r\n").await.unwrap();
            
            let result = handle_h2_downgrade(&mut client, "GET / HTTP/1.1").await.unwrap();
            assert!(result.is_none(), "Non-H2 preface should return None");
        });
    }

    #[test]
    #[ignore]
    fn test_handle_h2_downgrade_with_preface() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut client, mut server) = tokio::io::duplex(4096);
            
            // Write H2 preface + SETTINGS frame + actual request
            server.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n").await.unwrap();
            // SETTINGS frame: length=0, type=4, flags=0, stream=0
            server.write_all(&[0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00]).await.unwrap();
            // Actual HTTP/1.1 request
            server.write_all(b"GET / HTTP/1.1\r\n").await.unwrap();
            
            let result = handle_h2_downgrade(&mut client, "PRI * HTTP/2.0").await.unwrap();
            assert!(result.is_some(), "H2 preface should return Some");
            if let Some(line) = result {
                assert_eq!(line, "GET / HTTP/1.1");
            }
        });
    }

    #[test]
    #[ignore]
    fn test_handle_h2_downgrade_empty_after_preface() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut client, mut server) = tokio::io::duplex(4096);
            
            // Write H2 preface + SETTINGS frame + empty line
            server.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n").await.unwrap();
            server.write_all(&[0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00]).await.unwrap();
            server.write_all(b"\r\n").await.unwrap();
            
            let result = handle_h2_downgrade(&mut client, "PRI * HTTP/2.0").await.unwrap();
            assert!(result.is_some(), "Should return Some even for empty");
            if let Some(line) = result {
                assert!(line.is_empty(), "Should return empty string");
            }
        });
    }


    #[test]
    fn test_connect_upstream_tls_invalid_host() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            connect_upstream_tls("http://invalid", "").await
        });
        assert!(result.is_err(), "Empty host should fail");
    }

    #[test]
    fn test_log_redaction_mode() {
        log_redaction_mode();
    }

    #[tokio::test]
    async fn test_forward_response_no_redaction_headers_only() {
        use tokio::io::{AsyncWriteExt, AsyncReadExt, duplex};
        
        let (mut upstream_write, mut upstream_read) = duplex(65536);
        let (mut client_write, mut client_read) = duplex(65536);
        
        // Write HTTP response with headers only (no body)
        upstream_write.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n").await.unwrap();
        drop(upstream_write);
        
        let result = forward_response_no_redaction(
            &mut upstream_read,
            &mut client_write,
            "HTTP/1.1 200 OK",
        ).await;
        assert!(result.is_ok(), "forward_response_no_redaction failed: {:?}", result);
        
        // Read the output from client
        drop(client_write);
        let mut output = Vec::new();
        client_read.read_to_end(&mut output).await.unwrap();
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("HTTP/1.1 200 OK"), "Should contain status line");
        assert!(output_str.contains("Content-Type: text/plain"), "Should contain content-type");
    }
}
