/// Phase 4: Chunked + Phase 3c: Streaming Response Handler
///
/// Streams HTTP responses from upstream → redactor → client
/// without buffering the entire response body.
/// Now with support for chunked transfer-encoding.
use anyhow::Result;
use scred_redactor::{RedactionStream, StreamingRedactor};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::debug;

use crate::chunked_parser::ChunkedParser;
use crate::http_headers::parse_http_headers;

/// Normalize Location header by removing default ports and optionally rewriting to proxy
fn normalize_location_header(
    headers: &mut Vec<(String, String)>,
    upstream_host: Option<&str>,
    proxy_host: Option<&str>,
    client_scheme: Option<&str>,
) {
    let Some(location_pos) = headers
        .iter()
        .position(|(k, _)| k.eq_ignore_ascii_case("Location"))
    else {
        return;
    };

    let mut location = headers[location_pos].1.clone();
    debug!("[Location] Found Location header: {}", location);

    // Remove :443 from https:// URLs and :80 from http:// URLs
    if location.contains("https://") && location.contains(":443/") {
        location = location.replace(":443/", "/");
        debug!("[Location] Normalized (removed default HTTPS port): {}", location);
    } else if location.contains("http://") && location.contains(":80/") {
        location = location.replace(":80/", "/");
        debug!("[Location] Normalized (removed default HTTP port): {}", location);
    }

    // Optionally rewrite Location headers to point back to proxy
    if let Some(proxy_hostname) = proxy_host {
        if crate::location_rewriter::is_absolute_uri(&location) {
            let scheme = client_scheme.unwrap_or("http");
            let rewritten = crate::location_rewriter::rewrite_location_to_proxy(
                &location, scheme, proxy_hostname,
            );
            headers[location_pos].1 = rewritten.clone();
            debug!("[Location] Rewriting absolute-URI to proxy: {} → {}", location, rewritten);
            return;
        }
    }

    headers[location_pos].1 = location;
}

/// Forward response line and headers to client with redaction
async fn forward_response_headers<W: AsyncWriteExt + Unpin>(
    client_writer: &mut W,
    response_line: &str,
    headers: &[(String, String)],
    redactor: &StreamingRedactor,
    config: &StreamingResponseConfig,
) -> Result<scred_redactor::StreamingStats> {
    client_writer
        .write_all(format!("{}\r\n", response_line).as_bytes())
        .await?;

    let mut forwarded_headers = headers
        .iter()
        .filter(|(k, _)| {
            !k.eq_ignore_ascii_case("content-length")
                && !k.eq_ignore_ascii_case("transfer-encoding")
                && !k.eq_ignore_ascii_case("connection")
        })
        .cloned()
        .collect::<Vec<_>>();

    forwarded_headers.push(("Connection".to_string(), "close".to_string()));
    if config.add_scred_header {
        forwarded_headers.push(("X-SCRED-Redacted".to_string(), "true".to_string()));
    }

    let mut headers_text = String::new();
    for (key, value) in &forwarded_headers {
        headers_text.push_str(key);
        headers_text.push_str(": ");
        headers_text.push_str(value);
        headers_text.push_str("\r\n");
    }

    let (redacted_headers, header_stats) = redactor.redact_buffer(headers_text.as_bytes());
    client_writer.write_all(redacted_headers.as_bytes()).await?;
    client_writer.write_all(b"\r\n").await?;

    if header_stats.patterns_found > 0 {
        debug!(
            "[REDACTION] Headers: {} patterns found and redacted",
            header_stats.patterns_found
        );
    }

    Ok(header_stats)
}

/// Stream response body through redactor based on content-length or chunked encoding
async fn stream_response_body<R, W>(
    upstream_reader: &mut BufReader<R>,
    client_writer: &mut W,
    headers: &crate::http_headers::HttpHeaders,
    redactor: Arc<StreamingRedactor>,
    config: &StreamingResponseConfig,
    response_line: &str,
) -> Result<StreamingStats>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let should_have_no_body = response_is_head_or_bodyless(response_line);

    if should_have_no_body {
        debug!("[streaming] Response has no body by status/method semantics");
        return Ok(StreamingStats::default());
    }

    if let Some(content_length) = headers.content_length {
        stream_response_body_content_length_passthrough(
            upstream_reader, client_writer, content_length, redactor, config,
        )
        .await
    } else if headers.is_chunked() {
        stream_response_body_chunked_passthrough(
            upstream_reader, client_writer, redactor, config,
        )
        .await
    } else {
        debug!("[streaming] No response body");
        Ok(StreamingStats::default())
    }
}

/// Configuration for streaming response handling
#[derive(Clone, Debug)]
pub struct StreamingResponseConfig {
    /// Enable debug logging
    pub debug: bool,
    /// Add X-SCRED-Redacted header to response
    pub add_scred_header: bool,
}

impl Default for StreamingResponseConfig {
    fn default() -> Self {
        Self {
            debug: false,
            add_scred_header: true,
        }
    }
}

