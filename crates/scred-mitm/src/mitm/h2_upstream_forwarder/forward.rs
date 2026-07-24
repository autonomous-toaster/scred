async fn forward_via_http1_1(
    request: &Request<Bytes>,
    engine: &Arc<RedactionEngine>,
    _upstream_addr: &str,
    mode: RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
    _redact_patterns: &scred_http::PatternSelector,
) -> Result<Vec<u8>> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let body = request.body();
    tracing::info!("[H2 Upstream] Forwarding via HTTP/1.1");

    // Extract host from URI
    let host = uri.host().unwrap_or("localhost");
    let port = uri.port_u16().unwrap_or(443);

    // Use unified connection logic with proxy support
    let proxy_url = get_proxy_url(host, true);
    let conn_config = UpstreamConnectionConfig::https(host, port);
    let conn_config = if let Some(ref proxy) = proxy_url {
        tracing::info!("[H2 Upstream] Using proxy: {}", proxy);
        conn_config.with_proxy(proxy)
    } else {
        conn_config
    };

    let tcp_stream = connect_tcp(&conn_config).await?;
    let mut tls_stream = establish_tls(tcp_stream, host).await?;
    tracing::debug!("[H2 Upstream HTTP/1.1] Connected and TLS established");

    let body_len = body.len();
    let content_length = if body_len > 0 {
        format!("Content-Length: {}\r\n", body_len)
    } else {
        String::new()
    };

    // Start with request line and Host header
    let mut http1_request = format!("{} {} HTTP/1.1\r\nHost: {}\r\n", method, uri, host);

    // Add all client headers (except hop-by-hop headers and pseudo-headers)
    for (name, value) in request.headers() {
        let name_str = name.as_str().to_lowercase();

        // Skip hop-by-hop headers, pseudo-headers, and headers we set explicitly
        if matches!(
            name_str.as_str(),
            "connection"
                | "transfer-encoding"
                | "upgrade"
                | "te"
                | "trailer"
                | "proxy-authenticate"
                | "proxy-authorization"
                | "host"
                | ":authority"
                | ":method"
                | ":path"
                | ":scheme"
                | "content-length" // We set this explicitly below
        ) {
            tracing::debug!("[H2 Upstream HTTP/1.1] Skipping header: {}", name);
            continue;
        }

        // Add header to request
        if let Ok(value_str) = value.to_str() {
            http1_request.push_str(&format!("{}: {}\r\n", name, value_str));
            tracing::debug!("[H2 Upstream HTTP/1.1] Added header: {}", name);
        }
    }

    // Add Content-Length and Connection headers
    http1_request.push_str(&content_length);
    http1_request.push_str("Connection: close\r\n\r\n");

    // Send HTTP/1.1 request headers
    tls_stream.write_all(http1_request.as_bytes()).await?;

    // Send body if present
    if body_len > 0 {
        tls_stream.write_all(body).await?;
        tracing::debug!(
            "[H2 Upstream HTTP/1.1] Request body sent: {} bytes",
            body_len
        );
    }

    tls_stream.flush().await?;

    tracing::debug!("[H2 Upstream HTTP/1.1] Request sent");

    // LAYER 1: For PASSTHROUGH and DETECT modes, read directly without streaming redaction
    // This avoids the buffering issue where small responses (≤512 bytes) get buffered and lost
    if !mode.should_redact() {
        tracing::debug!("[H2 Upstream HTTP/1.1] Mode: {:?} - Reading response directly (no streaming redaction)", mode);

        let response_bytes = read_response_direct(&mut tls_stream).await?;

        // Extract body from HTTP response
        let body = extract_http_response_body(&response_bytes)?;

        // If DETECT mode: log detected secrets
        if mode.should_detect() {
            tracing::info!("[H2 Upstream HTTP/1.1] DETECT mode - scanning for secrets");
            log_detected_secrets(engine, &response_bytes, detect_patterns);
        }

        return Ok(body);
    }

    // LAYER 2: REDACT mode - Use streaming redaction pipeline
    tracing::debug!("[H2 Upstream HTTP/1.1] Mode: REDACT - Using streaming redaction");
    let streaming_redactor = StreamingRedactor::with_defaults(engine.clone());
    let config = StreamingConfig::default();
    let mut response_output = Vec::new();
    let mut lookahead = Vec::with_capacity(config.lookahead_size);
    let mut read_buf = vec![0u8; config.chunk_size];
    let mut bytes_read = 0u64;
    let mut body_started = false;

    loop {
        match tls_stream.read(&mut read_buf).await {
            Ok(0) => {
                // EOF: process final chunk
                tracing::debug!("[H2 Upstream HTTP/1.1] EOF reached");

                // Final redaction pass if we have lookahead data
                if !lookahead.is_empty() {
                    let (redacted, _, _) =
                        streaming_redactor.process_chunk(&lookahead, &mut vec![], true);
                    response_output.extend_from_slice(redacted.as_bytes());
                }
                break;
            }
            Ok(n) => {
                bytes_read += n as u64;

                // Process chunk through streaming redactor
                let (redacted, _patterns, _) =
                    streaming_redactor.process_chunk(&read_buf[..n], &mut lookahead, false);

                // Skip HTTP headers, only output body
                if !body_started {
                    // Look for end of headers (double CRLF or double LF)
                    if let Some(header_end) = redacted.find("\r\n\r\n") {
                        body_started = true;
                        let body_part = &redacted[header_end + 4..];
                        response_output.extend_from_slice(body_part.as_bytes());
                        tracing::debug!(
                            "[H2 Upstream HTTP/1.1] Headers skipped, body streaming started"
                        );
                    } else if let Some(header_end) = redacted.find("\n\n") {
                        body_started = true;
                        let body_part = &redacted[header_end + 2..];
                        response_output.extend_from_slice(body_part.as_bytes());
                        tracing::debug!(
                            "[H2 Upstream HTTP/1.1] Headers skipped, body streaming started"
                        );
                    }
                } else {
                    response_output.extend_from_slice(redacted.as_bytes());
                }

                tracing::debug!(
                    "[H2 Upstream HTTP/1.1] Processed {} bytes, output: {} bytes",
                    n,
                    response_output.len()
                );
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                tracing::debug!("[H2 Upstream HTTP/1.1] Connection reset by peer - normal closure");
                break;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                tracing::debug!("[H2 Upstream HTTP/1.1] Unexpected EOF - server closed connection");
                break;
            }
            Err(e) => {
                // Check the error message for common connection closure patterns
                let err_msg = e.to_string();
                if err_msg.contains("EOF")
                    || err_msg.contains("Connection reset")
                    || err_msg.contains("connection closed")
                {
                    tracing::debug!("[H2 Upstream HTTP/1.1] Connection closed by peer: {}", e);
                    break; // ← Return what we got, don't error
                } else {
                    tracing::warn!("[H2 Upstream HTTP/1.1] Real read error: {}", e);
                    return Err(anyhow!("Read error: {}", e));
                }
            }
        }
    }

    tracing::info!(
        "[H2 Upstream HTTP/1.1] Response received: {} bytes read, {} bytes output",
        bytes_read,
        response_output.len()
    );

    Ok(response_output)
}

