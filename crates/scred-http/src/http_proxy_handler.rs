/// Shared HTTP Proxy Handler for Forward and MITM Proxies
///
/// This module handles HTTP proxy requests (non-CONNECT) with:
/// - Request parsing (method, URI, headers, body)
/// - Secret redaction (via external redaction engine)
/// - Upstream forwarding
/// - Response redaction
/// - Standard proxy headers (Via, X-SCRED-Redacted)
///
/// Used by:
/// - scred-mitm: MITM proxy with TLS interception
/// - scred-proxy: Forward proxy with fixed upstream

use anyhow::{anyhow, Result};
use scred_redactor::RedactionEngine;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, info, warn, error};
use crate::dns_resolver::DnsResolver;
use crate::proxy_resolver::connect_through_proxy;
use crate::header_rewriter;
use crate::location_rewriter;

/// Configuration for HTTP proxy forwarding
#[derive(Clone, Debug)]
pub struct HttpProxyConfig {
    /// Add Via header (RFC 7230)
    pub add_via_header: bool,
    /// Add X-SCRED-Redacted header
    pub add_scred_header: bool,
}

impl Default for HttpProxyConfig {
    fn default() -> Self {
        Self {
            add_via_header: true,
            add_scred_header: true,
        }
    }
}

/// Handle HTTP proxy request (non-CONNECT)
///
/// # Arguments
/// * `client_read` - Read half of client connection
/// * `client_write` - Write half of client connection
/// * `first_line` - Initial HTTP request line
/// * `redaction_engine` - Engine for redacting secrets
/// * `detect_selector` - Optional selector for secret detection
/// * `redact_selector` - Optional selector for secret redaction
/// * `upstream_addr` - Upstream server address (host:port)
/// * `upstream_host` - Optional upstream hostname (for header rewriting)
/// * `config` - Proxy configuration (headers, options)
pub async fn handle_http_proxy(
    mut client_read: tokio::net::tcp::OwnedReadHalf,
    mut client_write: tokio::net::tcp::OwnedWriteHalf,
    first_line: &str,
    redaction_engine: Arc<RedactionEngine>,
    detect_selector: Option<scred_redactor::PatternSelector>,
    redact_selector: Option<scred_redactor::PatternSelector>,
    upstream_addr: &str,
    upstream_host: Option<&str>,
    config: HttpProxyConfig,
) -> Result<()> {
    debug!("HTTP proxy request: {}", first_line);

    // Parse HTTP request line: METHOD URL HTTP/VERSION
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 3 {
        send_error_response(&mut client_write, 400, "Bad Request").await?;
        return Err(anyhow!("Invalid HTTP request line"));
    }

    let method = parts[0];
    let url = parts[1];

    // Parse URL to extract host and path
    let (target_host, target_port, path) = parse_proxy_url(url)?;

    debug!("HTTP proxy: {} {} (target: {} via {})", method, path, format!("{}:{}", target_host, target_port), upstream_addr);

    // Read all headers from client
    let mut client_headers = Vec::new();
    let mut header_buf = BufReader::new(&mut client_read);
    let mut line = String::new();

    // Read headers until blank line
    loop {
        line.clear();
        header_buf.read_line(&mut line).await?;
        if line.trim().is_empty() {
            break;
        }
        client_headers.push(line.clone());
    }

    // Read body if Content-Length present
    let mut body = Vec::new();
    let mut headers_str = client_headers.join("");

    // Normalize Host header: rewrite to upstream target or inject if missing
    if let Some(upstream_host_name) = upstream_host {
        // Rewrite Host header to upstream target
        header_rewriter::replace_header_value(&mut headers_str, "Host", upstream_host_name);
        debug!("Rewrote Host header to upstream target: {}", upstream_host_name);
        
        // If Host header was missing (replace_header_value does nothing), inject it
        header_rewriter::inject_header_if_missing(&mut headers_str, "Host", upstream_host_name);
        info!("Host header normalized: {}", upstream_host_name);
    } else {
        // No upstream_host provided, but ensure Host header is present for HTTP/1.1
        if header_rewriter::extract_header_value(&headers_str, "Host").is_none() {
            debug!("Host header missing, extracting from upstream_addr: {}", upstream_addr);
            // Try to extract hostname from upstream_addr (format: "host:port")
            let upstream_hostname = upstream_addr.split(':').next().unwrap_or(upstream_addr);
            header_rewriter::inject_header_if_missing(&mut headers_str, "Host", upstream_hostname);
            info!("Host header injected from upstream_addr: {}", upstream_hostname);
        }
    }

    if let Some(content_length_str) = extract_header_value(&headers_str, "content-length") {
        if let Ok(content_length) = content_length_str.parse::<usize>() {
            let mut body_buf = vec![0u8; content_length];
            header_buf.read_exact(&mut body_buf).await?;
            body = body_buf;
        }
    }

    // Combine request for redaction
    let mut full_request = format!("{}\r\n{}\r\n", first_line, headers_str);
    if !body.is_empty() {
        full_request.push_str(&String::from_utf8_lossy(&body));
    }

    // REDACT request
    // Store selector for potential use in actual filtering (future: implement in redaction engine)
    let _detect_selector_ref = detect_selector.as_ref();
    let _redact_selector_ref = redact_selector.as_ref();
    
    let redacted_request_result = redaction_engine.redact(&full_request);
    let redacted_request = redacted_request_result.redacted;

    if !redacted_request_result.warnings.is_empty() {
        warn!(
            "HTTP proxy request: {} bytes, {} patterns detected (redacted to {} bytes)",
            full_request.len(),
            redacted_request_result.warnings.len(),
            redacted_request.len()
        );
    } else {
        info!(
            "HTTP proxy request: {} bytes, no secrets detected",
            full_request.len()
        );
    }

    // Connect to upstream server or upstream proxy
    let mut upstream = match if upstream_addr.contains("://") {
        connect_through_proxy(upstream_addr, &target_host, target_port).await
    } else {
        DnsResolver::connect_with_retry(upstream_addr).await
    } {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to connect to upstream {}: {}", upstream_addr, e);
            send_error_response(&mut client_write, 502, "Bad Gateway").await?;
            return Err(anyhow!("Upstream connection failed: {}", e));
        }
    };

    // Forward redacted request to upstream
    // When routing through an upstream proxy via CONNECT tunnel, we still send the
    // origin-form HTTP request through the established tunnel.
    upstream.write_all(redacted_request.as_bytes()).await?;
    upstream.flush().await?;

    // Read response from upstream
    let mut response_buf = Vec::new();
    let mut buf = [0u8; 4096];

    loop {
        match upstream.read(&mut buf).await? {
            0 => break, // EOF
            n => response_buf.extend_from_slice(&buf[..n]),
        }
    }

    let response_str = String::from_utf8_lossy(&response_buf).to_string();

    // REDACT response
    // Store selector for potential use in actual filtering (future: implement in redaction engine)
    let _redact_selector_ref_response = redact_selector.as_ref();
    
    let redacted_response_result = redaction_engine.redact(&response_str);
    let redacted_response = redacted_response_result.redacted.clone();

    if !redacted_response_result.warnings.is_empty() {
        warn!(
            "HTTP proxy response: {} bytes, {} patterns detected (redacted to {} bytes)",
            response_str.len(),
            redacted_response_result.warnings.len(),
            redacted_response.len()
        );
    } else {
        info!(
            "HTTP proxy response: {} bytes, no secrets detected",
            response_str.len()
        );
    }

    // Rewrite Location headers to point back to proxy (if applicable)
    let mut final_response = redacted_response.clone();
    
    if let Some(upstream_hostname) = upstream_host {
        // Determine client scheme and proxy host for Location rewriting
        let client_scheme = if first_line.contains("https") { "https" } else { "http" };
        let proxy_host = upstream_addr; // Use the upstream_addr as proxy_host for rewriting
        
        // Check if response contains a Location header
        if let Some(location) = header_rewriter::extract_header_value(&final_response, "Location") {
            if location_rewriter::should_rewrite_location(&location, upstream_hostname) {
                let rewritten_location = location_rewriter::rewrite_location_to_proxy(
                    &location,
                    client_scheme,
                    proxy_host,
                );
                
                header_rewriter::replace_header_value(&mut final_response, "Location", &rewritten_location);
                info!("Rewriting Location header: {} → {}", location, rewritten_location);
            } else {
                debug!("Location header does not point to upstream, keeping as-is: {}", location);
            }
        }
    } else {
        debug!("No upstream_host provided, skipping Location header rewriting");
    }

    // Add proxy detection headers to response
    let final_response = if config.add_via_header || config.add_scred_header {
        inject_proxy_headers(
            &final_response,
            &redacted_response_result,
            &response_str,
            &config,
        )?
    } else {
        final_response
    };

    info!("HTTP proxy completed: response forwarded to client ({} bytes)", final_response.len());

    // Send redacted response with detection headers to client
    client_write.write_all(final_response.as_bytes()).await?;
    client_write.flush().await?;

    debug!("HTTP proxy request completed successfully");

    Ok(())
}

