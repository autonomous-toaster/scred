//! HTTP/2 Header Validation & Edge Cases
//! 
//! RFC 7540 Section 4.3 & 5.1: Header block validation and size limits

use std::collections::HashMap;

/// Maximum header block size (RFC 7540 default: 4096 bytes)
pub const MAX_HEADER_BLOCK_SIZE: usize = 4096;

/// Maximum single header field size (commonly 8KB)
pub const MAX_HEADER_FIELD_SIZE: usize = 8192;

/// Header validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderValidationResult {
    /// Is valid
    pub valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Total size in bytes
    pub total_size: usize,
    /// Field count
    pub field_count: usize,
}

impl HeaderValidationResult {
    /// Create a valid result
    pub fn valid(total_size: usize, field_count: usize) -> Self {
        HeaderValidationResult {
            valid: true,
            error: None,
            total_size,
            field_count,
        }
    }

    /// Create an invalid result
    pub fn invalid(error: String) -> Self {
        HeaderValidationResult {
            valid: false,
            error: Some(error),
            total_size: 0,
            field_count: 0,
        }
    }
}

/// HTTP/2 pseudo-headers
/// RFC 7540 Section 8.3: HTTP Request Pseudo-Header Fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoHeader {
    /// :method
    Method,
    /// :scheme
    Scheme,
    /// :authority
    Authority,
    /// :path
    Path,
    /// :status (response only)
    Status,
}

impl PseudoHeader {
    /// Get pseudo-header name
    pub fn name(&self) -> &'static str {
        match self {
            PseudoHeader::Method => ":method",
            PseudoHeader::Scheme => ":scheme",
            PseudoHeader::Authority => ":authority",
            PseudoHeader::Path => ":path",
            PseudoHeader::Status => ":status",
        }
    }

    /// Check if pseudo-header is request-only
    pub fn is_request_only(&self) -> bool {
        matches!(
            self,
            PseudoHeader::Method | PseudoHeader::Scheme | PseudoHeader::Authority | PseudoHeader::Path
        )
    }

    /// Check if pseudo-header is response-only
    pub fn is_response_only(&self) -> bool {
        matches!(self, PseudoHeader::Status)
    }
}

/// Header block validator
pub struct HeaderBlockValidator;

impl HeaderBlockValidator {
    /// Validate header block size
    /// 
    /// RFC 7540 Section 4.3: The SETTINGS_MAX_HEADER_LIST_SIZE setting
    pub fn validate_block_size(total_size: usize) -> Result<(), String> {
        if total_size > MAX_HEADER_BLOCK_SIZE {
            return Err(format!(
                "Header block too large: {} bytes (max: {})",
                total_size, MAX_HEADER_BLOCK_SIZE
            ));
        }

        Ok(())
    }

    /// Validate individual header field
    pub fn validate_field(name: &str, value: &str) -> Result<(), String> {
        // Field size check
        let field_size = name.len() + value.len() + 32; // 32 bytes overhead per RFC 7541
        if field_size > MAX_HEADER_FIELD_SIZE {
            return Err(format!(
                "Header field too large: {} bytes (max: {})",
                field_size, MAX_HEADER_FIELD_SIZE
            ));
        }

        // Pseudo-header validation (must start with ':')
        if name.starts_with(':') {
            if !Self::is_valid_pseudo_header(name) {
                return Err(format!("Invalid pseudo-header: {}", name));
            }
        } else {
            // Regular header field must not contain uppercase letters
            if name.chars().any(|c| c.is_uppercase()) {
                return Err(format!("Header name contains uppercase: {}", name));
            }
        }

        // Empty header name not allowed
        if name.is_empty() {
            return Err("Header name cannot be empty".to_string());
        }

        Ok(())
    }

    /// Check if pseudo-header is valid
    fn is_valid_pseudo_header(name: &str) -> bool {
        matches!(
            name,
            ":method" | ":scheme" | ":authority" | ":path" | ":status"
        )
    }

