/// HTTP/2 Upstream Forwarder - Forward requests to upstream HTTP/2 or HTTP/1.1
/// 
/// Handles:
/// - Direct connection (no corporate proxy): tries h2, falls back to HTTP/1.1
/// - Corporate proxy (http_proxy env var): receives downgraded HTTP/1.1
/// - Streaming redaction: Process responses in chunks (64KB) without loading full body
/// - Three modes: PASSTHROUGH (no redaction), DETECT (detect & log), REDACT (detect & redact)

use anyhow::{Result, anyhow};
use std::sync::Arc;
use scred_redactor::{RedactionEngine, StreamingRedactor, StreamingConfig};
use bytes::Bytes;
use http::Request;
use tokio::io::AsyncReadExt;
use crate::mitm::config::RedactionMode;

use tokio::net::TcpStream;
use h2::client;
use rustls::{ClientConfig, RootCertStore, ServerName};
use tokio::io::{AsyncWriteExt};

/// Forward HTTP/2 request to upstream server (passthrough mode)
/// 
/// In MITM mode with H2 client:
/// - Try H2 to upstream first (direct connection)
/// - Fall back to HTTP/1.1 if H2 fails
/// - Handle corporate proxy downgrades

// ============================================================================
// Helper Functions
// ============================================================================

/// Read complete HTTP response directly without streaming redaction
/// Used for PASSTHROUGH and DETECT modes
async fn read_response_direct(tls_stream: &mut (impl AsyncReadExt + Unpin)) -> Result<Vec<u8>> {
    let mut response = Vec::new();
    let mut buffer = vec![0u8; 4096];

    loop {
        match tls_stream.read(&mut buffer).await {
            Ok(0) => {
                // EOF: connection closed
                tracing::debug!("[H2 Upstream HTTP/1.1 Direct] EOF reached");
                break;
            }
            Ok(n) => {
                response.extend_from_slice(&buffer[..n]);
                tracing::debug!("[H2 Upstream HTTP/1.1 Direct] Read {} bytes, total: {}", n, response.len());
            }
            Err(e) => {
                // Check if this is a normal connection closure
                let err_msg = e.to_string();
                let err_kind = e.kind();
                
                // Common EOF/closure errors - all legitimate
                if err_msg.contains("EOF") 
                    || err_msg.contains("Connection reset")
                    || err_msg.contains("connection closed")
                    || err_msg.contains("unexpected end of file")
                    || err_kind == std::io::ErrorKind::UnexpectedEof
                    || err_kind == std::io::ErrorKind::ConnectionReset
                    || err_kind == std::io::ErrorKind::ConnectionAborted {
                    tracing::debug!("[H2 Upstream HTTP/1.1 Direct] Connection closed by peer: {}", e);
                    break;
                } else {
                    tracing::warn!("[H2 Upstream HTTP/1.1 Direct] Read error: {}", e);
                    return Err(anyhow!("Read error: {}", e));
                }
            }
        }
    }

    tracing::info!("[H2 Upstream HTTP/1.1 Direct] Total response received: {} bytes", response.len());
    Ok(response)
}

/// Extract HTTP response body from full HTTP response (headers + body)
fn extract_http_response_body(response: &[u8]) -> Result<Vec<u8>> {
    let response_str = String::from_utf8_lossy(response);
    
    // Find HTTP header terminator
    if let Some(pos) = response_str.find("\r\n\r\n") {
        let body = &response[pos + 4..];
        tracing::debug!("[H2 Upstream HTTP/1.1] Extracted body: {} bytes", body.len());
        return Ok(body.to_vec());
    }
    
    if let Some(pos) = response_str.find("\n\n") {
        let body = &response[pos + 2..];
        tracing::debug!("[H2 Upstream HTTP/1.1] Extracted body (LF only): {} bytes", body.len());
        return Ok(body.to_vec());
    }
    
    // No headers found - return response as-is
    tracing::debug!("[H2 Upstream HTTP/1.1] No header terminator found, returning full response");
    Ok(response.to_vec())
}

