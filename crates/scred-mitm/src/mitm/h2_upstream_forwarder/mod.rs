#![allow(clippy::empty_line_after_doc_comments)]
use crate::mitm::config::RedactionMode;
mod forward;
/// HTTP/2 Upstream Forwarder - Forward requests to upstream HTTP/2 or HTTP/1.1
///
/// Handles:
/// - Direct connection (no upstream proxy): tries h2, falls back to HTTP/1.1
/// - Upstream proxy (http_proxy env var): receives downgraded HTTP/1.1
/// - Streaming redaction: Process responses in chunks (64KB) without loading full body
/// - Three modes: PASSTHROUGH (no redaction), DETECT (detect & log), REDACT (detect & redact)
use anyhow::{anyhow, Result};
use bytes::Bytes;
use forward::{forward_via_http1_1};
use http::{Request, StatusCode};
use scred_redactor::RedactionEngine;
use std::sync::Arc;
use tokio::io::AsyncReadExt;

use h2::client;

use scred_http::upstream_connection::{
    connect_tcp, establish_tls_h2, get_proxy_url, UpstreamConnectionConfig,
};

/// Forward HTTP/2 request to upstream server (passthrough mode)
///
/// In MITM mode with H2 client:
/// - Try H2 to upstream first (direct connection)
/// - Fall back to HTTP/1.1 if H2 fails
/// - Handle upstream proxy downgrades
// ============================================================================
// Helper Functions
// ============================================================================

/// Check if an I/O error indicates connection closed (not a real error)
pub(crate) fn is_connection_closed_error(e: &std::io::Error) -> bool {
    let err_msg = e.to_string();
    let err_kind = e.kind();

    err_msg.contains("EOF")
        || err_msg.contains("Connection reset")
        || err_msg.contains("connection closed")
        || err_msg.contains("unexpected end of file")
        || err_kind == std::io::ErrorKind::UnexpectedEof
        || err_kind == std::io::ErrorKind::ConnectionReset
        || err_kind == std::io::ErrorKind::ConnectionAborted
}

/// Read complete HTTP response directly without streaming redaction
/// Used for PASSTHROUGH and DETECT modes
#[allow(clippy::needless_borrow, clippy::doc_lazy_continuation)]
pub(crate) async fn read_response_direct(
    tls_stream: &mut (impl AsyncReadExt + Unpin),
) -> Result<Vec<u8>> {
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
                tracing::debug!(
                    "[H2 Upstream HTTP/1.1 Direct] Read {} bytes, total: {}",
                    n,
                    response.len()
                );
            }
            Err(e) => {
                if is_connection_closed_error(&e) {
                    tracing::debug!(
                        "[H2 Upstream HTTP/1.1 Direct] Connection closed by peer: {}",
                        e
                    );
                    break;
                } else {
                    tracing::warn!("[H2 Upstream HTTP/1.1 Direct] Read error: {}", e);
                    return Err(anyhow!("Read error: {}", e));
                }
            }
        }
    }

    tracing::info!(
        "[H2 Upstream HTTP/1.1 Direct] Total response received: {} bytes",
        response.len()
    );
    Ok(response)
}

/// Extract HTTP response body from full HTTP response (headers + body)
pub(crate) fn extract_http_response_body(response: &[u8]) -> Result<Vec<u8>> {
    let Some((header_end, body_start)) = find_header_end(response) else {
        tracing::debug!(
            "[H2 Upstream HTTP/1.1] No header terminator found, returning full response"
        );
        return Ok(response.to_vec());
    };

    let headers = parse_header_lines(&response[..header_end]);
    let mut chunked = false;
    let mut content_length: Option<usize> = None;
    for (name, value) in &headers {
        match name.as_str() {
            "transfer-encoding" if value.to_ascii_lowercase().contains("chunked") => chunked = true,
            "content-length" => content_length = value.trim().parse::<usize>().ok(),
            _ => {}
        }
    }

    let raw = &response[body_start..];
    let body = if chunked {
        // A chunked body arrives with its framing ("1a2b\r\n<data>\r\n…0\r\n\r\n"). We
        // answer the client with a Content-Length, so the framing must go: without
        // this the client reads "174" as the first bytes of its JSON or SSE stream.
        let decoded = dechunk(raw);
        tracing::debug!(
            "[H2 Upstream HTTP/1.1] De-chunked body: {} -> {} bytes",
            raw.len(),
            decoded.len()
        );
        decoded
    } else if let Some(len) = content_length {
        raw[..len.min(raw.len())].to_vec()
    } else {
        raw.to_vec()
    };

    tracing::debug!("[H2 Upstream HTTP/1.1] Extracted body: {} bytes", body.len());
    Ok(body)
}

/// Offset of the header block and of the first body byte.
fn find_header_end(response: &[u8]) -> Option<(usize, usize)> {
    if let Some(p) = response.windows(4).position(|w| w == b"\r\n\r\n") {
        return Some((p, p + 4));
    }
    response
        .windows(2)
        .position(|w| w == b"\n\n")
        .map(|p| (p, p + 2))
}

