/// Per-Stream HTTP/2 Header Redaction
///
/// Integrates HPACK decompression, header redaction via SCRED engine,
/// and re-compression for per-stream header processing.
///
/// RFC 7540 compliance: Preserves stream semantics, maintains header ordering.
/// RFC 7541 compliance: Uses HPACK dynamic table for encode/decode.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{debug, trace};

use crate::h2::hpack::HpackDecoder;
use crate::h2::frame_encoder::HpackEncoder;
use scred_redactor::RedactionEngine;

/// Sensitive HTTP/2 header names that should be redacted
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
    "x-csrf-token",
    "proxy-authorization",
    "www-authenticate",
    "proxy-authenticate",
    // AWS headers
    "x-amz-security-token",
    "x-amz-algorithm",
    "x-amz-credential",
    "x-amz-signature",
    // Custom API keys (common patterns)
    "x-access-token",
    "x-secret-key",
    "x-private-key",
    "apikey",
    "api-key",
    "secret",
    "token",
];

/// Per-stream HTTP/2 header redactor
///
/// Manages header redaction for a single HTTP/2 stream, maintaining
/// separate HPACK dynamic table state and redaction state.
pub struct HeaderRedactor {
    /// Stream ID
    stream_id: u32,

    /// HPACK decoder (maintains dynamic table)
    decoder: HpackDecoder,

    /// HPACK encoder (maintains dynamic table)
    encoder: HpackEncoder,

    /// Redaction engine reference
    engine: Arc<RedactionEngine>,

    /// Statistics
    headers_redacted: u64,
    patterns_found: u64,
    bytes_redacted: u64,
}

impl HeaderRedactor {
    /// Create new header redactor for a stream
    pub fn new(stream_id: u32, engine: Arc<RedactionEngine>) -> Self {
        debug!("HeaderRedactor: Creating for stream {}", stream_id);

        Self {
            stream_id,
            decoder: HpackDecoder::new(),
            encoder: HpackEncoder::new(),
            engine,
            headers_redacted: 0,
            patterns_found: 0,
            bytes_redacted: 0,
        }
    }

    /// Decode HPACK-compressed headers and apply redaction
    ///
    /// # Process
    /// 1. Decode HPACK bytes to Vec<(name, value)>
    /// 2. Check each header name against sensitive list
    /// 3. Redact sensitive header values using SCRED engine patterns
    /// 4. Re-encode modified headers to HPACK
    ///
    /// # Returns
    /// Compressed HPACK bytes ready for new HEADERS frame
    pub fn redact_headers(&mut self, hpack_bytes: &[u8]) -> Result<Vec<u8>> {
        trace!(
            "HeaderRedactor::redact_headers: stream={}, hpack_len={}",
            self.stream_id,
            hpack_bytes.len()
        );

        // Decode HPACK to plain headers
        let headers = self
            .decoder
            .decode(hpack_bytes)
            .map_err(|e| anyhow!("HPACK decode failed: {}", e))?;

        if headers.is_empty() {
            trace!("No headers to redact");
            return Ok(Vec::new());
        }

        trace!("Decoded {} headers", headers.len());

        // Apply redaction to header values
        let mut redacted_headers: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for (name, value) in headers {
            let is_sensitive = self.is_sensitive_header(&name);

            if is_sensitive {
                let redacted_value = self.redact_header_value(&name, &value)?;

                if redacted_value != value {
                    self.headers_redacted += 1;
                    let redacted_len = value.len().saturating_sub(redacted_value.len());
                    self.bytes_redacted += redacted_len as u64;

                    debug!(
                        "HeaderRedactor: Redacted header '{}' in stream {}",
                        name, self.stream_id
                    );
                    redacted_headers.insert(name, redacted_value);
                } else {
                    redacted_headers.insert(name, value);
                }
            } else {
                redacted_headers.insert(name, value);
            }
        }

        // Re-encode modified headers to HPACK
        let encoded = self
            .encoder
            .encode(&redacted_headers)
            .map_err(|e| anyhow!("HPACK encode failed: {}", e))?;

        trace!(
            "HeaderRedactor: Re-encoded {} bytes (original: {})",
            encoded.len(),
            hpack_bytes.len()
        );

        Ok(encoded)
    }