/// Handle HTTP response with streaming
///
/// Flow:
/// 1. Read response line from upstream
/// 2. Parse headers (non-streaming)
/// 3. Add SCRED headers if enabled
/// 4. Stream body through redactor to client
///
/// # Arguments
/// * `upstream_reader` - Read half from upstream
/// * `client_writer` - Write half to client
/// * `response_line` - First line (HTTP/VERSION STATUS MESSAGE)
/// * `redactor` - Streaming redactor
/// * `config` - Configuration
///
/// # Returns
/// Result with stats
#[allow(clippy::too_many_arguments)]
pub async fn stream_response_to_client<R, W>(
    upstream_reader: &mut BufReader<R>,
    mut client_writer: W,
    response_line: &str,
    redactor: Arc<StreamingRedactor>,
    config: StreamingResponseConfig,
    upstream_host: Option<&str>,
    proxy_host: Option<&str>,
    client_scheme: Option<&str>,
) -> Result<StreamingStats>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    debug!("[streaming] Response: {}", response_line);

    // 1. Parse headers (non-streaming)
    let headers = parse_http_headers(upstream_reader, true).await?;

    if config.debug {
        debug!(
            "[streaming] Parsed response headers: content_length={:?}, transfer_encoding={:?}, connection={:?}, content_type={:?}",
            headers.content_length,
            headers.transfer_encoding,
            headers.connection,
            headers.content_type,
        );
        debug!("[streaming] Response headers parsed");
    }

    // 2. Build forwarded headers list
    let mut forwarded_headers = headers
        .headers
        .iter()
        .filter(|(k, _)| {
            !k.eq_ignore_ascii_case("content-length")
                && !k.eq_ignore_ascii_case("transfer-encoding")
                && !k.eq_ignore_ascii_case("connection")
        })
        .cloned()
        .collect::<Vec<_>>();

    // 3. Normalize Location headers
    normalize_location_header(&mut forwarded_headers, upstream_host, proxy_host, client_scheme);

    // 4. Forward response line and headers to client
    let header_stats = forward_response_headers(
        &mut client_writer,
        response_line,
        &forwarded_headers,
        &redactor,
        &config,
    )
    .await?;

    // 5. Stream body through redactor
    let mut stats = stream_response_body(
        upstream_reader,
        &mut client_writer,
        &headers,
        redactor,
        &config,
        response_line,
    )
    .await?;

    stats.patterns_found += header_stats.patterns_found;

    client_writer.flush().await?;

    // Report redaction stats at WARNING level if anything was redacted
    if stats.patterns_found > 0 {
        debug!(
            "[REDACTION] Body: {} patterns found and redacted",
            stats.patterns_found
        );
    }

    if config.debug {
        debug!(
            "[streaming] Response body stats: bytes_read={}, bytes_written={}, chunks_processed={}, patterns_found={}",
            stats.bytes_read,
            stats.bytes_written,
            stats.chunks_processed,
            stats.patterns_found,
        );
        debug!(
            "[streaming] Response complete: {} bytes streamed, {} patterns found",
            stats.bytes_read, stats.patterns_found
        );
    }

    Ok(stats)
}

/// Stream response body with known Content-Length, delimited downstream by connection close.
async fn stream_response_body_content_length_passthrough<R, W>(
    upstream_reader: &mut BufReader<R>,
    client_writer: &mut W,
    content_length: usize,
    redactor: Arc<StreamingRedactor>,
    _config: &StreamingResponseConfig,
) -> Result<StreamingStats>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    debug!(
        "[streaming] Streaming response body with connection-close framing: {} bytes",
        content_length
    );

    let engine = redactor.engine().clone();
    let mut stream = RedactionStream::new(engine);
    let mut stats = StreamingStats::default();
    let mut remaining = content_length;

    while remaining > 0 {
        let chunk_size = std::cmp::min(remaining, 64 * 1024);
        let mut chunk = vec![0u8; chunk_size];
        upstream_reader.read_exact(&mut chunk).await?;

        let output = stream.feed(&chunk);
        client_writer.write_all(&output).await?;

        stats.bytes_read += chunk.len() as u64;
        stats.bytes_written += output.len() as u64;
        remaining -= chunk_size;
    }

    let (final_output, final_stats) = stream.finalize();
    if !final_output.is_empty() {
        client_writer.write_all(&final_output).await?;
    }
    stats.bytes_written += final_output.len() as u64;
    stats.patterns_found = final_stats.patterns_found;
    stats.chunks_processed = (content_length / (64 * 1024)) as u64 + 1;

    Ok(stats)
}

/// Stream response body with chunked transfer-encoding, but send plain body downstream
/// and rely on connection close as framing.
async fn stream_response_body_chunked_passthrough<R, W>(
    upstream_reader: &mut BufReader<R>,
    client_writer: &mut W,
    redactor: Arc<StreamingRedactor>,
    _config: &StreamingResponseConfig,
) -> Result<StreamingStats>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    debug!("[streaming] Streaming chunked response body with connection-close framing");

    let mut parser = ChunkedParser::new();
    parser.init_stream(redactor.engine().clone());
    let mut total_stats = StreamingStats::default();

    loop {
        let (data, chunk_stats) = parser.next_chunk(upstream_reader, redactor.clone()).await?;

        if data.is_empty() {
            break;
        }

        client_writer.write_all(&data).await?;

        total_stats.bytes_read += chunk_stats.total_data_bytes;
        total_stats.bytes_written += data.len() as u64;
        total_stats.patterns_found += chunk_stats.patterns_found;
        total_stats.chunks_processed += chunk_stats.chunks_read;
    }

    debug!(
        "[streaming] Chunked response complete: {} bytes",
        total_stats.bytes_read
    );
    Ok(total_stats)
}

/// Statistics from streaming response
#[derive(Debug, Clone, Default)]
pub struct StreamingStats {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub chunks_processed: u64,
    pub patterns_found: u64,
}

fn response_is_head_or_bodyless(response_line: &str) -> bool {
    let mut parts = response_line.split_whitespace();
    let _http_version = parts.next();
    let status_code = parts
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    // 1xx (informational), 204 (no content), 304 (not modified) have no body
    // Note: text/event-stream (SSE) and streaming responses DO have a body!
    (100..200).contains(&status_code) || status_code == 204 || status_code == 304
}