/// Lowercased (name, value) pairs from a raw header block.
fn parse_header_lines(header_block: &[u8]) -> Vec<(String, String)> {
    let text = String::from_utf8_lossy(header_block);
    text.lines()
        .skip(1) // status line
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect()
}

/// Decode a chunked body, stopping at the terminating chunk. Tolerates a
/// truncated tail: whatever was decoded so far is returned.
pub(crate) fn dechunk(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len());
    let mut pos = 0usize;
    while pos < body.len() {
        let Some(eol) = body[pos..].windows(2).position(|w| w == b"\r\n") else {
            break;
        };
        let size_line = &body[pos..pos + eol];
        let size_hex = size_line.split(|b| *b == b';').next().unwrap_or(size_line);
        let Ok(size_str) = std::str::from_utf8(size_hex) else { break };
        let Ok(size) = usize::from_str_radix(size_str.trim(), 16) else { break };
        pos += eol + 2;
        if size == 0 {
            break;
        }
        let end = (pos + size).min(body.len());
        out.extend_from_slice(&body[pos..end]);
        pos = end + 2;
    }
    out
}

/// Log detected secrets in response without redacting
/// Filters by detect_patterns selector - only logs secrets that match the selector
pub(crate) fn log_detected_secrets(
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
        tracing::info!(
            "[DETECT] Found {} secrets in response (filtered by selector):",
            filtered_warnings.len()
        );
        for (idx, warning) in filtered_warnings.iter().enumerate() {
            tracing::info!(
                "[DETECT]   [{}] pattern_type: {}, count: {}",
                idx + 1,
                warning.pattern_type,
                warning.count
            );
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
) -> Result<(StatusCode, Vec<u8>)> {
    let _method = request.method().clone();
    let _uri = request.uri().clone();

    tracing::info!(
        "[H2 Upstream] Forwarding {} {} (host: {}, upstream: {})",
        _method,
        _uri,
        host,
        upstream_addr
    );

    // Extract body from request
    let (mut request_parts, request_body) = request.into_parts();

    // Keep the absolute target here: HTTP/2 derives :scheme and :authority from
    // the URI, and an origin-form URI produces an invalid request. The HTTP/1.1
    // writer takes path_and_query() itself (see forward_via_http1_1).

    // Rebuild request with parts and body for H2 attempt
    let h2_request = http::Request::from_parts(request_parts.clone(), request_body.clone());

    match try_forward_h2(h2_request, engine.clone(), host, mode, &detect_patterns, &redact_patterns).await {
        Ok((status, response)) => {
            tracing::info!("[H2 Upstream] H2 forward successful, status={}", status);
            Ok((status, response))
        }
        Err(e) => {
            tracing::warn!(
                "[H2 Upstream] H2 forward failed ({}), falling back to HTTP/1.1",
                e
            );
            // Rebuild request for HTTP/1.1 fallback
            let http1_request = http::Request::from_parts(request_parts, request_body);
            forward_via_http1_1(
                &http1_request,
                &engine,
                &upstream_addr,
                host,
                mode,
                &detect_patterns,
                &redact_patterns,
            )
            .await
        }
    }
}

/// Connect to upstream via H2 and establish TLS with ALPN h2
async fn connect_h2_upstream(host: &str) -> Result<(client::SendRequest<Bytes>, tokio::task::JoinHandle<()>)> {
    let proxy_url = get_proxy_url(host, true);
    let conn_config = UpstreamConnectionConfig::https(host, 443);
    let conn_config = if let Some(ref proxy) = proxy_url {
        tracing::info!("[H2 Upstream] Using proxy: {}", proxy);
        conn_config.with_proxy(proxy)
    } else {
        conn_config
    };

    let tcp_stream = connect_tcp(&conn_config).await?;
    let tls_stream = establish_tls_h2(tcp_stream, host).await?;
    tracing::debug!("[H2 Upstream] TLS handshake complete with {}", host);

    // Speak h2 only if the peer actually agreed to it. Announcing h2 and then
    // sending HTTP/2 frames to a hop that negotiated http/1.1 is how a request
    // ends up answered with a bogus 400 (or an empty 200): the hop parses our
    // frames as HTTP/1.1. Return an error so the caller takes the HTTP/1.1 path,
    // which is what the peer asked for.
    let negotiated = tls_stream
        .get_ref()
        .1
        .alpn_protocol()
        .map(|p| p.to_vec());
    if negotiated.as_deref() != Some(b"h2") {
        let shown = negotiated
            .as_deref()
            .map(|p| String::from_utf8_lossy(p).to_string())
            .unwrap_or_else(|| "none".to_string());
        return Err(anyhow!(
            "upstream {} negotiated ALPN {}, not h2",
            host,
            shown
        ));
    }

    let (send_request, connection) = client::handshake(tls_stream).await?;
    tracing::debug!("[H2 Upstream] H2 client handshake complete");

    let connection_handle = tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::debug!("[H2 Upstream] Connection driver ended: {}", e);
        }
    });

    Ok((send_request, connection_handle))
}