    /// Validate header list (collection of headers)
    pub fn validate_header_list(headers: &[(String, String)]) -> HeaderValidationResult {
        let mut total_size = 0;

        // Track pseudo-headers
        let mut pseudo_headers = HashMap::new();

        for (name, value) in headers {
            // Validate individual field
            if let Err(e) = Self::validate_field(name, value) {
                return HeaderValidationResult::invalid(e);
            }

            // Track size
            total_size += name.len() + value.len() + 32;

            // Check total size
            if let Err(e) = Self::validate_block_size(total_size) {
                return HeaderValidationResult::invalid(e);
            }

            // Track pseudo-headers
            if name.starts_with(':') {
                *pseudo_headers.entry(name.clone()).or_insert(0) += 1;
            }
        }

        // Check for duplicate pseudo-headers
        for (name, count) in &pseudo_headers {
            if *count > 1 {
                return HeaderValidationResult::invalid(format!(
                    "Duplicate pseudo-header: {} ({} occurrences)",
                    name, count
                ));
            }
        }

        HeaderValidationResult::valid(total_size, headers.len())
    }

    /// Validate request headers (CONNECT, GET, POST, etc.)
    pub fn validate_request_headers(headers: &[(String, String)]) -> Result<(), String> {
        let mut has_method = false;
        let mut has_scheme = false;
        let mut has_path = false;

        for (name, value) in headers {
            match name.as_str() {
                ":method" => {
                    if has_method {
                        return Err("Duplicate :method pseudo-header".to_string());
                    }
                    has_method = true;

                    // Validate method
                    match value.as_str() {
                        "GET" | "HEAD" | "POST" | "PUT" | "DELETE" | "CONNECT" | "OPTIONS"
                        | "PATCH" => {},
                        _ => {
                            return Err(format!("Invalid HTTP method: {}", value));
                        }
                    }
                }
                ":scheme" => {
                    if has_scheme {
                        return Err("Duplicate :scheme pseudo-header".to_string());
                    }
                    has_scheme = true;

                    // Validate scheme
                    match value.as_str() {
                        "http" | "https" => {},
                        _ => {
                            return Err(format!("Invalid scheme: {}", value));
                        }
                    }
                }
                ":path" => {
                    if has_path {
                        return Err("Duplicate :path pseudo-header".to_string());
                    }
                    has_path = true;

                    // Path must not be empty
                    if value.is_empty() {
                        return Err(":path cannot be empty".to_string());
                    }
                }
                ":authority" | _ => {}
            }
        }

        // CONNECT method doesn't require :scheme and :path
        let method = headers
            .iter()
            .find(|(n, _)| n == ":method")
            .map(|(_, v)| v.as_str());

        if method != Some("CONNECT") {
            if !has_method {
                return Err("Missing required :method pseudo-header".to_string());
            }
            if !has_scheme {
                return Err("Missing required :scheme pseudo-header".to_string());
            }
            if !has_path {
                return Err("Missing required :path pseudo-header".to_string());
            }
        }

        Ok(())
    }