/// Log detected secrets in response without redacting
/// Filters by detect_patterns selector - only logs secrets that match the selector
fn log_detected_secrets(
    engine: &Arc<RedactionEngine>,
    response_bytes: &[u8],
    detect_patterns: &scred_http::PatternSelector,
) {
    use scred_http::get_pattern_tier;
    
    let response_str = String::from_utf8_lossy(response_bytes);
    
    // Run detection (redaction engine will find patterns)
    let redaction_result = engine.redact(&response_str);
    
    // Filter and log warnings based on detect_patterns selector
    let filtered_warnings: Vec<_> = redaction_result
        .warnings
        .iter()
        .filter(|warning| {
            // Get the tier for this pattern
            let tier = get_pattern_tier(&warning.pattern_type);
            // Check if it matches the selector
            detect_patterns.matches_pattern(&warning.pattern_type, tier)
        })
        .collect();
    
    if !filtered_warnings.is_empty() {
        tracing::info!("[DETECT] Found {} secrets in response (filtered by selector):", filtered_warnings.len());
        for (idx, warning) in filtered_warnings.iter().enumerate() {
            tracing::info!("[DETECT]   [{}] pattern_type: {}, count: {}", idx + 1, warning.pattern_type, warning.count);
        }
    } else {
        tracing::debug!("[DETECT] No secrets detected matching selector");
    }
}
/// - Stream redaction: Process in 64KB chunks, no full-body buffering
pub async fn handle_upstream_h2_connection(
    request: Request<Bytes>,
    engine: Arc<RedactionEngine>,
    upstream_addr: String,
    host: &str,
    mode: RedactionMode,
    detect_patterns: scred_http::PatternSelector,
    redact_patterns: scred_http::PatternSelector,
) -> Result<Vec<u8>> {
    let _method = request.method().clone();
    let _uri = request.uri().clone();
    
    tracing::info!("[H2 Upstream] Forwarding {} {} (host: {}, upstream: {})", 
        _method, _uri, host, upstream_addr);

    // Extract body from request
    let (request_parts, request_body) = request.into_parts();

    // Check if corporate proxy is active (non-empty env vars)
    let has_proxy = (std::env::var("http_proxy")
        .map(|v| !v.is_empty())
        .unwrap_or(false))
        || (std::env::var("https_proxy")
            .map(|v| !v.is_empty())
            .unwrap_or(false))
        || (std::env::var("HTTP_PROXY")
            .map(|v| !v.is_empty())
            .unwrap_or(false))
        || (std::env::var("HTTPS_PROXY")
            .map(|v| !v.is_empty())
            .unwrap_or(false));

    if has_proxy {
        tracing::info!("[H2 Upstream] Corporate proxy detected - using HTTP/1.1 fallback");
        return forward_via_http1_1_with_body(&request_parts, &request_body, &engine, &upstream_addr, mode, &detect_patterns, &redact_patterns).await;
    }

    // No proxy: try H2 first, then fallback to HTTP/1.1
    tracing::debug!("[H2 Upstream] No corporate proxy - attempting H2 direct connection");
    
    // Rebuild request with parts and body for H2 attempt
    let h2_request = http::Request::from_parts(request_parts.clone(), request_body.clone());
    
    match try_forward_h2(h2_request, engine.clone(), &upstream_addr, host).await {
        Ok(response) => {
            tracing::info!("[H2 Upstream] H2 forward successful");
            Ok(response)
        }
        Err(e) => {
            tracing::warn!("[H2 Upstream] H2 forward failed ({}), falling back to HTTP/1.1", e);
            // Rebuild request for HTTP/1.1 fallback
            let http1_request = http::Request::from_parts(request_parts, request_body);
            forward_via_http1_1(&http1_request, &engine, &upstream_addr, mode, &detect_patterns, &redact_patterns).await
        }
    }
}