/// Send H2 request with optional body
async fn send_h2_request(
    send_request: &mut client::SendRequest<Bytes>,
    request_parts: http::request::Parts,
    request_body: Bytes,
    connection_handle: &tokio::task::JoinHandle<()>,
) -> Result<(http::response::Response<()>, h2::RecvStream)> {
    let has_body = !request_body.is_empty();
    let upstream_request = http::Request::from_parts(request_parts, ());

    let (response_future, mut send_stream) = send_request
        .send_request(upstream_request, !has_body)
        .map_err(|e| {
            connection_handle.abort();
            anyhow!("Failed to send request: {}", e)
        })?;

    if has_body {
        tracing::debug!("[H2 Upstream] Sending request body: {} bytes", request_body.len());
        send_stream.send_data(request_body, true).map_err(|e| {
            connection_handle.abort();
            anyhow!("Failed to send body: {}", e)
        })?;
    }

    let response = match response_future.await {
        Ok(r) => r,
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("EOF") || err_msg.contains("unexpected end of file") || err_msg.contains("connection closed") {
                connection_handle.abort();
                return Err(anyhow!("H2 connection closed before response: {}", e));
            }
            connection_handle.abort();
            return Err(anyhow!("Response error: {}", e));
        }
    };

    let (response_parts, recv_stream) = response.into_parts();
    tracing::info!("[H2 Upstream] Received H2 response: status={}", response_parts.status);

    Ok((http::Response::from_parts(response_parts, ()), recv_stream))
}

/// Read response body from H2 stream
async fn read_h2_response_body(recv_stream: &mut h2::RecvStream) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    let mut chunks_received = 0;

    loop {
        match recv_stream.data().await {
            Some(Ok(chunk)) => {
                chunks_received += 1;
                body.extend_from_slice(&chunk);
                tracing::debug!("[H2 Upstream] Received response chunk #{}: {} bytes", chunks_received, chunk.len());
            }
            Some(Err(e)) => {
                let err_msg = e.to_string();
                if err_msg.contains("unexpected end of file") || err_msg.contains("EOF") {
                    tracing::warn!("[H2 Upstream] Connection closed by upstream ({}). Got {} bytes", e, body.len());
                    break;
                }
                return Err(anyhow!("Failed to read response body: {}", e));
            }
            None => {
                tracing::debug!("[H2 Upstream] Response stream ended");
                break;
            }
        }
    }

    tracing::info!("[H2 Upstream] H2 response body received: {} bytes", body.len());
    Ok(body)
}

/// Apply redaction to H2 response based on mode
fn apply_h2_response_redaction(
    response_body: &[u8],
    engine: &Arc<RedactionEngine>,
    mode: RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
) -> Result<Vec<u8>> {
    if mode.should_redact() {
        tracing::debug!("[H2 Upstream] Mode: REDACT - Applying redaction to H2 response");
        // Byte-preserving: a String round-trip turns every non-UTF-8 byte into
        // U+FFFD, so a binary response came back longer and corrupt (1000 random
        // bytes arrived as 1787).
        let _ = engine;
        let mut redacted = response_body.to_vec();
        let detection = scred_redactor::scred_detector::detect_all(&redacted);
        let matches = detection.matches.len();
        scred_redactor::scred_detector::redact_in_place(&mut redacted, &detection.matches);
        tracing::info!(
            "[H2 Upstream] Redacted H2 response: {} bytes -> {} bytes ({} matches)",
            response_body.len(),
            redacted.len(),
            matches
        );
        Ok(redacted)
    } else if mode.should_detect() {
        tracing::debug!("[H2 Upstream] Mode: DETECT - Scanning H2 response for secrets");
        log_detected_secrets(engine, response_body, detect_patterns);
        Ok(response_body.to_vec())
    } else {
        tracing::debug!("[H2 Upstream] Mode: PASSTHROUGH - No redaction applied");
        Ok(response_body.to_vec())
    }
}

/// Try to forward via HTTP/2 direct connection
async fn try_forward_h2(
    request: Request<Bytes>,
    engine: Arc<RedactionEngine>,
    host: &str,
    mode: RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
    _redact_patterns: &scred_http::PatternSelector,
) -> Result<(StatusCode, Vec<u8>)> {
    let (request_parts, request_body) = request.into_parts();

    // Connect to upstream via H2
    let (mut send_request, connection_handle) = connect_h2_upstream(host).await?;

    // Send request with optional body
    let (response, mut recv_stream) = send_h2_request(
        &mut send_request,
        request_parts,
        request_body,
        &connection_handle,
    )
    .await?;

    let status = response.status();

    // Read response body
    let response_body = read_h2_response_body(&mut recv_stream).await?;

    // Apply redaction based on mode
    let redacted_body = apply_h2_response_redaction(&response_body, &engine, mode, detect_patterns)?;

    Ok((status, redacted_body))
}

#[cfg(test)]
pub(crate) mod tests;

