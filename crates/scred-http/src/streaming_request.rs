/// Phase 3b: Streaming Request Handler
///
/// Streams HTTP requests from client → redactor → upstream server
/// without buffering the entire request body.

use anyhow::{anyhow, Result};
use scred_redactor::StreamingRedactor;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::debug;

use crate::http_headers::parse_http_headers;

/// Configuration for streaming request handling
#[derive(Clone, Debug)]
pub struct StreamingRequestConfig {
    /// Enable debug logging
    pub debug: bool,
    /// Maximum headers size (default: 64KB)
    pub max_headers_size: usize,
}

impl Default for StreamingRequestConfig {
    fn default() -> Self {
        Self {
            debug: false,
            max_headers_size: 64 * 1024,
        }
    }
}

/// Helper: Apply selector filtering to redacted text
/// If selector is None, returns redacted text as-is (backward compatible)
/// If selector is Some, uses ConfigurableEngine to filter which patterns stay redacted

/// Handle HTTP request with streaming
///
/// Flow:
/// 1. Read and parse request line (METHOD URL HTTP/VERSION)
/// 2. Parse headers (non-streaming)
/// 3. Stream body through redactor to upstream
///
/// # Arguments
/// * `client_reader` - Read half from client
/// * `upstream_writer` - Write half to upstream
/// * `request_line` - First line (METHOD URL HTTP/VERSION)
/// * `redactor` - Streaming redactor
/// * `config` - Configuration
///
/// # Returns
/// Result with stats (bytes_streamed, patterns_found, etc.)
pub async fn stream_request_to_upstream<R, W>(
    client_reader: &mut BufReader<R>,
    mut upstream_writer: W,
    request_line: &str,
    redactor: Arc<StreamingRedactor>,
    config: StreamingRequestConfig,
) -> Result<StreamingStats>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    use tracing::info;
    
    info!("[stream_request_to_upstream] ENTRY: request_line={}", request_line);

    // 1. Parse headers (non-streaming)
    info!("[stream_request_to_upstream] STEP 1: Parsing headers from client...");
    let headers = parse_http_headers(client_reader).await?;
    info!("[stream_request_to_upstream] STEP 1 DONE: Parsed {} header lines, content_length={:?}, is_chunked={}", 
        headers.headers.len(), headers.content_length, headers.is_chunked());

    if config.debug {
        debug!("[streaming] Headers parsed: {:?}", headers);
    }

    // 2. Forward request line to upstream
    info!("[stream_request_to_upstream] STEP 2: Writing request line to upstream...");
    upstream_writer
        .write_all(format!("{}\r\n", request_line).as_bytes())
        .await?;
    info!("[stream_request_to_upstream] STEP 2 DONE: Request line sent");

    // 3. Forward headers to upstream (no redaction needed - headers don't contain secrets in body)
    // Actually, headers might contain Authorization, so we should redact them too
    info!("[stream_request_to_upstream] STEP 3: Redacting and writing headers to upstream...");
    let (redacted_headers, _) = redactor.redact_buffer(headers.raw_headers.as_bytes());
    // NOTE: raw_headers already includes the final \r\n blank line, don't add another!
    let headers_len = redacted_headers.len();
    upstream_writer.write_all(redacted_headers.as_bytes()).await?;
    info!("[stream_request_to_upstream] STEP 3 DONE: Headers sent ({} bytes)", headers_len);

    // 4. Stream body through redactor
    info!("[stream_request_to_upstream] STEP 4: Processing request body...");
    let mut stats = StreamingStats::default();

    if let Some(content_length) = headers.content_length {
        // Content-Length: stream exactly N bytes
        info!("[stream_request_to_upstream] STEP 4a: Streaming body with content-length={}", content_length);
        stats = stream_request_body_content_length(
            client_reader,
            &mut upstream_writer,
            content_length,
            redactor,
            &config,
        )
        .await?;
        info!("[stream_request_to_upstream] STEP 4a DONE: Body streamed (bytes_read={}, bytes_written={})", stats.bytes_read, stats.bytes_written);
    } else if headers.is_chunked() {
        // Transfer-Encoding: chunked
        info!("[stream_request_to_upstream] STEP 4b: ERROR - Chunked requests not supported");
        return Err(anyhow!("Chunked requests not yet supported in Phase 3b"));
    } else {
        // No body
        info!("[stream_request_to_upstream] STEP 4c: No body");
        stats = StreamingStats::default();
    }

    info!("[stream_request_to_upstream] STEP 5: Flushing upstream writer...");
    upstream_writer.flush().await?;
    info!("[stream_request_to_upstream] STEP 5 DONE: Upstream flushed");

    // Report redaction stats at INFO level if anything was redacted
    if stats.patterns_found > 0 {
        info!("[REDACTION] Request body: {} patterns found and redacted", stats.patterns_found);
    }

    if config.debug {
        debug!(
            "[streaming] Request complete: {} bytes streamed, {} patterns found",
            stats.bytes_read, stats.patterns_found
        );
    }

    info!("[stream_request_to_upstream] EXIT: SUCCESS (bytes_read={}, bytes_written={})", stats.bytes_read, stats.bytes_written);
    Ok(stats)
}

/// Stream request body with known Content-Length
async fn stream_request_body_content_length<R, W>(
    client_reader: &mut BufReader<R>,
    upstream_writer: &mut W,
    content_length: usize,
    redactor: Arc<StreamingRedactor>,
    _config: &StreamingRequestConfig,
) -> Result<StreamingStats>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    debug!("[streaming] Streaming request body: {} bytes", content_length);

    let mut stats = StreamingStats::default();
    let mut remaining = content_length;
    let mut lookahead = Vec::new();

    while remaining > 0 {
        // Read chunk
        let chunk_size = std::cmp::min(remaining, 64 * 1024);
        let mut chunk = vec![0u8; chunk_size];
        client_reader.read_exact(&mut chunk).await?;

        // Redact chunk (streaming redaction uses all patterns - selector filtering not supported)
        let is_eof = remaining == chunk_size;
        let (output, bytes_written, patterns) = redactor.process_chunk(&chunk, &mut lookahead, is_eof);

        // Write redacted chunk (no selector filtering - streaming preserves all redactions)
        upstream_writer.write_all(output.as_bytes()).await?;

        stats.bytes_read += chunk.len() as u64;
        stats.bytes_written += bytes_written;
        stats.patterns_found += patterns;
        remaining -= chunk_size;
    }

    stats.chunks_processed = (content_length / (64 * 1024)) as u64 + 1;
    Ok(stats)
}

/// Statistics from streaming request
#[derive(Debug, Clone, Default)]
pub struct StreamingStats {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub chunks_processed: u64,
    pub patterns_found: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_stats() {
        let mut stats = StreamingStats::default();
        stats.bytes_read = 100;
        stats.patterns_found = 5;
        assert_eq!(stats.bytes_read, 100);
        assert_eq!(stats.patterns_found, 5);
    }

    #[test]
    fn test_config_default() {
        let config = StreamingRequestConfig::default();
        assert_eq!(config.max_headers_size, 64 * 1024);
        assert!(!config.debug);
    }
}
