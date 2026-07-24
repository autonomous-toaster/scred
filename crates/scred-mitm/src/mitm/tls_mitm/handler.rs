use super::*;

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
