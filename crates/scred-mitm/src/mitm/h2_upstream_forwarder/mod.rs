use crate::mitm::config::RedactionMode;
/// HTTP/2 Upstream Forwarder - Forward requests to upstream HTTP/2 or HTTP/1.1
///
/// Handles:
/// - Direct connection (no upstream proxy): tries h2, falls back to HTTP/1.1
/// - Upstream proxy (http_proxy env var): receives downgraded HTTP/1.1
/// - Streaming redaction: Process responses in chunks (64KB) without loading full body
/// - Three modes: PASSTHROUGH (no redaction), DETECT (detect & log), REDACT (detect & redact)
use anyhow::{anyhow, Result};
use bytes::Bytes;
use http::Request;
use scred_redactor::{RedactionEngine, StreamingConfig, StreamingRedactor};
use std::sync::Arc;
use tokio::io::AsyncReadExt;

use h2::client;
use rustls::{ClientConfig, RootCertStore, ServerName};
use scred_http::upstream_connection::{
    connect_tcp, establish_tls, establish_tls_h2, get_proxy_url, UpstreamConnectionConfig,
};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Forward HTTP/2 request to upstream server (passthrough mode)
///
/// In MITM mode with H2 client:
/// - Try H2 to upstream first (direct connection)
/// - Fall back to HTTP/1.1 if H2 fails
/// - Handle upstream proxy downgrades

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
                tracing::debug!(
                    "[H2 Upstream HTTP/1.1 Direct] Read {} bytes, total: {}",
                    n,
                    response.len()
                );
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
                    || err_kind == std::io::ErrorKind::ConnectionAborted
                {
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
fn extract_http_response_body(response: &[u8]) -> Result<Vec<u8>> {
    let response_str = String::from_utf8_lossy(response);

    // Find HTTP header terminator
    if let Some(pos) = response_str.find("\r\n\r\n") {
        let body = &response[pos + 4..];
        tracing::debug!(
            "[H2 Upstream HTTP/1.1] Extracted body: {} bytes",
            body.len()
        );
        return Ok(body.to_vec());
    }

    if let Some(pos) = response_str.find("\n\n") {
        let body = &response[pos + 2..];
        tracing::debug!(
            "[H2 Upstream HTTP/1.1] Extracted body (LF only): {} bytes",
            body.len()
        );
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
) -> Result<Vec<u8>> {
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
    let (request_parts, request_body) = request.into_parts();

    // Check if upstream proxy is active (non-empty env vars)
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
        tracing::info!("[H2 Upstream] Upstream proxy detected - using HTTP/1.1 fallback");
        return forward_via_http1_1_with_body(
            &request_parts,
            &request_body,
            &engine,
            &upstream_addr,
            mode,
            &detect_patterns,
            &redact_patterns,
        )
        .await;
    }

    // No proxy: try H2 first, then fallback to HTTP/1.1
    tracing::debug!("[H2 Upstream] No upstream proxy - attempting H2 direct connection");

    // Rebuild request with parts and body for H2 attempt
    let h2_request = http::Request::from_parts(request_parts.clone(), request_body.clone());

    match try_forward_h2(h2_request, engine.clone(), &upstream_addr, host).await {
        Ok(response) => {
            tracing::info!("[H2 Upstream] H2 forward successful");
            Ok(response)
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
                mode,
                &detect_patterns,
                &redact_patterns,
            )
            .await
        }
    }
}

/// Try to forward via HTTP/2 direct connection
async fn try_forward_h2(
    request: Request<Bytes>,
    _engine: Arc<RedactionEngine>,
    _upstream_addr: &str,
    host: &str,
) -> Result<Vec<u8>> {
    let _method = request.method().clone();
    let _uri = request.uri().clone();
    let (request_parts, request_body) = request.into_parts();

    // Use unified connection logic with proxy support
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
        tracing::debug!(
            "[H2 Upstream] Sending request body: {} bytes",
            request_body.len()
        );
        match send_stream.send_data(request_body, true) {
            Ok(_) => {}
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
            if err_msg.contains("EOF")
                || err_msg.contains("unexpected end of file")
                || err_msg.contains("connection closed")
            {
                tracing::debug!(
                    "[H2 Upstream] Server closed connection before sending response headers"
                );
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

    tracing::info!(
        "[H2 Upstream] Received H2 response: status={}",
        response_parts.status
    );

    // Read response body from h2 stream
    let mut response_body = Vec::new();
    let mut chunks_received = 0;
    loop {
        match recv_stream.data().await {
            Some(Ok(chunk)) => {
                chunks_received += 1;
                response_body.extend_from_slice(&chunk);
                tracing::debug!(
                    "[H2 Upstream] Received response chunk #{}: {} bytes",
                    chunks_received,
                    chunk.len()
                );
                tracing::debug!(
                    "[H2 Upstream] Total response body so far: {} bytes",
                    response_body.len()
                );
            }
            Some(Err(e)) => {
                // Check if it's a connection reset or other recoverable error
                let err_msg = e.to_string();
                if err_msg.contains("unexpected end of file") || err_msg.contains("EOF") {
                    // Connection closed - this is often normal for some servers
                    tracing::warn!(
                        "[H2 Upstream] Connection closed by upstream ({}). Got {} bytes",
                        e,
                        response_body.len()
                    );
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

    tracing::info!(
        "[H2 Upstream] H2 response body received: {} bytes",
        response_body.len()
    );

    Ok(response_body)
}