    /// Validate response headers
    pub fn validate_response_headers(headers: &[(String, String)]) -> Result<(), String> {
        let mut has_status = false;

        for (name, value) in headers {
            if name == ":status" {
                if has_status {
                    return Err("Duplicate :status pseudo-header".to_string());
                }
                has_status = true;

                // Validate status code (3 digits)
                if value.len() != 3 || !value.chars().all(|c| c.is_numeric()) {
                    return Err(format!("Invalid status code: {}", value));
                }
            } else if name.starts_with(':') {
                return Err(format!(
                    "Invalid response pseudo-header: {} (request-only)",
                    name
                ));
            }
        }

        if !has_status {
            return Err("Missing required :status pseudo-header".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_field_basic() {
        assert!(HeaderBlockValidator::validate_field("content-type", "text/html").is_ok());
        assert!(HeaderBlockValidator::validate_field("x-custom-header", "value").is_ok());
    }

    #[test]
    fn test_validate_field_empty_name() {
        let result = HeaderBlockValidator::validate_field("", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_field_uppercase() {
        let result = HeaderBlockValidator::validate_field("Content-Type", "text/html");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_field_pseudo_header_valid() {
        assert!(HeaderBlockValidator::validate_field(":method", "GET").is_ok());
        assert!(HeaderBlockValidator::validate_field(":scheme", "https").is_ok());
        assert!(HeaderBlockValidator::validate_field(":path", "/").is_ok());
    }

    #[test]
    fn test_validate_field_pseudo_header_invalid() {
        let result = HeaderBlockValidator::validate_field(":invalid", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_block_size_valid() {
        assert!(HeaderBlockValidator::validate_block_size(1000).is_ok());
        assert!(HeaderBlockValidator::validate_block_size(MAX_HEADER_BLOCK_SIZE).is_ok());
    }

    #[test]
    fn test_validate_block_size_too_large() {
        let result = HeaderBlockValidator::validate_block_size(MAX_HEADER_BLOCK_SIZE + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_header_list_valid() {
        let headers = vec![
            (":method".to_string(), "GET".to_string()),
            (":scheme".to_string(), "https".to_string()),
            (":path".to_string(), "/".to_string()),
            ("content-type".to_string(), "text/html".to_string()),
        ];

        let result = HeaderBlockValidator::validate_header_list(&headers);
        assert!(result.valid);
        assert_eq!(result.field_count, 4);
    }

    #[test]
    fn test_validate_header_list_duplicate_pseudo() {
        let headers = vec![
            (":method".to_string(), "GET".to_string()),
            (":method".to_string(), "POST".to_string()),
        ];

        let result = HeaderBlockValidator::validate_header_list(&headers);
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_request_headers_valid_get() {
        let headers = vec![
            (":method".to_string(), "GET".to_string()),
            (":scheme".to_string(), "https".to_string()),
            (":path".to_string(), "/index.html".to_string()),
            (":authority".to_string(), "example.com".to_string()),
        ];

        assert!(HeaderBlockValidator::validate_request_headers(&headers).is_ok());
    }

    #[test]
    fn test_validate_request_headers_valid_post() {
        let headers = vec![
            (":method".to_string(), "POST".to_string()),
            (":scheme".to_string(), "https".to_string()),
            (":path".to_string(), "/api/data".to_string()),
        ];

        assert!(HeaderBlockValidator::validate_request_headers(&headers).is_ok());
    }

    #[test]
    fn test_validate_request_headers_valid_connect() {
        let headers = vec![(":method".to_string(), "CONNECT".to_string())];

        assert!(HeaderBlockValidator::validate_request_headers(&headers).is_ok());
    }

    #[test]
    fn test_validate_request_headers_missing_method() {
        let headers = vec![
            (":scheme".to_string(), "https".to_string()),
            (":path".to_string(), "/".to_string()),
        ];

        assert!(HeaderBlockValidator::validate_request_headers(&headers).is_err());
    }

    #[test]
    fn test_validate_request_headers_invalid_method() {
        let headers = vec![
            (":method".to_string(), "INVALID".to_string()),
            (":scheme".to_string(), "https".to_string()),
            (":path".to_string(), "/".to_string()),
        ];

        assert!(HeaderBlockValidator::validate_request_headers(&headers).is_err());
    }

    #[test]
    fn test_validate_request_headers_empty_path() {
        let headers = vec![
            (":method".to_string(), "GET".to_string()),
            (":scheme".to_string(), "https".to_string()),
            (":path".to_string(), "".to_string()),
        ];

        assert!(HeaderBlockValidator::validate_request_headers(&headers).is_err());
    }

    #[test]
    fn test_validate_response_headers_valid() {
        let headers = vec![
            (":status".to_string(), "200".to_string()),
            ("content-type".to_string(), "text/html".to_string()),
        ];

        assert!(HeaderBlockValidator::validate_response_headers(&headers).is_ok());
    }

    #[test]
    fn test_validate_response_headers_missing_status() {
        let headers = vec![("content-type".to_string(), "text/html".to_string())];

        assert!(HeaderBlockValidator::validate_response_headers(&headers).is_err());
    }

    #[test]
    fn test_validate_response_headers_invalid_status() {
        let headers = vec![(":status".to_string(), "OK".to_string())];

        assert!(HeaderBlockValidator::validate_response_headers(&headers).is_err());
    }

    #[test]
    fn test_validate_response_headers_request_pseudo() {
        let headers = vec![
            (":status".to_string(), "200".to_string()),
            (":method".to_string(), "GET".to_string()),
        ];

        assert!(HeaderBlockValidator::validate_response_headers(&headers).is_err());
    }

    #[test]
    fn test_pseudo_header_names() {
        assert_eq!(PseudoHeader::Method.name(), ":method");
        assert_eq!(PseudoHeader::Scheme.name(), ":scheme");
        assert_eq!(PseudoHeader::Status.name(), ":status");
    }

    #[test]
    fn test_pseudo_header_request_only() {
        assert!(PseudoHeader::Method.is_request_only());
        assert!(PseudoHeader::Scheme.is_request_only());
        assert!(!PseudoHeader::Status.is_request_only());
    }

    #[test]
    fn test_pseudo_header_response_only() {
        assert!(PseudoHeader::Status.is_response_only());
        assert!(!PseudoHeader::Method.is_response_only());
    }
}