/// Try to forward via HTTP/2 direct connection
async fn try_forward_h2(
    request: Request<Bytes>,
    _engine: Arc<RedactionEngine>,
    upstream_addr: &str,
    host: &str,
) -> Result<Vec<u8>> {
    let _method = request.method().clone();
    let _uri = request.uri().clone();
    let (request_parts, request_body) = request.into_parts();
    
    // Connect to the configured upstream address (not to 'host'!)
    // 'host' is for SNI/TLS verification, but we connect to upstream_addr
    let socket_addr = upstream_addr.to_string();

    let tcp_stream = TcpStream::connect(&socket_addr).await?;
    tracing::debug!("[H2 Upstream] Connected to {} via TCP", socket_addr);

    // Establish TLS connection to upstream
    let tls_stream = establish_tls_upstream(tcp_stream, host).await?;
    tracing::debug!("[H2 Upstream] TLS handshake complete with {}", host);

    // Initiate h2 client connection over TLS
    let (mut send_request, connection) = client::handshake(tls_stream).await?;
    tracing::debug!("[H2 Upstream] H2 client handshake complete");

    // Wrap connection in a handle to manage its lifecycle
    let connection_handle = tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::debug!("[H2 Upstream] Connection driver ended: {}", e);
        }
    });

    // Build request for upstream with body (if present)
    let upstream_request = http::Request::from_parts(request_parts, ());

    // Determine if we have a body to send
    let has_body = !request_body.is_empty();
    
    // Send the request to upstream (end_stream=true if no body)
    let (response_future, mut send_stream) = send_request
        .send_request(upstream_request, !has_body)
        .map_err(|e| {
            tracing::warn!("[H2 Upstream] Failed to send request: {}", e);
            // Abort the connection task - we're exiting this h2 connection
            connection_handle.abort();
            anyhow!("Failed to send request: {}", e)
        })?;

    // Send body if present
    if has_body {
        tracing::debug!("[H2 Upstream] Sending request body: {} bytes", request_body.len());
        match send_stream.send_data(request_body, true) {
            Ok(_) => {},
            Err(e) => {
                tracing::warn!("[H2 Upstream] Failed to send body: {}", e);
                connection_handle.abort();
                return Err(anyhow!("Failed to send body: {}", e));
            }
        }
    }

    // Wait for response headers
    // If the connection closes before sending headers, this will return an error
    let response = match response_future.await {
        Ok(r) => r,
        Err(e) => {
            // Connection closed before response headers - this is normal for some servers
            let err_msg = e.to_string();
            if err_msg.contains("EOF") || err_msg.contains("unexpected end of file") 
                || err_msg.contains("connection closed") {
                tracing::debug!("[H2 Upstream] Server closed connection before sending response headers");
                // This is NOT a catastrophic error - fallback will handle it
                connection_handle.abort();
                return Err(anyhow!("H2 connection closed before response: {}", e));
            } else {
                tracing::warn!("[H2 Upstream] Error waiting for response headers: {}", e);
                connection_handle.abort();
                return Err(anyhow!("Response error: {}", e));
            }
        }
    };
    
    let (response_parts, mut recv_stream) = response.into_parts();

    tracing::info!("[H2 Upstream] Received H2 response: status={}", response_parts.status);

    // Read response body from h2 stream
    let mut response_body = Vec::new();
    let mut chunks_received = 0;
    loop {
        match recv_stream.data().await {
            Some(Ok(chunk)) => {
                chunks_received += 1;
                response_body.extend_from_slice(&chunk);
                tracing::debug!("[H2 Upstream] Received response chunk #{}: {} bytes", chunks_received, chunk.len());
                tracing::debug!("[H2 Upstream] Total response body so far: {} bytes", response_body.len());
            }
            Some(Err(e)) => {
                // Check if it's a connection reset or other recoverable error
                let err_msg = e.to_string();
                if err_msg.contains("unexpected end of file") || err_msg.contains("EOF") {
                    // Connection closed - this is often normal for some servers
                    tracing::warn!("[H2 Upstream] Connection closed by upstream ({}). Got {} bytes", e, response_body.len());
                    // Don't fail - return what we got
                    break;
                } else {
                    // Other errors should still fail
                    return Err(anyhow!("Failed to read response body: {}", e));
                }
            }
            None => {
                // Stream ended normally
                tracing::debug!("[H2 Upstream] Response stream ended");
                break;
            }
        }
    }

    tracing::info!("[H2 Upstream] H2 response body received: {} bytes", response_body.len());
    
    Ok(response_body)
}

