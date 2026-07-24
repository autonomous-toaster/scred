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
        let mut body = vec![0u8; content_length];
        client_reader.read_exact(&mut body).await?;

        // Step 4a: REDACT secrets in request body FIRST
        let body_str = String::from_utf8_lossy(&body);
        let redacted = engine.redaction_engine().redact(&body_str);
        if !redacted.matches.is_empty() {
            tracing::info!(
                "[policy] Redacted {} secret(s) in request body",
                redacted.matches.len()
            );
        }
        let mut processed_body = redacted.redacted.into_bytes();

        // Step 4b: REPLACE placeholders in request body (after redaction)
        let (_, placeholder_count) = engine
            .create_placeholder_automaton()
            .replace_placeholders(&mut processed_body, domain, |_, _| true);

        if placeholder_count > 0 {
            tracing::info!(
                "[policy] Replaced {} placeholder(s) in request body",
                placeholder_count
            );
        }

        upstream_writer.write_all(&processed_body).await?;
        stats.bytes_written = processed_body.len() as u64;
    }

    upstream_writer.flush().await?;
    Ok(stats)
}

async fn handle_single_request<RW>(
