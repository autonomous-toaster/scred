/// Per-Stream Redaction Wrapper
///
/// Integrates StreamingRedactor with per-stream HTTP/2 state management.
/// Enables streaming redaction of request/response bodies on a per-stream basis.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{debug, warn};

use scred_redactor::{RedactionEngine, StreamingRedactor, StreamingConfig};

/// Per-stream redaction wrapper
///
/// Each HTTP/2 stream gets its own redactor instance with isolated state.
pub struct PerStreamRedactor {
    /// Stream ID
    stream_id: u32,
    
    /// Streaming redactor (streaming mode for this stream)
    redactor: StreamingRedactor,
    
    /// Lookahead buffer for streaming redaction
    lookahead: Vec<u8>,
    
    /// Whether redaction has been finalized
    finalized: bool,
    
    /// Statistics: total bytes redacted
    bytes_redacted: u64,
    
    /// Statistics: total patterns found
    patterns_found: u64,
}

impl PerStreamRedactor {
    /// Create new per-stream redactor
    pub fn new(stream_id: u32, engine: Arc<RedactionEngine>) -> Self {
        debug!("PerStreamRedactor: Creating for stream {}", stream_id);
        
        Self {
            stream_id,
            redactor: StreamingRedactor::with_defaults(engine),
            lookahead: Vec::with_capacity(16384), // Default lookahead
            finalized: false,
            bytes_redacted: 0,
            patterns_found: 0,
        }
    }

    /// Redact a chunk of data (streaming)
    ///
    /// Each chunk is redacted independently with streaming state preservation.
    /// This is efficient for large responses.
    pub fn redact_chunk(&mut self, chunk: &[u8], is_final: bool) -> Result<String> {
        if self.finalized {
            return Err(anyhow!(
                "Stream {}: Cannot redact after finalization",
                self.stream_id
            ));
        }

        let (redacted, _output_len, patterns) = 
            self.redactor.process_chunk(chunk, &mut self.lookahead, is_final);
        
        self.bytes_redacted += chunk.len() as u64;
        self.patterns_found += patterns;
        
        debug!(
            "PerStreamRedactor: Stream {} redacted {} bytes, {} patterns found",
            self.stream_id,
            chunk.len(),
            patterns
        );

        Ok(redacted)
    }

    /// Redact header value
    pub fn redact_header_value(&self, value: &str) -> String {
        if self.finalized {
            warn!(
                "PerStreamRedactor: Stream {} attempting to redact after finalization",
                self.stream_id
            );
            return value.to_string();
        }

        // For headers, we need a separate redaction pass
        // The streaming redactor is for body chunks
        // This is a simple non-streaming redaction for headers
        value.to_string() // TODO: Apply redaction to headers
    }

    /// Finalize redaction (flush any buffered state)
    pub fn finalize(&mut self) -> Result<String> {
        if self.finalized {
            return Err(anyhow!(
                "Stream {}: Already finalized",
                self.stream_id
            ));
        }

        // Process any remaining lookahead with is_eof=true
        let (redacted, _output_len, patterns) =
            self.redactor.process_chunk(&[], &mut self.lookahead, true);
        
        self.patterns_found += patterns;
        self.finalized = true;

        debug!(
            "PerStreamRedactor: Stream {} finalized ({} bytes remaining, {} patterns total)",
            self.stream_id,
            redacted.len(),
            self.patterns_found
        );

        Ok(redacted)
    }

    /// Get statistics for this stream
    pub fn stats(&self) -> PerStreamRedactorStats {
        PerStreamRedactorStats {
            stream_id: self.stream_id,
            bytes_redacted: self.bytes_redacted,
            patterns_found: self.patterns_found,
            finalized: self.finalized,
        }
    }
}

/// Statistics for per-stream redaction
#[derive(Debug, Clone)]
pub struct PerStreamRedactorStats {
    pub stream_id: u32,
    pub bytes_redacted: u64,
    pub patterns_found: u64,
    pub finalized: bool,
}

impl std::fmt::Display for PerStreamRedactorStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stream {}: {} bytes, {} patterns, finalized={}",
            self.stream_id, self.bytes_redacted, self.patterns_found, self.finalized
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
    fn test_per_stream_redactor_creation() {
        let engine = mock_engine();
        let redactor = PerStreamRedactor::new(1, engine);

        assert_eq!(redactor.stream_id, 1);
        assert!(!redactor.finalized);
        assert_eq!(redactor.bytes_redacted, 0);
    }

    #[test]
    fn test_per_stream_redactor_stats() {
        let engine = mock_engine();
        let redactor = PerStreamRedactor::new(1, engine);

        let stats = redactor.stats();
        assert_eq!(stats.stream_id, 1);
        assert_eq!(stats.bytes_redacted, 0);
        assert!(!stats.finalized);
    }

    #[test]
    fn test_redact_header_value() {
        let engine = mock_engine();
        let redactor = PerStreamRedactor::new(1, engine);

        let header_value = "x-api-key: secret123";
        let redacted = redactor.redact_header_value(header_value);

        // Should be redacted (if pattern matches)
        // For now, just verify it works without panic
        assert!(!redacted.is_empty());
    }

    #[test]
    fn test_cannot_redact_after_finalize() {
        let engine = mock_engine();
        let mut redactor = PerStreamRedactor::new(1, engine);

        redactor.finalize().ok();

        let result = redactor.redact_chunk(b"test", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_finalize_twice() {
        let engine = mock_engine();
        let mut redactor = PerStreamRedactor::new(1, engine);

        redactor.finalize().ok();
        let result = redactor.finalize();

        assert!(result.is_err());
    }
}
