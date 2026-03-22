//! HTTP Header Rewriting Utilities
//!
//! Provides functions for manipulating HTTP headers: extracting values, replacing headers,
//! and injecting missing headers. Used for proxy header normalization.
//!
//! # Examples
//!
//! ```ignore
//! let mut headers = "Host: localhost:9999\r\nUser-Agent: curl\r\n";
//! replace_header_value(&mut headers, "Host", "httpbin.org");
//! assert_eq!(extract_header_value(&headers, "Host"), Some("httpbin.org".to_string()));
//! ```

/// Extract HTTP header value by name (case-insensitive)
///
/// Parses HTTP header lines and returns the value of the first matching header.
/// Header names are matched case-insensitively per RFC 7230.
///
/// # Arguments
/// * `headers` - HTTP headers as text (CRLF-separated lines)
/// * `name` - Header name to extract (case-insensitive)
///
/// # Returns
/// * `Some(value)` - Header value with leading/trailing whitespace trimmed
/// * `None` - If header not found
///
/// # Examples
/// ```ignore
/// let headers = "Host: example.com\r\nContent-Length: 100\r\n";
/// assert_eq!(extract_header_value(headers, "Host"), Some("example.com".to_string()));
/// assert_eq!(extract_header_value(headers, "host"), Some("example.com".to_string()));
/// assert_eq!(extract_header_value(headers, "Missing"), None);
/// ```
pub fn extract_header_value(headers: &str, name: &str) -> Option<String> {
    let name_lower = format!("{}:", name.to_lowercase());
    for line in headers.lines() {
        if line.to_lowercase().starts_with(&name_lower) {
            // Split on first colon only, to preserve colons in value
            if let Some(colon_pos) = line.find(':') {
                let value = &line[colon_pos + 1..];
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Replace HTTP header value or do nothing if header missing
///
/// Finds a header by name (case-insensitive) and replaces its value.
/// Preserves CRLF line endings. If header is not found, headers are unchanged.
///
/// # Arguments
/// * `headers` - Mutable reference to HTTP headers text
/// * `name` - Header name to replace (case-insensitive)
/// * `value` - New header value
///
/// # Behavior
/// - Finds first matching header (case-insensitive)
/// - Replaces entire header value
/// - Preserves CRLF endings
/// - If header missing: no changes (caller should use `inject_header_if_missing`)
///
/// # Examples
/// ```ignore
/// let mut headers = "Host: localhost:9999\r\nUser-Agent: curl\r\n".to_string();
/// replace_header_value(&mut headers, "Host", "httpbin.org");
/// assert_eq!(extract_header_value(&headers, "Host"), Some("httpbin.org".to_string()));
///
/// // If header missing, nothing happens
/// let mut headers = "Content-Length: 100\r\n".to_string();
/// replace_header_value(&mut headers, "Host", "example.com");
/// assert_eq!(extract_header_value(&headers, "Host"), None);
/// ```
pub fn replace_header_value(headers: &mut String, name: &str, value: &str) {
    let name_lower = name.to_lowercase();
    let mut lines: Vec<String> = headers.split("\r\n").map(|s| s.to_string()).collect();
    let mut found = false;

    for line in &mut lines {
        if !found && !line.is_empty() && line.to_lowercase().starts_with(&format!("{}:", name_lower)) {
            // Replace the header value
            *line = format!("{}: {}", name, value);
            found = true;
        }
    }

    if found {
        *headers = lines.join("\r\n");
    }
}

/// Inject header if missing, do nothing if header already present
///
/// Checks if a header exists. If missing, injects it before the blank line
/// that separates headers from body. If header already present, does nothing.
///
/// # Arguments
/// * `headers` - Mutable reference to HTTP headers text
/// * `name` - Header name to inject (case-insensitive check)
/// * `value` - Header value
///
/// # Behavior
/// - Checks if header already present (case-insensitive)
/// - If missing: injects `{name}: {value}\r\n` before blank line
/// - If present: no changes
/// - Preserves CRLF endings
///
/// # Examples
/// ```ignore
/// let mut headers = "User-Agent: curl\r\n".to_string();
/// inject_header_if_missing(&mut headers, "Host", "httpbin.org");
/// assert_eq!(extract_header_value(&headers, "Host"), Some("httpbin.org".to_string()));
///
/// // If already present, nothing happens
/// let mut headers = "Host: localhost\r\nUser-Agent: curl\r\n".to_string();
/// inject_header_if_missing(&mut headers, "Host", "httpbin.org");
/// assert_eq!(extract_header_value(&headers, "Host"), Some("localhost".to_string()));
/// ```
pub fn inject_header_if_missing(headers: &mut String, name: &str, value: &str) {
    // Check if header already present (case-insensitive)
    if extract_header_value(headers, name).is_some() {
        return; // Header already present, do nothing
    }

    // Find blank line that marks end of headers (\r\n\r\n)
    // We insert after the first \r\n to put it before the blank line
    if let Some(blank_line_pos) = headers.find("\r\n\r\n") {
        let insert_pos = blank_line_pos + 2; // Position after first \r\n
        let header_line = format!("{}: {}\r\n", name, value);
        headers.insert_str(insert_pos, &header_line);
    } else {
        // No blank line found, just append
        if !headers.is_empty() && !headers.ends_with('\n') {
            headers.push_str("\r\n");
        }
        headers.push_str(&format!("{}: {}\r\n\r\n", name, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod extract_header_value {
        use super::*;

        #[test]
        fn test_extract_existing_header() {
            let headers = "Host: example.com\r\nContent-Length: 100\r\n";
            assert_eq!(
                extract_header_value(headers, "Host"),
                Some("example.com".to_string())
            );
        }

        #[test]
        fn test_extract_header_case_insensitive() {
            let headers = "Host: example.com\r\nContent-Length: 100\r\n";
            assert_eq!(
                extract_header_value(headers, "host"),
                Some("example.com".to_string())
            );
            assert_eq!(
                extract_header_value(headers, "HOST"),
                Some("example.com".to_string())
            );
        }

        #[test]
        fn test_extract_missing_header() {
            let headers = "Host: example.com\r\nContent-Length: 100\r\n";
            assert_eq!(extract_header_value(headers, "Missing"), None);
        }

        #[test]
        fn test_extract_with_whitespace() {
            let headers = "Host:   example.com  \r\nContent-Length: 100\r\n";
            assert_eq!(
                extract_header_value(headers, "Host"),
                Some("example.com".to_string())
            );
        }

        #[test]
        fn test_extract_multiple_headers_same_name() {
            // RFC 7230: multiple headers with same name - extract first
            let headers = "Set-Cookie: a=1\r\nSet-Cookie: b=2\r\n";
            assert_eq!(
                extract_header_value(headers, "Set-Cookie"),
                Some("a=1".to_string())
            );
        }
    }

    mod replace_header_value {
        use super::*;

        #[test]
        fn test_replace_existing_header() {
            let mut headers = "Host: localhost:9999\r\nContent-Length: 100\r\n".to_string();
            replace_header_value(&mut headers, "Host", "httpbin.org");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                Some("httpbin.org".to_string())
            );
        }

        #[test]
        fn test_replace_case_insensitive() {
            let mut headers = "Host: localhost\r\nContent-Length: 100\r\n".to_string();
            replace_header_value(&mut headers, "host", "httpbin.org");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                Some("httpbin.org".to_string())
            );
        }

        #[test]
        fn test_replace_missing_header_does_nothing() {
            let original = "Content-Length: 100\r\n".to_string();
            let mut headers = original.clone();
            replace_header_value(&mut headers, "Host", "httpbin.org");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                None,
                "Should not create header"
            );
        }

        #[test]
        fn test_replace_preserves_other_headers() {
            let mut headers =
                "Host: localhost\r\nUser-Agent: curl\r\nContent-Length: 100\r\n".to_string();
            replace_header_value(&mut headers, "Host", "httpbin.org");
            assert_eq!(
                extract_header_value(&headers, "User-Agent"),
                Some("curl".to_string())
            );
            assert_eq!(
                extract_header_value(&headers, "Content-Length"),
                Some("100".to_string())
            );
        }

        #[test]
        fn test_replace_with_port() {
            let mut headers = "Host: localhost:9999\r\n".to_string();
            replace_header_value(&mut headers, "Host", "httpbin.org:443");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                Some("httpbin.org:443".to_string())
            );
        }
    }

    mod inject_header_if_missing {
        use super::*;

        #[test]
        fn test_inject_missing_header() {
            let mut headers = "User-Agent: curl\r\n".to_string();
            inject_header_if_missing(&mut headers, "Host", "httpbin.org");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                Some("httpbin.org".to_string())
            );
        }

        #[test]
        fn test_inject_does_nothing_if_present() {
            let mut headers = "Host: localhost\r\nUser-Agent: curl\r\n".to_string();
            inject_header_if_missing(&mut headers, "Host", "httpbin.org");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                Some("localhost".to_string()),
                "Should preserve existing header"
            );
        }

        #[test]
        fn test_inject_case_insensitive_check() {
            let mut headers = "Host: localhost\r\nUser-Agent: curl\r\n".to_string();
            inject_header_if_missing(&mut headers, "host", "httpbin.org");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                Some("localhost".to_string()),
                "Should recognize existing header (case-insensitive)"
            );
        }

        #[test]
        fn test_inject_before_blank_line() {
            let mut headers = "User-Agent: curl\r\n\r\n".to_string();
            inject_header_if_missing(&mut headers, "Host", "httpbin.org");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                Some("httpbin.org".to_string())
            );
            // Should inject before the blank line
            assert!(headers.contains("Host: httpbin.org\r\n\r\n"));
        }

        #[test]
        fn test_inject_multiple_headers() {
            let mut headers = "User-Agent: curl\r\n".to_string();
            inject_header_if_missing(&mut headers, "Host", "httpbin.org");
            inject_header_if_missing(&mut headers, "Accept", "*/*");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                Some("httpbin.org".to_string())
            );
            assert_eq!(
                extract_header_value(&headers, "Accept"),
                Some("*/*".to_string())
            );
        }

        #[test]
        fn test_inject_empty_headers() {
            let mut headers = String::new();
            inject_header_if_missing(&mut headers, "Host", "httpbin.org");
            assert_eq!(
                extract_header_value(&headers, "Host"),
                Some("httpbin.org".to_string())
            );
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_empty_header_value() {
            let mut headers = "X-Empty: \r\n".to_string();
            let val = extract_header_value(&headers, "X-Empty");
            assert_eq!(val, Some("".to_string()));
        }

        #[test]
        fn test_header_with_colon_in_value() {
            let headers = "Custom-Header: value:with:colons\r\n";
            assert_eq!(
                extract_header_value(headers, "Custom-Header"),
                Some("value:with:colons".to_string())
            );
        }

        #[test]
        fn test_replace_first_occurrence_only() {
            let mut headers = "X-Test: first\r\nX-Other: middle\r\nX-Test: second\r\n".to_string();
            replace_header_value(&mut headers, "X-Test", "replaced");
            // Should only replace the first occurrence
            assert_eq!(
                extract_header_value(&headers, "X-Test"),
                Some("replaced".to_string())
            );
        }
    }
}
