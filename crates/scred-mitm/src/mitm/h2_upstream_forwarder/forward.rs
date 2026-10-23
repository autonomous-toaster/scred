use super::{extract_http_response_body, is_connection_closed_error, log_detected_secrets, read_response_direct};
use crate::mitm::config::RedactionMode;
use anyhow::{anyhow, Result};
use bytes::Bytes;
use http::{Request, StatusCode};
use scred_http::upstream_connection::{
    connect_tcp, establish_tls, get_proxy_url, UpstreamConnectionConfig,
};
use scred_redactor::{RedactionEngine, StreamingConfig, StreamingRedactor};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Build HTTP/1.1 request string from method, URI, host, headers, and body length
fn build_http1_request_string(
    method: &str,
    uri: &str,
    host: &str,
    headers: &http::HeaderMap,
    body_len: usize,
) -> String {
    let content_length = if body_len > 0 {
        format!("Content-Length: {}\r\n", body_len)
    } else {
        String::new()
    };

    let mut request = format!("{} {} HTTP/1.1\r\nHost: {}\r\n", method, uri, host);

    for (name, value) in headers {
        let name_str = name.as_str().to_lowercase();

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
                | "content-length"
        ) {
            continue;
        }

        if let Ok(value_str) = value.to_str() {
            request.push_str(&format!("{}: {}\r\n", name, value_str));
        }
    }

    request.push_str(&content_length);
    request.push_str("Connection: close\r\n\r\n");
    request
}

/// Parse HTTP status code from the first line of an HTTP response.
/// Handles both "HTTP/1.1 200 OK" and "HTTP/1.0 401 Unauthorized" formats.
pub(crate) fn parse_status_from_response(response: &[u8]) -> Result<StatusCode> {
    let response_str = String::from_utf8_lossy(response);
    // Find the end of the first line
    let line_end = response_str
        .find("\r\n")
        .or_else(|| response_str.find('\n'));
    let first_line = match line_end {
        Some(pos) => &response_str[..pos],
        None => response_str.as_ref(),
    };
    // Extract the status code (second space-separated token)
    let status_str = first_line.split(' ').nth(1).ok_or_else(|| {
        anyhow!("Cannot parse status code from response line: {}", first_line)
    })?;
    let code: u16 = status_str.parse().map_err(|e| {
        anyhow!("Cannot parse status code '{}': {}", status_str, e)
    })?;
    StatusCode::from_u16(code).map_err(|e| {
        anyhow!("Invalid status code {}: {}", code, e)
    })
}

/// Read HTTP/1.1 response directly (for PASSTHROUGH and DETECT modes)
async fn read_http1_response_direct(
    tls_stream: &mut (impl AsyncReadExt + Unpin + AsyncWriteExt),
    engine: &Arc<RedactionEngine>,
    mode: RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
) -> Result<(StatusCode, Vec<u8>)> {
    tracing::debug!("[H2 Upstream HTTP/1.1] Mode: {:?} - Reading response directly", mode);
    let response_bytes = read_response_direct(tls_stream).await?;
    let status = parse_status_from_response(&response_bytes)?;
    let body = extract_http_response_body(&response_bytes)?;

    if mode.should_detect() {
        tracing::info!("[H2 Upstream HTTP/1.1] DETECT mode - scanning for secrets");
        log_detected_secrets(engine, &response_bytes, detect_patterns);
    }

    Ok((status, body))
}

/// Read HTTP/1.1 response in REDACT mode.
///
/// Buffer the response so the header block and the body can be told apart: the
/// client must receive the body only, with the status carried separately. The
/// earlier version piped the whole stream -- status line and headers included --
/// through the redactor, so whenever the header/body boundary fell outside the
/// first read the client got a body that began with "HTTP/1.1 200 OK" and was cut
/// mid-header.
///
/// Redaction itself is length-preserving, so the buffered body can go through the
/// redactor in one pass once EOF arrives (the upstream request asks for
/// "Connection: close").
async fn read_http1_response_redacted(
    tls_stream: &mut (impl AsyncReadExt + Unpin + AsyncWriteExt),
    engine: &Arc<RedactionEngine>,
) -> Result<(StatusCode, Vec<u8>)> {
    tracing::debug!("[H2 Upstream HTTP/1.1] Mode: REDACT - buffering, then redacting the body");
    let response_bytes = read_response_direct(tls_stream).await?;
    let status = parse_status_from_response(&response_bytes)?;
    let body = extract_http_response_body(&response_bytes)?;

    let redactor = StreamingRedactor::with_defaults(engine.clone());
    let mut lookahead = Vec::new();
    let (redacted, _patterns, _bytes) = redactor.process_chunk(&body, &mut lookahead, true);

    tracing::info!(
        "[H2 Upstream HTTP/1.1] Response: status={}, {} body bytes in, {} out",
        status.as_u16(),
        body.len(),
        redacted.len()
    );

    Ok((status, redacted.into_bytes()))
}

/// Connect to upstream via HTTP/1.1 and establish TLS
async fn connect_http1_upstream(host: &str, port: u16) -> Result<impl AsyncReadExt + AsyncWriteExt + Unpin> {
    let proxy_url = get_proxy_url(host, true);
    let conn_config = UpstreamConnectionConfig::https(host, port);
    let conn_config = if let Some(ref proxy) = proxy_url {
        tracing::info!("[H2 Upstream] Using proxy: {}", proxy);
        conn_config.with_proxy(proxy)
    } else {
        conn_config
    };

    let tcp_stream = connect_tcp(&conn_config).await?;
    let tls_stream = establish_tls(tcp_stream, host).await?;
    tracing::debug!("[H2 Upstream HTTP/1.1] Connected and TLS established");
    Ok(tls_stream)
}

pub(crate) async fn forward_via_http1_1(
    request: &Request<Bytes>,
    engine: &Arc<RedactionEngine>,
    _upstream_addr: &str,
    target_host: &str,
    mode: RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
    _redact_patterns: &scred_http::PatternSelector,
) -> Result<(StatusCode, Vec<u8>)> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let body = request.body();
    tracing::info!("[H2 Upstream] Forwarding via HTTP/1.1");

    let port = uri.port_u16().unwrap_or(443);

    let mut tls_stream = connect_http1_upstream(target_host, port).await?;

    // Origin-form, not the client's absolute-form proxy target.
    let target = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let http1_request = build_http1_request_string(
        method.as_str(),
        target,
        target_host,
        request.headers(),
        body.len(),
    );

    tls_stream.write_all(http1_request.as_bytes()).await?;
    if !body.is_empty() {
        tls_stream.write_all(body).await?;
    }
    tls_stream.flush().await?;
    tracing::debug!("[H2 Upstream HTTP/1.1] Request sent");

    if !mode.should_redact() {
        read_http1_response_direct(&mut tls_stream, engine, mode, detect_patterns).await
    } else {
        read_http1_response_redacted(&mut tls_stream, engine).await
    }
}
