//! Core HTTP redactor trait and types
//!
//! Defines the interface for redacting HTTP requests and responses.

use crate::models::RedactionStats;
use anyhow::Result;

/// Represents an HTTP request or response
#[derive(Debug, Clone)]
pub struct HttpMessage {
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Trait for redacting HTTP messages
pub trait HttpRedactor {
    /// Redact an HTTP request
    fn redact_request(&self, request: &mut HttpMessage) -> Result<RedactionStats>;

    /// Redact an HTTP response
    fn redact_response(&self, response: &mut HttpMessage) -> Result<RedactionStats>;

    /// Redact just the headers
    fn redact_headers(&self, headers: &mut [(String, String)]) -> Result<RedactionStats>;

    /// Redact just the body
    fn redact_body(&self, body: &mut Vec<u8>) -> Result<RedactionStats>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_message_creation() {
        let msg = HttpMessage {
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: vec![1, 2, 3],
        };

        assert_eq!(msg.headers.len(), 1);
        assert_eq!(msg.body.len(), 3);
    }
}
