use super::{extract_http_response_body, is_connection_closed_error, log_detected_secrets, read_response_direct};
use crate::mitm::config::RedactionMode;
use anyhow::{anyhow, Result};
use bytes::Bytes;
use http::Request;
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

/// Read HTTP/1.1 response directly (for PASSTHROUGH and DETECT modes)
async fn read_http1_response_direct(
    tls_stream: &mut (impl AsyncReadExt + Unpin + AsyncWriteExt),
    engine: &Arc<RedactionEngine>,
    mode: RedactionMode,
    detect_patterns: &scred_http::PatternSelector,
) -> Result<Vec<u8>> {
    tracing::debug!("[H2 Upstream HTTP/1.1] Mode: {:?} - Reading response directly", mode);
    let response_bytes = read_response_direct(tls_stream).await?;
    let body = extract_http_response_body(&response_bytes)?;

    if mode.should_detect() {
        tracing::info!("[H2 Upstream HTTP/1.1] DETECT mode - scanning for secrets");
        log_detected_secrets(engine, &response_bytes, detect_patterns);
    }

    Ok(body)
}

/// Read HTTP/1.1 response with streaming redaction (for REDACT mode)
async fn read_http1_response_redacted(
    tls_stream: &mut (impl AsyncReadExt + Unpin + AsyncWriteExt),
    engine: &Arc<RedactionEngine>,
) -> Result<Vec<u8>> {
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
                if !lookahead.is_empty() {
                    let (redacted, _, _) =
                        streaming_redactor.process_chunk(&lookahead, &mut vec![], true);
                    response_output.extend_from_slice(redacted.as_bytes());
                }
                break;
            }
            Ok(n) => {
                bytes_read += n as u64;
                let (redacted, _, _) =
                    streaming_redactor.process_chunk(&read_buf[..n], &mut lookahead, false);

                if !body_started {
                    if let Some(header_end) = redacted.find("\r\n\r\n") {
                        body_started = true;
                        response_output.extend_from_slice(redacted[header_end + 4..].as_bytes());
                    } else if let Some(header_end) = redacted.find("\n\n") {
                        body_started = true;
                        response_output.extend_from_slice(redacted[header_end + 2..].as_bytes());
                    }
                } else {
                    response_output.extend_from_slice(redacted.as_bytes());
                }
            }
            Err(e) => {
                if is_connection_closed_error(&e) {
                    tracing::debug!("[H2 Upstream HTTP/1.1] Connection closed by peer: {}", e);
                    break;
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
) -> Result<Vec<u8>> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let body = request.body();
    tracing::info!("[H2 Upstream] Forwarding via HTTP/1.1");

    let port = uri.port_u16().unwrap_or(443);

    let mut tls_stream = connect_http1_upstream(target_host, port).await?;

    let http1_request = build_http1_request_string(
        method.as_str(),
        &uri.to_string(),
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