/// Inject Via and X-SCRED headers into HTTP response
fn inject_proxy_headers(
    response: &str,
    redaction_result: &scred_redactor::RedactionResult,
    original_response: &str,
    config: &HttpProxyConfig,
) -> Result<String> {
    let mut final_response = String::new();
    let lines: Vec<&str> = response.split("\r\n").collect();

    if !lines.is_empty() {
        // Add status line (e.g., "HTTP/1.1 200 OK")
        final_response.push_str(lines[0]);
        final_response.push_str("\r\n");

        // Add detection headers
        if config.add_via_header {
            final_response.push_str("Via: 1.1 scred-proxy\r\n");
        }

        if config.add_scred_header {
            let redacted_count = original_response.len() - response.len();
            if redacted_count != 0 || !redaction_result.warnings.is_empty() {
                final_response.push_str(&format!(
                    "X-SCRED-Redacted: true ({} bytes redacted, {} patterns found)\r\n",
                    redacted_count,
                    redaction_result.warnings.len()
                ));
            } else {
                final_response
                    .push_str("X-SCRED-Redacted: false (no secrets found)\r\n");
            }
        }

        // Add remaining headers and body
        for line in &lines[1..] {
            final_response.push_str(line);
            final_response.push_str("\r\n");
        }
    } else {
        final_response = response.to_string();
    }

    Ok(final_response)
}