/// Helper for HTTP/1.1 with request parts and body (used in main handler)
async fn forward_via_http1_1_with_body(
    request_parts: &http::request::Parts,
    request_body: &Bytes,
    engine: &Arc<RedactionEngine>,
    _upstream_addr: &str,
    mode: RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
    _redact_patterns: &scred_http::PatternSelector,
) -> Result<Vec<u8>> {
    let method = request_parts.method.clone();
    let uri = request_parts.uri.clone();

    tracing::info!("[H2 Upstream] Forwarding via HTTP/1.1");

    // Extract host from URI for connection
    let host = uri.host().unwrap_or("localhost");
    let port = uri.port_u16().unwrap_or(443);

    // Build connection config - check for proxy
    let proxy_url = get_proxy_url(host, true);
    let conn_config = UpstreamConnectionConfig::https(host, port);
    let conn_config = if let Some(ref proxy) = proxy_url {
        tracing::info!("[H2 Upstream] Using proxy: {}", proxy);
        conn_config.with_proxy(proxy)
    } else {
        conn_config
    };

    // Connect through proxy or directly
    let tcp_stream = connect_tcp(&conn_config).await?;

    // Establish TLS
    let mut tls_stream = establish_tls(tcp_stream, host).await?;
    tracing::debug!("[H2 Upstream HTTP/1.1] Connected and TLS established");

    let body_len = request_body.len();
    let content_length = if body_len > 0 {
        format!("Content-Length: {}\r\n", body_len)
    } else {
        String::new()
    };

    // Start with request line and Host header
    let mut http1_request = format!("{} {} HTTP/1.1\r\nHost: {}\r\n", method, uri, host);

    // Add all client headers (except hop-by-hop headers and pseudo-headers)
    for (name, value) in &request_parts.headers {
        let name_str = name.as_str().to_lowercase();

        // Skip hop-by-hop headers, pseudo-headers, and headers we set explicitly
        if matches!(
            name_str.as_str(),
            "connection"
                | "transfer-encoding"
                | "upgrade"
                | "te"
                | "trailer"
                | "proxy-authenticate"
                | "proxy-authorization"
                | "host"
                | ":authority"
                | ":method"
                | ":path"
                | ":scheme"
                | "content-length" // We set this explicitly below
        ) {
            tracing::debug!("[H2 Upstream HTTP/1.1] Skipping header: {}", name);
            continue;
        }

        // Add header to request
        if let Ok(value_str) = value.to_str() {
            http1_request.push_str(&format!("{}: {}\r\n", name, value_str));
            tracing::debug!("[H2 Upstream HTTP/1.1] Added header: {}", name);
        }
    }

    // Add Content-Length and Connection headers
    http1_request.push_str(&content_length);
    http1_request.push_str("Connection: close\r\n\r\n");

    // Send HTTP/1.1 request headers
    tls_stream.write_all(http1_request.as_bytes()).await?;

    // Send body if present
    if body_len > 0 {
        tls_stream.write_all(request_body).await?;
        tracing::debug!(
            "[H2 Upstream HTTP/1.1] Request body sent: {} bytes",
            body_len
        );
    }

    tls_stream.flush().await?;

    tracing::debug!("[H2 Upstream HTTP/1.1] Request sent");

    // LAYER 1: For PASSTHROUGH and DETECT modes, read directly without streaming redaction
    // This avoids the buffering issue where small responses (≤512 bytes) get buffered and lost
    if !mode.should_redact() {
        tracing::debug!("[H2 Upstream HTTP/1.1] Mode: {:?} - Reading response directly (no streaming redaction)", mode);

        let response_bytes = read_response_direct(&mut tls_stream).await?;

        // Extract body from HTTP response
        let body = extract_http_response_body(&response_bytes)?;

        // If DETECT mode: log detected secrets
        if mode.should_detect() {
            tracing::info!("[H2 Upstream HTTP/1.1] DETECT mode - scanning for secrets");
            log_detected_secrets(engine, &response_bytes, detect_patterns);
        }

        return Ok(body);
    }

    // LAYER 2: REDACT mode - Use streaming redaction pipeline
    tracing::debug!("[H2 Upstream HTTP/1.1] Mode: REDACT - Using streaming redaction");
    let streaming_redactor = StreamingRedactor::with_defaults(engine.clone());
    let config = StreamingConfig::default();
    let mut response_output = Vec::new();
    let mut lookahead = Vec::with_capacity(config.lookahead_size);
    let mut read_buf = vec![0u8; config.chunk_size];
    let mut bytes_read = 0u64;
    let mut body_started = false;

    loop {
        match tls_stream.read(&mut read_buf).await {
            Ok(0) => {
                // EOF: process final chunk
                tracing::debug!("[H2 Upstream HTTP/1.1] EOF reached");

                // Final redaction pass if we have lookahead data
                if !lookahead.is_empty() {
                    let (redacted, _, _) =
                        streaming_redactor.process_chunk(&lookahead, &mut vec![], true);
                    response_output.extend_from_slice(redacted.as_bytes());
                }
                break;
            }
            Ok(n) => {
                bytes_read += n as u64;

                // Process chunk through streaming redactor
                let (redacted, _patterns, _) =
                    streaming_redactor.process_chunk(&read_buf[..n], &mut lookahead, false);

                // Skip HTTP headers, only output body
                if !body_started {
                    // Look for end of headers (double CRLF or double LF)
                    if let Some(header_end) = redacted.find("\r\n\r\n") {
                        body_started = true;
                        let body_part = &redacted[header_end + 4..];
                        response_output.extend_from_slice(body_part.as_bytes());
                        tracing::debug!(
                            "[H2 Upstream HTTP/1.1] Headers skipped, body streaming started"
                        );
                    } else if let Some(header_end) = redacted.find("\n\n") {
                        body_started = true;
                        let body_part = &redacted[header_end + 2..];
                        response_output.extend_from_slice(body_part.as_bytes());
                        tracing::debug!(
                            "[H2 Upstream HTTP/1.1] Headers skipped, body streaming started"
                        );
                    }
                } else {
                    response_output.extend_from_slice(redacted.as_bytes());
                }

                tracing::debug!(
                    "[H2 Upstream HTTP/1.1] Processed {} bytes, output: {} bytes",
                    n,
                    response_output.len()
                );
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                tracing::debug!("[H2 Upstream HTTP/1.1] Connection reset by peer - normal closure");
                break;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                tracing::debug!("[H2 Upstream HTTP/1.1] Unexpected EOF - server closed connection");
                break;
            }
            Err(e) => {
                // Check the error message for common connection closure patterns
                let err_msg = e.to_string();
                if err_msg.contains("EOF")
                    || err_msg.contains("Connection reset")
                    || err_msg.contains("connection closed")
                {
                    tracing::debug!("[H2 Upstream HTTP/1.1] Connection closed by peer: {}", e);
                    break; // ← Return what we got, don't error
                } else {
                    tracing::warn!("[H2 Upstream HTTP/1.1] Real read error: {}", e);
                    return Err(anyhow!("Read error: {}", e));
                }
            }
        }
    }

    tracing::info!(
        "[H2 Upstream HTTP/1.1] Response received: {} bytes read, {} bytes output",
        bytes_read,
        response_output.len()
    );

    Ok(response_output)
}

/// Establish TLS connection to upstream server
async fn establish_tls_upstream(
    tcp_stream: TcpStream,
    host: &str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let root_store = crate::build_root_cert_store();

    let client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let server_name =
        ServerName::try_from(host).map_err(|_| anyhow!("Invalid upstream host: {}", host))?;

    connector
        .connect(server_name, tcp_stream)
        .await
        .map_err(|e| anyhow!("TLS handshake failed: {}", e))
}
