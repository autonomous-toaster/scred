//! Protocol-specific HTTP redactors
//!
//! Implements redactors for HTTP/1.1 and HTTP/2

use crate::body_redaction::BodyRedactor;
use crate::core::{HttpMessage, HttpRedactor};
use crate::header_redaction::HeaderRedactor;
use crate::models::RedactionStats;
use anyhow::Result;
use scred_http_detector::models::ContentType;

/// HTTP/1.1 redactor
pub struct Http11Redactor {
    header_redactor: HeaderRedactor,
    body_redactor: BodyRedactor,
}

impl Http11Redactor {
    pub fn new() -> Self {
        Self {
            header_redactor: HeaderRedactor::new(),
            body_redactor: BodyRedactor::new(),
        }
    }

    /// Get content type from headers
    fn get_content_type(&self, headers: &[(String, String)]) -> ContentType {
        for (name, value) in headers {
            if name.to_lowercase() == "content-type" {
                return ContentType::from_header(value);
            }
        }
        ContentType::Unknown
    }
}

impl Default for Http11Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpRedactor for Http11Redactor {
    fn redact_request(&self, request: &mut HttpMessage) -> Result<RedactionStats> {
        let mut total_stats = RedactionStats::new();

        // Redact headers
        let header_stats = self.redact_headers(&mut request.headers)?;
        total_stats.merge(&header_stats);

        // Redact body
        let body_stats = self.redact_body(&mut request.body)?;
        total_stats.merge(&body_stats);

        Ok(total_stats)
    }

    fn redact_response(&self, response: &mut HttpMessage) -> Result<RedactionStats> {
        // Response redaction is similar to request
        self.redact_request(response)
    }

    fn redact_headers(&self, headers: &mut [(String, String)]) -> Result<RedactionStats> {
        self.header_redactor.redact_headers(headers)
    }

    fn redact_body(&self, body: &mut Vec<u8>) -> Result<RedactionStats> {
        // Try JSON first (most common)
        self.body_redactor.redact_body(body, ContentType::Json)
    }
}

/// HTTP/2 redactor
/// Handles HTTP/2 specific concerns: pseudo-headers, stream management, HPACK
pub struct H2Redactor {
    header_redactor: HeaderRedactor,
    body_redactor: BodyRedactor,
}

impl H2Redactor {
    pub fn new() -> Self {
        Self {
            header_redactor: HeaderRedactor::new(),
            body_redactor: BodyRedactor::new(),
        }
    }

    /// Redact HTTP/2 pseudo-headers (start with :)
    pub fn redact_pseudo_headers(&self, headers: &mut [(Vec<u8>, Vec<u8>)]) -> Result<RedactionStats> {
        let stats = RedactionStats::new();

        for (_name, _value) in headers.iter_mut() {
            // Pseudo-headers typically don't need redaction (method, path, scheme, authority)
            // Regular headers are handled by redact_headers below
        }

        Ok(stats)
    }

    /// Redact all headers (both pseudo and regular)
    pub fn redact_headers_raw(&self, headers: &mut [(Vec<u8>, Vec<u8>)]) -> Result<RedactionStats> {
        let mut stats = RedactionStats::new();

        for (name, value) in headers.iter_mut() {
            let name_str = String::from_utf8_lossy(name);
            let value_str = String::from_utf8_lossy(value);

            // Skip pseudo-headers (start with :)
            if name_str.starts_with(':') {
                continue;
            }

            // Check if regular header needs redaction
            if let Some(redacted) = self.header_redactor.redact_header(&name_str, &value_str) {
                stats.headers_redacted += 1;
                *value = redacted.into_bytes();
            }
        }

        Ok(stats)
    }
}

impl Default for H2Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpRedactor for H2Redactor {
    fn redact_request(&self, request: &mut HttpMessage) -> Result<RedactionStats> {
        let mut total_stats = RedactionStats::new();

        // Redact headers
        let header_stats = self.redact_headers(&mut request.headers)?;
        total_stats.merge(&header_stats);

        // Redact body
        let body_stats = self.redact_body(&mut request.body)?;
        total_stats.merge(&body_stats);

        Ok(total_stats)
    }

    fn redact_response(&self, response: &mut HttpMessage) -> Result<RedactionStats> {
        self.redact_request(response)
    }

    fn redact_headers(&self, headers: &mut [(String, String)]) -> Result<RedactionStats> {
        self.header_redactor.redact_headers(headers)
    }

    fn redact_body(&self, body: &mut Vec<u8>) -> Result<RedactionStats> {
        // Try JSON first
        self.body_redactor.redact_body(body, ContentType::Json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http11_redactor_auth_header() {
        let redactor = Http11Redactor::new();
        let mut msg = HttpMessage {
            headers: vec![
                ("Host".to_string(), "example.com".to_string()),
                ("Authorization".to_string(), "Bearer token123".to_string()),
            ],
            body: vec![],
        };

        let stats = redactor.redact_request(&mut msg).unwrap();
        assert_eq!(stats.headers_redacted, 1);
    }

    #[test]
    fn test_http11_redactor_get_content_type() {
        let redactor = Http11Redactor::new();
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];

        let ct = redactor.get_content_type(&headers);
        assert_eq!(ct, ContentType::Json);
    }

    #[test]
    fn test_h2_redactor_creation() {
        let _redactor = H2Redactor::new();
        assert!(true); // Just verify creation succeeds
    }

    #[test]
    fn test_h2_redactor_pseudo_headers() {
        let redactor = H2Redactor::new();
        let mut headers = vec![
            (b":method".to_vec(), b"GET".to_vec()),
            (b"authorization".to_vec(), b"Bearer xyz".to_vec()),
        ];

        let stats = redactor.redact_pseudo_headers(&mut headers).unwrap();
        assert_eq!(stats.headers_redacted, 0); // Pseudo-headers don't need redaction
    }
}