/// Parse HTTP proxy URL format: http://host:port/path or http://host/path
pub fn parse_proxy_url(url: &str) -> Result<(String, u16, String)> {
    let url = if url.starts_with("http://") {
        &url[7..]
    } else if url.starts_with("https://") {
        &url[8..]
    } else {
        url
    };

    // Split by first /
    let (host_port, path) = if let Some(idx) = url.find('/') {
        (&url[..idx], &url[idx..])
    } else {
        (url, "/")
    };

    // Split host:port
    let (host, port) = if let Some(idx) = host_port.rfind(':') {
        let h = &host_port[..idx];
        let p = &host_port[idx + 1..]
            .parse::<u16>()
            .map_err(|_| anyhow!("Invalid port"))?;
        (h.to_string(), *p)
    } else {
        (host_port.to_string(), 80)
    };

    Ok((host, port, path.to_string()))
}

/// Extract header value (case-insensitive)
fn extract_header_value(headers: &str, name: &str) -> Option<String> {
    let name_lower = format!("{}:", name.to_lowercase());
    for line in headers.lines() {
        if line.to_lowercase().starts_with(&name_lower) {
            let value = line.split(':').nth(1)?;
            return Some(value.trim().to_string());
        }
    }
    None
}

/// Send HTTP error response
async fn send_error_response(
    write: &mut tokio::net::tcp::OwnedWriteHalf,
    code: u16,
    reason: &str,
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        code, reason
    );
    write.write_all(response.as_bytes()).await?;
    write.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_with_port() {
        let (host, port, path) = parse_proxy_url("http://example.com:8080/api/test").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 8080);
        assert_eq!(path, "/api/test");
    }

    #[test]
    fn test_parse_url_without_port() {
        let (host, port, path) = parse_proxy_url("http://example.com/api").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
        assert_eq!(path, "/api");
    }

    #[test]
    fn test_parse_url_root_path() {
        let (host, port, path) = parse_proxy_url("http://example.com").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
        assert_eq!(path, "/");
    }

    #[test]
    fn test_extract_header_value() {
        let headers = "Host: example.com\r\nContent-Length: 100\r\n";
        assert_eq!(
            extract_header_value(headers, "content-length"),
            Some("100".to_string())
        );
        assert_eq!(extract_header_value(headers, "host"), Some("example.com".to_string()));
    }

    mod host_header_normalization {
        use super::*;

        #[test]
        fn test_host_header_rewriting() {
            let mut headers = "Host: localhost:9999\r\nUser-Agent: curl\r\n".to_string();
            
            // Before: Host: localhost:9999
            assert_eq!(
                header_rewriter::extract_header_value(&headers, "Host"),
                Some("localhost:9999".to_string())
            );
            
            // Rewrite to upstream target
            header_rewriter::replace_header_value(&mut headers, "Host", "httpbin.org");
            
            // After: Host: httpbin.org
            assert_eq!(
                header_rewriter::extract_header_value(&headers, "Host"),
                Some("httpbin.org".to_string())
            );
            
            // Other headers preserved
            assert_eq!(
                header_rewriter::extract_header_value(&headers, "User-Agent"),
                Some("curl".to_string())
            );
        }

        #[test]
        fn test_host_header_injection_if_missing() {
            let mut headers = "User-Agent: curl\r\n".to_string();
            
            // Before: no Host header
            assert_eq!(
                header_rewriter::extract_header_value(&headers, "Host"),
                None
            );
            
            // Inject Host header
            header_rewriter::inject_header_if_missing(&mut headers, "Host", "httpbin.org");
            
            // After: Host: httpbin.org
            assert_eq!(
                header_rewriter::extract_header_value(&headers, "Host"),
                Some("httpbin.org".to_string())
            );
        }

        #[test]
        fn test_host_header_not_overwritten_if_present() {
            let mut headers = "Host: already-present.com\r\nUser-Agent: curl\r\n".to_string();
            
            // Before: Host: already-present.com
            assert_eq!(
                header_rewriter::extract_header_value(&headers, "Host"),
                Some("already-present.com".to_string())
            );
            
            // Inject should do nothing if already present
            header_rewriter::inject_header_if_missing(&mut headers, "Host", "httpbin.org");
            
            // After: still Host: already-present.com
            assert_eq!(
                header_rewriter::extract_header_value(&headers, "Host"),
                Some("already-present.com".to_string())
            );
        }
    }

    mod location_header_rewriting {
        use super::*;

        #[test]
        fn test_location_absolute_uri_should_rewrite() {
            let location = "https://httpbin.org/redirect/1";
            let upstream_host = "httpbin.org";
            
            assert!(location_rewriter::should_rewrite_location(location, upstream_host));
        }

        #[test]
        fn test_location_absolute_path_should_not_rewrite() {
            let location = "/other/path";
            let upstream_host = "httpbin.org";
            
            assert!(!location_rewriter::should_rewrite_location(location, upstream_host));
        }

        #[test]
        fn test_location_rewrite_to_proxy() {
            let location = "https://httpbin.org/redirect/1?next=/get";
            let result = location_rewriter::rewrite_location_to_proxy(location, "http", "localhost:9999");
            
            assert_eq!(result, "http://localhost:9999/redirect/1?next=/get");
        }

        #[test]
        fn test_location_different_upstream_should_not_rewrite() {
            let location = "https://example.com/path";
            let upstream_host = "httpbin.org";
            
            assert!(!location_rewriter::should_rewrite_location(location, upstream_host));
        }
    }
}