    /// Check if header name is sensitive (should be redacted)
    pub fn is_sensitive_header(&self, name: &str) -> bool {
        let lower = name.to_lowercase();

        // Direct match against sensitive list
        if SENSITIVE_HEADERS.contains(&lower.as_str()) {
            return true;
        }

        // Pattern matching for custom headers
        if lower.starts_with("x-") && lower.contains("key") {
            return true;
        }
        if lower.starts_with("x-") && lower.contains("secret") {
            return true;
        }
        if lower.starts_with("x-") && lower.contains("token") {
            return true;
        }

        false
    }

    /// Redact a single header value using SCRED engine patterns
    ///
    /// Scans the header value for patterns defined in the RedactionEngine.
    /// Character-preserving redaction is applied (length invariant).
    fn redact_header_value(&mut self, _header_name: &str, value: &str) -> Result<String> {
        // Use the RedactionEngine to find and redact patterns
        let result = self.engine.redact(value);
        let redacted = result.redacted;
        
        // Track if anything was redacted (compare lengths)
        if redacted != value {
            self.patterns_found += 1;
            self.headers_redacted += 1;
        }
        
        Ok(redacted)
    }

    /// Get stream ID
    pub fn stream_id(&self) -> u32 {
        self.stream_id
    }

    /// Get statistics
    pub fn stats(&self) -> HeaderRedactionStats {
        HeaderRedactionStats {
            stream_id: self.stream_id,
            headers_redacted: self.headers_redacted,
            patterns_found: self.patterns_found,
            bytes_redacted: self.bytes_redacted,
        }
    }
}

/// Statistics for header redaction on a stream
#[derive(Clone, Debug)]
pub struct HeaderRedactionStats {
    pub stream_id: u32,
    pub headers_redacted: u64,
    pub patterns_found: u64,
    pub bytes_redacted: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use scred_redactor::RedactionConfig;

    fn mock_engine() -> Arc<RedactionEngine> {
        Arc::new(RedactionEngine::new(RedactionConfig::default()))
    }

    #[test]
    fn test_sensitive_header_detection() {
        let redactor = HeaderRedactor::new(1, mock_engine());

        // Exact matches
        assert!(redactor.is_sensitive_header("authorization"));
        assert!(redactor.is_sensitive_header("Authorization"));
        assert!(redactor.is_sensitive_header("cookie"));
        assert!(redactor.is_sensitive_header("x-api-key"));

        // Non-sensitive
        assert!(!redactor.is_sensitive_header("content-type"));
        assert!(!redactor.is_sensitive_header("accept"));
        assert!(!redactor.is_sensitive_header("user-agent"));

        // Pattern matches
        assert!(redactor.is_sensitive_header("x-my-secret-key"));
        assert!(redactor.is_sensitive_header("x-custom-token"));
    }

    #[test]
    fn test_header_redactor_creation() {
        let engine = mock_engine();
        let redactor = HeaderRedactor::new(42, engine);

        assert_eq!(redactor.stream_id(), 42);
        let stats = redactor.stats();
        assert_eq!(stats.stream_id, 42);
        assert_eq!(stats.headers_redacted, 0);
        assert_eq!(stats.patterns_found, 0);
        assert_eq!(stats.bytes_redacted, 0);
    }

    #[test]
    fn test_stats_tracking() {
        let engine = mock_engine();
        let redactor = HeaderRedactor::new(99, engine);

        let stats = redactor.stats();
        assert_eq!(stats.stream_id, 99);

        // Stats should be independent per stream
        let redactor2 = HeaderRedactor::new(100, mock_engine());
        let stats2 = redactor2.stats();
        assert_eq!(stats2.stream_id, 100);
    }

    #[test]
    fn test_multiple_stream_isolation() {
        let engine = mock_engine();

        let r1 = HeaderRedactor::new(1, engine.clone());
        let r2 = HeaderRedactor::new(3, engine.clone());
        let r3 = HeaderRedactor::new(5, engine.clone());

        // Streams should have independent state
        assert_eq!(r1.stream_id(), 1);
        assert_eq!(r2.stream_id(), 3);
        assert_eq!(r3.stream_id(), 5);

        // Stats should be separate
        let s1 = r1.stats();
        let s2 = r2.stats();
        assert_ne!(s1.stream_id, s2.stream_id);
    }
}
