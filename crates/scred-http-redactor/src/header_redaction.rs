//! HTTP header redaction
//!
//! Handles sensitive header detection and redaction.

use crate::models::RedactionStats;
use anyhow::Result;
use scred_http_detector::classification::{classify_header, Sensitivity};

/// Redactor for HTTP headers
pub struct HeaderRedactor;

impl HeaderRedactor {
    pub fn new() -> Self {
        Self
    }

    /// Redact headers in-place
    pub fn redact_headers(&self, headers: &mut [(String, String)]) -> Result<RedactionStats> {
        let mut stats = RedactionStats::new();

        for (name, value) in headers.iter_mut() {
            let sensitivity = classify_header(name, value);
            if sensitivity > Sensitivity::Public {
                stats.headers_redacted += 1;
                stats.bytes_processed += value.len() as u64;

                *value = self.mask_value(value, sensitivity);
                stats.bytes_redacted += value.len() as u64;
            }
        }

        Ok(stats)
    }

    /// Redact a single header value
    pub fn redact_header(&self, name: &str, value: &str) -> Option<String> {
        let sensitivity = classify_header(name, value);
        if sensitivity > Sensitivity::Public {
            Some(self.mask_value(value, sensitivity))
        } else {
            None
        }
    }

    /// Mask value based on sensitivity
    fn mask_value(&self, value: &str, sensitivity: Sensitivity) -> String {
        match sensitivity {
            Sensitivity::Public => value.to_string(),
            Sensitivity::Internal => "[***]".to_string(),
            Sensitivity::Confidential => "[REDACTED]".to_string(),
            Sensitivity::Secret => "[CLASSIFIED]".to_string(),
        }
    }
}

impl Default for HeaderRedactor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_headers_auth() {
        let redactor = HeaderRedactor::new();
        let mut headers = vec![
            ("Host".to_string(), "example.com".to_string()),
            ("Authorization".to_string(), "Bearer token123".to_string()),
            ("User-Agent".to_string(), "curl".to_string()),
        ];

        let stats = redactor.redact_headers(&mut headers).unwrap();

        assert_eq!(stats.headers_redacted, 1);
        assert_eq!(headers[1].1, "[CLASSIFIED]");
    }

    #[test]
    fn test_redact_headers_cookie() {
        let redactor = HeaderRedactor::new();
        let mut headers = vec![
            ("Cookie".to_string(), "session=abc123".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];

        let stats = redactor.redact_headers(&mut headers).unwrap();

        assert_eq!(stats.headers_redacted, 1);
        assert_eq!(headers[0].1, "[REDACTED]");
        assert_eq!(headers[1].1, "application/json"); // Unchanged
    }

    #[test]
    fn test_redact_header_single() {
        let redactor = HeaderRedactor::new();

        let result = redactor.redact_header("Authorization", "Bearer xyz");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "[CLASSIFIED]");

        let result = redactor.redact_header("Accept", "application/json");
        assert!(result.is_none());
    }
}