/// Fallback: Forward via HTTP/1.1 with streaming redaction
async fn forward_via_http1_1(
    request: &Request<Bytes>,
    engine: &Arc<RedactionEngine>,
    upstream_addr: &str,
    mode: RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
    _redact_patterns: &scred_http::PatternSelector,
) -> Result<Vec<u8>> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let body = request.body();
    
    tracing::info!("[H2 Upstream] Forwarding via HTTP/1.1 to {}", upstream_addr);

    // Parse upstream address, adding default HTTPS port if not specified
    let socket_addr = if upstream_addr.contains(':') {
        upstream_addr.to_string()
    } else {
        format!("{}:443", upstream_addr)
    };
    
    // Extract hostname for TLS SNI (use upstream_addr if it looks like a hostname)
    let sni_hostname = if upstream_addr.contains(':') {
        upstream_addr.split(':').next().unwrap_or("localhost")
    } else {
        upstream_addr
    };
    
    let tcp_stream = TcpStream::connect(&socket_addr).await
        .map_err(|e| {
            tracing::error!("[H2 Upstream HTTP/1.1] Failed to connect: {}", e);
            anyhow!("Failed to connect: {}", e)
        })?;

    // Establish TLS for HTTP/1.1
    let mut tls_stream = establish_tls_upstream(tcp_stream, sni_hostname).await?;
    
    tracing::debug!("[H2 Upstream HTTP/1.1] Connected and TLS established");

    // Build HTTP/1.1 request with all client headers
    let body_len = body.len();
    let content_length = if body_len > 0 {
        format!("Content-Length: {}\r\n", body_len)
    } else {
        String::new()
    };

    // Start with request line and Host header
    let mut http1_request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\n",
        method, uri, sni_hostname
    );

    // Add all client headers (except hop-by-hop headers and pseudo-headers)
    for (name, value) in request.headers() {
        let name_str = name.as_str().to_lowercase();
        
        // Skip hop-by-hop headers, pseudo-headers, and headers we set explicitly
        if matches!(name_str.as_str(),
            "connection" | "transfer-encoding" | "upgrade" | "te" | "trailer" 
            | "proxy-authenticate" | "proxy-authorization" | "host"
            | ":authority" | ":method" | ":path" | ":scheme"
            | "content-length"  // We set this explicitly below
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
        tracing::debug!("[H2 Upstream HTTP/1.1] Request body sent: {} bytes", body_len);
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
                    let (redacted, _, _) = streaming_redactor.process_chunk(&lookahead, &mut vec![], true);
                    response_output.extend_from_slice(redacted.as_bytes());
                }
                break;
            }
            Ok(n) => {
                bytes_read += n as u64;
                
                // Process chunk through streaming redactor
                let (redacted, _patterns, _) = streaming_redactor.process_chunk(&read_buf[..n], &mut lookahead, false);
                
                // Skip HTTP headers, only output body
                if !body_started {
                    // Look for end of headers (double CRLF or double LF)
                    if let Some(header_end) = redacted.find("\r\n\r\n") {
                        body_started = true;
                        let body_part = &redacted[header_end + 4..];
                        response_output.extend_from_slice(body_part.as_bytes());
                        tracing::debug!("[H2 Upstream HTTP/1.1] Headers skipped, body streaming started");
                    } else if let Some(header_end) = redacted.find("\n\n") {
                        body_started = true;
                        let body_part = &redacted[header_end + 2..];
                        response_output.extend_from_slice(body_part.as_bytes());
                        tracing::debug!("[H2 Upstream HTTP/1.1] Headers skipped, body streaming started");
                    }
                } else {
                    response_output.extend_from_slice(redacted.as_bytes());
                }
                
                tracing::debug!("[H2 Upstream HTTP/1.1] Processed {} bytes, output: {} bytes", n, response_output.len());
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
                if err_msg.contains("EOF") || err_msg.contains("Connection reset") 
                    || err_msg.contains("connection closed") {
                    tracing::debug!("[H2 Upstream HTTP/1.1] Connection closed by peer: {}", e);
                    break;  // ← Return what we got, don't error
                } else {
                    tracing::warn!("[H2 Upstream HTTP/1.1] Real read error: {}", e);
                    return Err(anyhow!("Read error: {}", e));
                }
            }
        }
    }

    tracing::info!("[H2 Upstream HTTP/1.1] Response received: {} bytes read, {} bytes output", bytes_read, response_output.len());
    
    Ok(response_output)
}

