/// HTTP/2 Stream Redaction State - Per-Stream Redaction Management
///
/// Maintains per-stream context for redacting secrets in HTTP/2 requests and responses.
/// Each stream has independent redaction state for proper isolation.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

use scred_redactor::RedactionEngine;

/// Redaction state for a single HTTP/2 stream
#[derive(Clone)]
pub struct StreamRedactionState {
    pub stream_id: u32,
    pub headers: HashMap<String, String>,
    pub body_chunks: Vec<Vec<u8>>,
    redaction_engine: Arc<RedactionEngine>,
    pub end_stream: bool,
    pub headers_complete: bool,
}

impl StreamRedactionState {
    /// Create new stream redaction state
    pub fn new(stream_id: u32, redaction_engine: Arc<RedactionEngine>) -> Self {
        debug!("Creating stream redaction state: stream_id={}", stream_id);
        Self {
            stream_id,
            headers: HashMap::new(),
            body_chunks: Vec::new(),
            redaction_engine,
            end_stream: false,
            headers_complete: false,
        }
    }

    /// Add headers to this stream
    pub fn add_headers(&mut self, headers: http::HeaderMap) -> Result<()> {
        if self.headers_complete {
            warn!("Stream {} received duplicate headers", self.stream_id);
            return Err(anyhow!("Stream {} duplicate headers", self.stream_id));
        }

        debug!("Adding headers to stream {}: {} headers", self.stream_id, headers.len());

        for (name, value) in headers.iter() {
            let name_str = name.as_str().to_string();
            if let Ok(value_str) = value.to_str() {
                self.headers.insert(name_str, value_str.to_string());
            } else {
                warn!("Stream {}: Non-UTF8 header value for {}", self.stream_id, name);
            }
        }

        self.headers_complete = true;
        Ok(())
    }

    /// Add body chunk to this stream
    pub fn add_body_chunk(&mut self, chunk: Vec<u8>) -> Result<()> {
        if self.end_stream {
            warn!("Stream {} received data after END_STREAM", self.stream_id);
            return Err(anyhow!("Stream {} data after END_STREAM", self.stream_id));
        }

        debug!(
            "Adding body chunk to stream {}: {} bytes",
            self.stream_id,
            chunk.len()
        );

        self.body_chunks.push(chunk);
        Ok(())
    }

    /// Mark stream as complete (END_STREAM received)
    pub fn mark_end_stream(&mut self) {
        debug!("Stream {} marked END_STREAM", self.stream_id);
        self.end_stream = true;
    }

    /// Check if stream is complete
    pub fn is_complete(&self) -> bool {
        self.headers_complete && self.end_stream
    }

    /// Apply redaction to headers and body
    pub fn apply_redaction(&self) -> Result<(HashMap<String, String>, Vec<u8>)> {
        if !self.is_complete() {
            return Err(anyhow!(
                "Stream {} not complete (headers_complete={}, end_stream={})",
                self.stream_id,
                self.headers_complete,
                self.end_stream
            ));
        }

        debug!("Applying redaction to stream {}", self.stream_id);

        // 1. Redact headers
        let mut redacted_headers = self.headers.clone();
        for (name, value) in redacted_headers.iter_mut() {
            let redacted_result = self.redaction_engine.redact(value);
            *value = redacted_result.redacted;

            debug!(
                "Stream {} redacted header {}: {} patterns found",
                self.stream_id,
                name,
                redacted_result.warnings.len()
            );
        }

        // 2. Combine body chunks
        let mut body = Vec::new();
        for chunk in &self.body_chunks {
            body.extend_from_slice(chunk);
        }

        // 3. Redact body
        let redacted_result = self.redaction_engine.redact(&String::from_utf8_lossy(&body));
        let redacted_body = redacted_result.redacted.as_bytes().to_vec();

        debug!(
            "Stream {} redaction complete: {} header values, {} body bytes, {} patterns",
            self.stream_id,
            redacted_headers.len(),
            redacted_body.len(),
            redacted_result.warnings.len()
        );

        Ok((redacted_headers, redacted_body))
    }

    /// Get stream statistics
    pub fn stats(&self) -> StreamStats {
        let total_body_bytes: usize = self.body_chunks.iter().map(|c| c.len()).sum();
        StreamStats {
            stream_id: self.stream_id,
            headers_count: self.headers.len(),
            body_chunks: self.body_chunks.len(),
            total_body_bytes,
            end_stream: self.end_stream,
            headers_complete: self.headers_complete,
        }
    }
}

/// Statistics for a stream
#[derive(Debug, Clone)]
pub struct StreamStats {
    pub stream_id: u32,
    pub headers_count: usize,
    pub body_chunks: usize,
    pub total_body_bytes: usize,
    pub end_stream: bool,
    pub headers_complete: bool,
}

impl std::fmt::Display for StreamStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stream {}: {} headers, {} chunks, {} bytes, end_stream={}",
            self.stream_id, self.headers_count, self.body_chunks, self.total_body_bytes, self.end_stream
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_engine() -> Arc<RedactionEngine> {
        Arc::new(RedactionEngine::new(Default::default()))
    }

    #[test]
    fn test_stream_state_creation() {
        let engine = mock_engine();
        let state = StreamRedactionState::new(1, engine);
        assert_eq!(state.stream_id, 1);
        assert!(!state.end_stream);
        assert!(!state.headers_complete);
    }

    #[test]
    fn test_add_headers() {
        let engine = mock_engine();
        let mut state = StreamRedactionState::new(1, engine);

        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            http::header::HeaderValue::from_static("application/json"),
        );

        state.add_headers(headers).unwrap();
        assert!(state.headers_complete);
    }

    #[test]
    fn test_stream_complete() {
        let engine = mock_engine();
        let mut state = StreamRedactionState::new(1, engine);

        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            http::header::HeaderValue::from_static("text/plain"),
        );
        state.add_headers(headers).unwrap();

        state.mark_end_stream();
        assert!(state.is_complete());
    }
}
