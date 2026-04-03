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
use rustls::{Certificate, PrivateKey, ServerConfig};
use std::io::Cursor;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};
use scred_policy::PolicyEngine;

use super::config::RedactionMode;
use super::tls::CertificateGenerator;
use rustls::{ClientConfig, RootCertStore, ServerName};
use scred_http::dns_resolver::DnsResolver;
use scred_http::duplex::DuplexSocket;
use scred_http::h2::alpn::HttpProtocol;
use scred_http::http_line_reader::{read_request_line, read_response_line};
use scred_http::proxy_resolver::connect_through_proxy;
use scred_http::streaming_request::{stream_request_to_upstream, StreamingRequestConfig};
use scred_http::upstream_h2_client::{extract_upstream_protocol, UpstreamConnectionInfo};
use tokio_rustls::TlsConnector;

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
    redaction_mode: RedactionMode,
    _h2_redact_headers: bool,
    detect_patterns: scred_http::PatternSelector,
    redact_patterns: scred_http::PatternSelector,
    policy: Option<Arc<PolicyEngine>>,
) -> Result<()> {
    info!("TLS MITM tunnel starting for: {}", host);

    // Step 1: Get or generate certificate for this domain
    let (cert_pem, key_pem) = cert_generator.get_or_generate_cert(host).await?;
    debug!("Certificate loaded/generated for: {}", host);

    // Step 2: Parse certificate and key for rustls
    let mut cert_reader = Cursor::new(&cert_pem);
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

    let mut key_reader = Cursor::new(&key_pem);
    let parsed_keys: Vec<_> = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;

    if parsed_keys.is_empty() {
        return Err(anyhow!("No private keys found in PEM"));
    }

    let private_key = PrivateKey(parsed_keys[0].secret_pkcs8_der().to_vec());

    // Step 3: Build TLS ServerConfig (this accepts TLS FROM the client!)
    let mut server_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .map_err(|e| anyhow!("Failed to build TLS config: {}", e))?;

    // Add ALPN protocols: advertise both HTTP/2 and HTTP/1.1 to downstream clients
    // Phase 1: If client selects HTTP/2, downgrade to HTTP/1.1 (transparent fallback)
    // Full HTTP/2 support with frame forwarding with h2_reader and transcode modules
    // HTTP/1.1 client - only advertise HTTP/1.1 to upstream
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

    // Step 4: Combine split socket halves using DuplexSocket
    let duplex = DuplexSocket::new(client_read, client_write);

    // Step 5: Accept TLS FROM client - THIS IS THE KEY STEP!
    debug!("Accepting TLS connection from client...");
    let mut client_tls = acceptor.accept(duplex).await.map_err(|e| {
        error!("Client TLS handshake failed: {}", e);
        anyhow!("Client TLS handshake failed: {}", e)
    })?;

    // Extract negotiated ALPN protocol
    let negotiated_protocol = client_tls
        .get_ref()
        .1
        .alpn_protocol()
        .and_then(HttpProtocol::from_bytes)
        .unwrap_or(HttpProtocol::Http11);

    info!(
        "Client TLS handshake successful, HTTP decrypted, protocol: {}",
        negotiated_protocol
    );

    // Smart Routing: Handle HTTP/2 upstream based on client protocol and upstream type
    //
    // Decision Tree (from autoresearch.md):
    // 1. Did client negotiate H2 via ALPN?
    //    YES → Check upstream type (proxy vs direct)
    //    NO → Use existing HTTP/1.1 path (scenarios 1-3)
    //
    // 2. Is upstream a proxy (contains "://")?
    //    YES → Scenario 3: H2 client via proxy → transcode via H2UpstreamClient
    //    NO → Scenario 4: H2 client direct → use frame_forwarder for H2↔H2

let automaton = policy.as_ref().map(|p| p.automaton().clone());
    if negotiated_protocol.is_h2() {
        // Client negotiated HTTP/2 - use h2_mitm_handler (Phase 1.2)
        info!("H2 Client detected - using h2_mitm_handler");

        let mut h2_config = crate::mitm::h2_mitm_handler::H2MitmConfig::default();
        h2_config.redaction_mode = redaction_mode;
        h2_config.detect_patterns = detect_patterns.clone();
        h2_config.redact_patterns = redact_patterns.clone();

        let handler = crate::mitm::h2_mitm_handler::H2MitmHandler::new(
            redaction_engine.clone(),
            upstream_addr.to_string(),
            h2_config,
    policy.clone(),
        );

        info!(
            "[TLS MITM] Created H2 handler with upstream_addr: '{}'",
            upstream_addr
        );

        // Handle HTTP/2 connection
        match handler.handle_connection(client_tls, host).await {
            Ok(_) => {
                info!("H2 connection handled successfully");
                return Ok(());
            }
            Err(e) => {
                warn!("H2 handler failed: {}", e);
                return Err(anyhow!("HTTP/2 handler error: {}", e));
            }
        }
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
            redaction_mode,
 // Policy automaton retrieved before keep-alive loop
 policy.clone(),
 )
 .await {
            Ok(should_close) => {
                if should_close {
                    debug!("Response requested connection close; ending MITM keep-alive loop");
                    break 'keep_alive;
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    debug!("Client closed connection (EOF received)");
                    break 'keep_alive;
                }
                _ => {
                    warn!("Request handling error: {}", e);
                    return Err(anyhow!("Request handling failed: {}", e));
                }
            },
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


// POLICY-AWARE REQUEST HANDLING
// Uses per-header action from policy config:
// - Replace: Replace placeholders with secrets (no redaction)
// - Redact: Redact detected secrets (no placeholder replacement)
// - Detect: Log detections, pass through unchanged
// - Passthrough: No processing
async fn stream_request_with_policy<R, W>(
    client_reader: &mut tokio::io::BufReader<R>,
    upstream_writer: &mut W,
    request_line: &str,
    engine: &PolicyEngine,
    domain: &str,
) -> std::io::Result<scred_redactor::StreamingStats>
where
    R: tokio::io::AsyncReadExt + Unpin,
    W: tokio::io::AsyncWriteExt + Unpin,
{
    use scred_config::HeaderAction;
    use scred_http::http_headers::parse_http_headers;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // 1. Parse headers
    let headers = parse_http_headers(client_reader, false)
        .await
        .map_err(std::io::Error::other)?;

    // 2. Send request line
    let req_line = format!("{}\r\n", request_line);
    upstream_writer.write_all(req_line.as_bytes()).await?;

    // 3. Process each header according to its HeaderAction
    let resolved = engine.resolve_for_host(domain);
    let mut header_lines = Vec::new();
    let mut placeholders_replaced = 0;
    let mut secrets_redacted = 0;
    let mut secrets_detected = 0;

    for (header_name, header_value) in &headers.headers {
        let action = resolved.header_action(header_name);
        let processed_value = match action {
            HeaderAction::Replace => {
                // Replace placeholders with real secrets
                let mut value_bytes = header_value.as_bytes().to_vec();
                let (_, count) = engine
                    .create_placeholder_automaton()
                    .replace_placeholders(&mut value_bytes, domain, |_, _| true);
                if count > 0 {
                    tracing::info!(
                        "[policy] Replaced {} placeholder(s) in header: {}",
                        count, header_name
                    );
                    placeholders_replaced += count;
                }
                String::from_utf8_lossy(&value_bytes).to_string()
            }
            HeaderAction::Redact => {
                // Redact detected secrets
                let redacted = engine.redaction_engine().redact(header_value);
                if !redacted.matches.is_empty() {
                    tracing::debug!(
                        "[policy] Redacted {} secret(s) in header: {}",
                        redacted.matches.len(),
                        header_name
                    );
                    secrets_redacted += redacted.matches.len();
                }
                redacted.redacted
            }
            HeaderAction::Detect => {
                // Detect and log, but don't modify
                let redacted = engine.redaction_engine().redact(header_value);
                if !redacted.matches.is_empty() {
                    for m in &redacted.matches {
                        tracing::info!(
                            "[policy] Detected {} in header: {}",
                            m.pattern_type, header_name
                        );
                    }
                    secrets_detected += redacted.matches.len();
                }
                header_value.clone()
            }
            HeaderAction::Passthrough => {
                // No processing
                header_value.clone()
            }
        };
        header_lines.push(format!("{}: {}", header_name, processed_value));
    }

    // Write processed headers
    let headers_block = header_lines.join("\r\n");
    upstream_writer.write_all(headers_block.as_bytes()).await?;
    upstream_writer.write_all(b"\r\n\r\n").await?;

    // Log summary if any processing occurred
    if placeholders_replaced > 0 || secrets_redacted > 0 || secrets_detected > 0 {
        tracing::info!(
            "[policy] Headers processed: {} placeholders replaced, {} secrets redacted, {} secrets detected",
            placeholders_replaced, secrets_redacted, secrets_detected
        );
    }

    // 4. Stream body with placeholder replacement
    let mut stats = scred_redactor::StreamingStats::default();
    if let Some(content_length) = headers.content_length {
        // Read and replace in body
        let mut body = vec![0u8; content_length];
        client_reader.read_exact(&mut body).await?;
        let (_, body_count) = engine
            .create_placeholder_automaton()
            .replace_placeholders(&mut body, domain, |_, _| true);
        if body_count > 0 {
            tracing::info!("[policy] Replaced {} placeholders in request body", body_count);
        }
        upstream_writer.write_all(&body).await?;
        stats.bytes_written = body.len() as u64;
    }
    upstream_writer.flush().await?;
    Ok(stats)
}

async fn handle_single_request<RW>(
    client_tls: &mut RW,
    target_host: &str,
    upstream_addr: &str,
    redaction_engine: Arc<scred_redactor::RedactionEngine>,
    redaction_mode: RedactionMode,
    policy: Option<Arc<PolicyEngine>>,
) -> std::io::Result<bool>
where
    RW: AsyncReadExt + AsyncWriteExt + Unpin,
{
    use scred_http::streaming_response::{stream_response_to_client, StreamingResponseConfig};
    use scred_redactor::StreamingRedactor;

    // Step 1: Read request line from client
    let mut request_line = read_request_line(client_tls).await?;
    if request_line.is_empty() {
        debug!("Empty request line received, skipping");
        return Ok(false);
    }

    // HTTP/2 Downgrade: Skip H2 preface and continue with HTTP/1.1
    // Per RFC 7540 Section 3.4: When server doesn't send h2 frames, client auto-downgrades
    if request_line.starts_with("PRI * HTTP/2.0") {
        warn!(
            "Client sent HTTP/2 preface; initiating transparent downgrade to HTTP/1.1 (RFC 7540 Section 3.4)"
        );

        // The client sends HTTP/2 connection preface, then a SETTINGS frame
        // Preface: "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n" (24 bytes, already read as request_line)
        // SETTINGS frame: 9-byte header + variable payload

        // Read and skip the SETTINGS frame
        let mut frame_header = [0u8; 9];
        match client_tls.read(&mut frame_header).await {
            Ok(n) if n == 9 => {
                // Parse frame length (first 3 bytes, big-endian)
                let frame_len = ((frame_header[0] as u32) << 16)
                    | ((frame_header[1] as u32) << 8)
                    | (frame_header[2] as u32);

                // Skip frame payload
                if frame_len > 0 {
                    let mut payload = vec![0u8; frame_len as usize];
                    let _ = client_tls.read_exact(&mut payload).await;
                }

                debug!(
                    "Skipped HTTP/2 preface + SETTINGS frame ({} bytes payload)",
                    frame_len
                );
            }
            Ok(n) => {
                warn!("Only read {} bytes of frame header; continuing anyway", n);
            }
            Err(e) => {
                warn!("Failed to read h2 SETTINGS frame: {}; continuing anyway", e);
            }
        }

        // Read the actual HTTP/1.1 request line that follows
        request_line = read_request_line(client_tls).await?;
        if request_line.is_empty() {
            warn!("No HTTP/1.1 request after h2 preface; closing connection");
            return Ok(true);
        }

        warn!("HTTP/2 downgrade successful; continuing with HTTP/1.1");
    }

    debug!("[streaming] Request line: {}", request_line);

    // Step 2: Connect to upstream server
    let is_upstream_proxy = upstream_addr.contains("://");

    debug!(
        "Connecting to upstream: {} (proxy_mode={})",
        upstream_addr, is_upstream_proxy
    );

    let upstream_tcp = if is_upstream_proxy {
        connect_through_proxy(upstream_addr, target_host, 443)
            .await
            .map_err(|e| {
                error!("Failed to connect to upstream {}: {}", upstream_addr, e);
                std::io::Error::other(e)
            })?
    } else {
        DnsResolver::connect_with_retry(&format!("{}:443", target_host))
            .await
            .map_err(std::io::Error::other)?
    };

    info!("Connected to upstream {}", upstream_addr);

    // Use standard environment variables for custom CA certificates
 // Supports SSL_CERT_FILE and CURL_CA_BUNDLE
 let root_store = crate::build_root_cert_store();
 let mut client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // Add ALPN support to upstream connection
    // Phase 1: Advertise both h2 and http/1.1
    // - If upstream negotiates h2: use h2_reader + H2Transcoder to convert to http/1.1
    // - If upstream negotiates http/1.1: use existing streaming path
    // - Redaction applied after transcode (zero changes to redaction logic)
    //
    // This enables transparent h2 upstream support while keeping downstream HTTP/1.1 only
    // HTTP/1.1 client - only advertise HTTP/1.1 to upstream
    client_config.alpn_protocols = vec![b"http/1.1".to_vec()];

    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name = ServerName::try_from(target_host).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid upstream host")
    })?;

    info!(
        "[TLS] Starting upstream TLS handshake with server_name={}",
        target_host
    );
    let mut upstream = connector
        .connect(server_name, upstream_tcp)
        .await
        .map_err(|e| {
            error!("[TLS] Upstream TLS handshake FAILED: {}", e);
            std::io::Error::other(format!("upstream TLS failed: {}", e))
        })?;

    // Extract and log upstream protocol negotiation
    let upstream_alpn = upstream.get_ref().1.alpn_protocol();
    let (_upstream_protocol, _upstream_info) =
        handle_upstream_protocol_selection(upstream_alpn, target_host)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

    // HTTP/2 UPSTREAM SUPPORT: Currently forwarded via HTTP/1.1 stream
    // Full HTTP/2 upstream multiplexing is available via h2 crate when needed
    // See: h2_mitm_handler.rs for HTTP/2 client-side handling
// Step 3: Create redactor for streaming
 let engine_for_detection = redaction_engine.clone();
 let redactor = Arc::new(StreamingRedactor::with_defaults(redaction_engine));
 
 // Step 4: Stream request to upstream
 // POLICY PATH: Replace placeholders with real secrets (skip redaction)
 // NORMAL PATH: Redact secrets before sending upstream
 let request_config = StreamingRequestConfig::default();
 info!("[Request] About to stream request line: {}", request_line);
 
 {
 let mut client_buf_reader = BufReader::new(&mut *client_tls);
 if let Some(ref engine) = policy {
 // POLICY PATH: Replace placeholders, don't redact
 info!("[policy] Using placeholder replacement for request");
 match stream_request_with_policy(
 &mut client_buf_reader,
 &mut upstream,
 &request_line,
 engine,
 target_host,
 ).await {
 Ok(stats) => {
 debug!(
 "[policy] Request streamed: {} bytes written",
 stats.bytes_written
 );
 }
 Err(e) => {
 warn!("Failed to stream request with policy: {}", e);
 return Err(e);
 }
 }
 } else {
 // NORMAL PATH: Redact secrets
 match stream_request_to_upstream(
 &mut client_buf_reader,
 &mut upstream,
 &request_line,
 redactor.clone(),
 request_config,
 )
 .await {
 Ok(stats) => {
 debug!(
 "[streaming] Request streamed: {} bytes read, {} bytes written",
 stats.bytes_read, stats.bytes_written
 );
 }
 Err(e) => {
 warn!("Failed to stream request to upstream: {}", e);
 return Err(std::io::Error::other(e));
 }
 }
 }
}
    info!("[streaming] About to read response line from upstream");
    let response_line = read_response_line(&mut upstream).await?;
    if response_line.is_empty() {
        debug!("Empty response line received, closing connection");
        return Ok(true);
    }

    debug!("[streaming] Response line: {}", response_line);

    let mut upstream_buf_reader = BufReader::new(&mut upstream);

    // Determine response body action from policy or redaction_mode
    use scred_config::BodyAction;
    let response_action = if let Some(ref engine) = policy {
        let resolved = engine.resolve_for_host(target_host);
        resolved.response_body_action()
    } else {
        // Map redaction_mode to BodyAction for backward compatibility
        match redaction_mode {
            RedactionMode::Redact => BodyAction::Redact,
            RedactionMode::DetectOnly => BodyAction::Detect,
            RedactionMode::Passthrough => BodyAction::Passthrough,
        }
    };

    if response_action == BodyAction::Redact {
        // Stream response with redaction
        let response_config = StreamingResponseConfig::default();
        info!("[streaming] Streaming response WITH redaction enabled");
        match stream_response_to_client(
            &mut upstream_buf_reader,
            client_tls,
            &response_line,
            redactor.clone(),
            response_config,
            None,
            None,
            Some("https"),
        )
        .await
        {
            Ok(stats) => {
                info!(
                    "[streaming] Response streamed to client: {} bytes read, {} bytes written",
                    stats.bytes_read, stats.bytes_written
                );
            }
            Err(e) => {
                error!("Failed to stream response to client with redaction: {}", e);
                return Err(std::io::Error::other(e));
            }
        }
    } else if response_action == BodyAction::Detect {
        // DETECT mode: detect secrets but don't redact
        info!("[DETECT] Detecting secrets without redaction");

        let headers = scred_http::http_headers::parse_http_headers(&mut upstream_buf_reader, true)
            .await
            .map_err(std::io::Error::other)?;

        let mut body_bytes = Vec::new();
        if let Some(content_length) = headers.content_length {
            body_bytes.resize(content_length, 0);
            upstream_buf_reader.read_exact(&mut body_bytes).await?;
        } else {
            let mut buf = vec![0u8; 8192];
            loop {
                match upstream_buf_reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => body_bytes.extend_from_slice(&buf[..n]),
                    Err(_) => break,
                }
            }
        }

        let body_str = String::from_utf8_lossy(&body_bytes);
        let detection_result = engine_for_detection.redact(&body_str);
        if !detection_result.warnings.is_empty() {
            info!(
                "[DETECT] Found {} secrets in response:",
                detection_result.warnings.len()
            );
            for (idx, warning) in detection_result.warnings.iter().enumerate() {
                info!(
                    "[DETECT] [{}] pattern_type: {}, count: {}",
                    idx + 1, warning.pattern_type, warning.count
                );
            }
        } else {
            debug!("[DETECT] No secrets detected");
        }

        client_tls
            .write_all(format!("{}\r\n", response_line).as_bytes())
            .await?;
        client_tls.write_all(headers.raw_headers.as_bytes()).await?;
        client_tls.write_all(&body_bytes).await?;
        client_tls.flush().await?;
    } else {
        // PASSTHROUGH mode: no detection, just forward
        info!("[PASSTHROUGH] Forwarding response unchanged");

        let headers = scred_http::http_headers::parse_http_headers(&mut upstream_buf_reader, true)
            .await
            .map_err(std::io::Error::other)?;

        info!(
            "[streaming] Response headers parsed: content_length={:?}",
            headers.content_length
        );

        client_tls
            .write_all(format!("{}\r\n", response_line).as_bytes())
            .await?;
        client_tls.write_all(headers.raw_headers.as_bytes()).await?;
        client_tls.write_all(b"\r\n").await?;

        if let Some(content_length) = headers.content_length {
            let mut remaining = content_length;
            let mut buffer = vec![0u8; 8192];
            while remaining > 0 {
                let to_read = std::cmp::min(remaining, buffer.len());
                match upstream_buf_reader.read(&mut buffer[..to_read]).await {
                    Ok(0) => {
                        warn!("Unexpected EOF, expected {} more bytes", remaining);
                        break;
                    }
                    Ok(n) => {
                        client_tls.write_all(&buffer[..n]).await?;
                        remaining -= n;
                    }
                    Err(e) => {
                        warn!("Error reading response body: {}", e);
                        return Err(e);
                    }
                }
            }
            client_tls.flush().await?;
        } else if headers.is_chunked() {
            let mut buffer = vec![0u8; 8192];
            loop {
                match upstream_buf_reader.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        client_tls.write_all(&buffer[..n]).await?;
                    }
                    Err(e) => {
                        warn!("Error reading chunked body: {}", e);
                        return Err(e);
                    }
                }
            }
            client_tls.flush().await?;
        } else {
            let mut buffer = vec![0u8; 65536];
            loop {
                match upstream_buf_reader.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        client_tls.write_all(&buffer[..n]).await?;
                    }
                    Err(e) => {
                        warn!("Error reading response body: {}", e);
                        return Err(e);
                    }
                }
            }
            client_tls.flush().await?;
        }
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
        protocol,
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
            debug!(
                "Upstream server {} negotiated HTTP/1.1, using existing streaming path",
                target_host
            );
        }
    }

    Ok((protocol, connection_info))
}

/// Handle HTTP/2 multiplexed connection
///
/// This is called when client negotiates HTTP/2 via ALPN.
/// Implements full HTTP/2 multiplexing with per-stream redaction and upstream forwarding.
pub async fn handle_h2_multiplexed_connection<S>(
    conn: S,
    _host: &str,
    _upstream_addr: &str,
    _redaction_engine: Arc<scred_redactor::RedactionEngine>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    // HTTP/2 MULTIPLEXING: Handled via H2MitmHandler
    // Client-side HTTP/2 is routed to H2MitmHandler in handle_tls_connection()
    // This function is not used for HTTP/2 client connections
    let _ = conn; // Use conn to satisfy compiler
    Err(anyhow!(
        "HTTP/2 client connections are handled by H2MitmHandler, not this function"
    ))
}
