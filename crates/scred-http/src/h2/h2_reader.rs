/// HTTP/2 Response Reader
///
/// Wraps http2 crate to provide high-level interface for reading HTTP/2
/// responses and converting them to HTTP/1.1 format.
///
/// This module simplifies HTTP/2 protocol handling by leveraging the
/// proven http2 crate (RFC 7540/7541 compliant) instead of manual
/// frame parsing and HPACK decompression.

use anyhow::{anyhow, Result};
use crate::http_headers::HttpHeaders;

/// Helper to work with HTTP/2 responses
pub struct H2ResponseReader {
    headers_complete: bool,
    status_code: Option<u16>,
}

impl H2ResponseReader {
    /// Create new HTTP/2 response reader
    pub fn new() -> Self {
        Self {
            headers_complete: false,
            status_code: None,
        }
    }

    /// Check if headers are complete
    pub fn is_headers_complete(&self) -> bool {
        self.headers_complete
    }

    /// Mark headers as complete
    pub fn set_headers_complete(&mut self) {
        self.headers_complete = true;
    }

    /// Set response status code
    pub fn set_status_code(&mut self, code: u16) {
        self.status_code = Some(code);
    }

    /// Get status code
    pub fn status_code(&self) -> Option<u16> {
        self.status_code
    }
}

impl Default for H2ResponseReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to convert HTTP/2 response parts to HTTP/1.1 format
pub struct H2ResponseConverter;

impl H2ResponseConverter {
    /// Convert HTTP/2 headers to HTTP/1.1 format
    ///
    /// Handles:
    /// - Status line (:status → HTTP/1.1 XXX Status)
    /// - Header case conversion (h2 uses lowercase, http/1.1 is case-insensitive)
    /// - Pseudo-header removal (:status, :method, etc.)
    pub fn headers_to_http11(
        status_code: u16,
        headers: &http::HeaderMap,
    ) -> Result<String> {
        let status_text = status_text(status_code);
        let mut result = format!("HTTP/1.1 {} {}\r\n", status_code, status_text);

        // Add regular headers (skip pseudo-headers)
        for (name, value) in headers.iter() {
            let name_str = name.as_str();
            if !name_str.starts_with(':') {
                if let Ok(value_str) = value.to_str() {
                    // HTTP/1.1 uses Title-Case convention
                    let header_name = title_case(name_str);
                    result.push_str(&format!("{}: {}\r\n", header_name, value_str));
                }
            }
        }

        result.push_str("\r\n");
        Ok(result)
    }
}

/// Get HTTP status text for status code
fn status_text(code: u16) -> &'static str {
    match code {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

/// Convert header name to Title-Case (HTTP/1.1 convention)
fn title_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h2_response_reader_creation() {
        let reader = H2ResponseReader::new();
        assert!(!reader.is_headers_complete());
        assert_eq!(reader.status_code(), None);
    }

    #[test]
    fn test_h2_response_reader_default() {
        let reader = H2ResponseReader::default();
        assert!(!reader.is_headers_complete());
    }

    #[test]
    fn test_set_status_code() {
        let mut reader = H2ResponseReader::new();
        reader.set_status_code(200);
        assert_eq!(reader.status_code(), Some(200));
    }

    #[test]
    fn test_set_headers_complete() {
        let mut reader = H2ResponseReader::new();
        assert!(!reader.is_headers_complete());
        reader.set_headers_complete();
        assert!(reader.is_headers_complete());
    }

    #[test]
    fn test_status_text() {
        assert_eq!(status_text(200), "OK");
        assert_eq!(status_text(404), "Not Found");
        assert_eq!(status_text(500), "Internal Server Error");
        assert_eq!(status_text(999), "Unknown");
    }

    #[test]
    fn test_title_case() {
        assert_eq!(title_case("content-type"), "Content-Type");
        assert_eq!(title_case("content-length"), "Content-Length");
        assert_eq!(title_case("x-custom-header"), "X-Custom-Header");
        assert_eq!(title_case("simple"), "Simple");
    }

    #[test]
    fn test_title_case_edge_cases() {
        assert_eq!(title_case(""), "");
        assert_eq!(title_case("-"), "-");
        assert_eq!(title_case("a"), "A");
    }
}