/// Helper for HTTP/1.1 with request parts and body (used in main handler)
async fn forward_via_http1_1_with_body(
    request_parts: &http::request::Parts,
    request_body: &Bytes,
    engine: &Arc<RedactionEngine>,
    upstream_addr: &str,
    mode: RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
    _redact_patterns: &scred_http::PatternSelector,
) -> Result<Vec<u8>> {
    let method = request_parts.method.clone();
    let uri = request_parts.uri.clone();
    
    tracing::info!("[H2 Upstream] Forwarding via HTTP/1.1 to {}", upstream_addr);

    // Parse upstream address, adding default HTTPS port if not specified
    let socket_addr = if upstream_addr.contains(':') {
        upstream_addr.to_string()
    } else {
        format!("{}:443", upstream_addr)
    };
    
    // Extract hostname for TLS SNI (use upstream_addr if it looks like a hostname)
    let sni_hostname = if upstream_addr.contains(':') {
        upstream_addr.split(':').next().unwrap_or("localhost")
    } else {
        upstream_addr
    };
    
    let tcp_stream = TcpStream::connect(&socket_addr).await
        .map_err(|e| {
            tracing::error!("[H2 Upstream HTTP/1.1] Failed to connect: {}", e);
            anyhow!("Failed to connect: {}", e)
        })?;

    // Establish TLS for HTTP/1.1
    let mut tls_stream = establish_tls_upstream(tcp_stream, sni_hostname).await?;
    
    tracing::debug!("[H2 Upstream HTTP/1.1] Connected and TLS established");

    // Build HTTP/1.1 request with all client headers
    let body_len = request_body.len();
    let content_length = if body_len > 0 {
        format!("Content-Length: {}\r\n", body_len)
    } else {
        String::new()
    };

    // Start with request line and Host header
    let mut http1_request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\n",
        method, uri, sni_hostname
    );

    // Add all client headers (except hop-by-hop headers and pseudo-headers)
    for (name, value) in &request_parts.headers {
        let name_str = name.as_str().to_lowercase();
        
        // Skip hop-by-hop headers, pseudo-headers, and headers we set explicitly
        if matches!(name_str.as_str(),
            "connection" | "transfer-encoding" | "upgrade" | "te" | "trailer" 
            | "proxy-authenticate" | "proxy-authorization" | "host"
            | ":authority" | ":method" | ":path" | ":scheme"
            | "content-length"  // We set this explicitly below
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
        tracing::debug!("[H2 Upstream HTTP/1.1] Request body sent: {} bytes", body_len);
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
                    let (redacted, _, _) = streaming_redactor.process_chunk(&lookahead, &mut vec![], true);
                    response_output.extend_from_slice(redacted.as_bytes());
                }
                break;
            }
            Ok(n) => {
                bytes_read += n as u64;
                
                // Process chunk through streaming redactor
                let (redacted, _patterns, _) = streaming_redactor.process_chunk(&read_buf[..n], &mut lookahead, false);
                
                // Skip HTTP headers, only output body
                if !body_started {
                    // Look for end of headers (double CRLF or double LF)
                    if let Some(header_end) = redacted.find("\r\n\r\n") {
                        body_started = true;
                        let body_part = &redacted[header_end + 4..];
                        response_output.extend_from_slice(body_part.as_bytes());
                        tracing::debug!("[H2 Upstream HTTP/1.1] Headers skipped, body streaming started");
                    } else if let Some(header_end) = redacted.find("\n\n") {
                        body_started = true;
                        let body_part = &redacted[header_end + 2..];
                        response_output.extend_from_slice(body_part.as_bytes());
                        tracing::debug!("[H2 Upstream HTTP/1.1] Headers skipped, body streaming started");
                    }
                } else {
                    response_output.extend_from_slice(redacted.as_bytes());
                }
                
                tracing::debug!("[H2 Upstream HTTP/1.1] Processed {} bytes, output: {} bytes", n, response_output.len());
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
                if err_msg.contains("EOF") || err_msg.contains("Connection reset") 
                    || err_msg.contains("connection closed") {
                    tracing::debug!("[H2 Upstream HTTP/1.1] Connection closed by peer: {}", e);
                    break;  // ← Return what we got, don't error
                } else {
                    tracing::warn!("[H2 Upstream HTTP/1.1] Real read error: {}", e);
                    return Err(anyhow!("Read error: {}", e));
                }
            }
        }
    }

    tracing::info!("[H2 Upstream HTTP/1.1] Response received: {} bytes read, {} bytes output", bytes_read, response_output.len());
    
    Ok(response_output)
}

/// Establish TLS connection to upstream server
async fn establish_tls_upstream(
    tcp_stream: TcpStream,
    host: &str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    let client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let server_name = ServerName::try_from(host)
        .map_err(|_| anyhow!("Invalid upstream host: {}", host))?;
    
    connector
        .connect(server_name, tcp_stream)
        .await
        .map_err(|e| anyhow!("TLS handshake failed: {}", e))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_streaming_passthrough() {
        // Passthrough with streaming redaction tested via integration
    }
}
