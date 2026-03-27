//! HTTP Location Header Rewriting Utilities
//!
//! Provides functions for analyzing and rewriting HTTP Location headers to maintain
//! proxy transparency. Absolute-URI redirects pointing to the upstream server are
//! rewritten to point back to the proxy, preventing clients from bypassing the proxy.
//!
//! # Redirect Types
//!
//! - **Absolute-URI**: `Location: https://httpbin.org/path`
//!   - Rewritten if pointing to upstream host
//!   - Rewritten to: scheme from client request, host from proxy, path preserved
//!
//! - **Absolute-path**: `Location: /other/path`
//!   - Not rewritten (already relative to proxy)
//!
//! - **Relative**: `Location: other/path`
//!   - Not rewritten (already relative to current location)

/// Check if a location string is an absolute-URI (contains scheme)
///
/// Absolute-URIs contain a scheme component (e.g., `https://`).
/// Returns true only for absolute-URI format, false for paths and relative URIs.
///
/// # Examples
/// ```ignore
/// assert_eq!(is_absolute_uri("https://example.com/path"), true);
/// assert_eq!(is_absolute_uri("/absolute/path"), false);
/// assert_eq!(is_absolute_uri("relative/path"), false);
/// ```
pub fn is_absolute_uri(location: &str) -> bool {
    location.contains("://")
}

/// Extract hostname from a URI string (case-sensitive, may include port)
///
/// Parses the host component from a URI in the format `scheme://host:port/path`.
/// Handles both hostnames with and without ports.
/// Returns the host component without the scheme or path.
///
/// # Arguments
/// * `uri` - URI string to parse
///
/// # Returns
/// * `Some(host)` - Extracted host (e.g., "httpbin.org" or "httpbin.org:443")
/// * `None` - If URI is malformed or missing host component
///
/// # Examples
/// ```ignore
/// assert_eq!(extract_host_from_uri("https://httpbin.org/path"), Some("httpbin.org".to_string()));
/// assert_eq!(extract_host_from_uri("https://httpbin.org:443/path"), Some("httpbin.org:443".to_string()));
/// assert_eq!(extract_host_from_uri("not-a-uri"), None);
/// ```
pub fn extract_host_from_uri(uri: &str) -> Option<String> {
    // Find scheme end (://)
    let scheme_end = uri.find("://")?;
    let after_scheme = &uri[scheme_end + 3..];
    
    // Find path start (first / after scheme)
    let path_start = after_scheme.find('/').unwrap_or(after_scheme.len());
    let host_part = &after_scheme[..path_start];
    
    if host_part.is_empty() {
        return None;
    }
    
    Some(host_part.to_string())
}

/// Check if a location should be rewritten based on upstream host
///
/// Returns true only if:
/// 1. Location is an absolute-URI (contains `://`)
/// 2. The hostname in the URI matches the upstream host (case-insensitive comparison of hostnames)
///
/// Returns false for:
/// - Absolute-path redirects (`/path`)
/// - Relative redirects (`path`)
/// - Absolute-URIs pointing to different hosts
///
/// # Arguments
/// * `location` - Location header value from server
/// * `upstream_host` - Upstream server hostname to match against
///
/// # Examples
/// ```ignore
/// // Should rewrite - points to upstream
/// assert_eq!(should_rewrite_location("https://httpbin.org/path", "httpbin.org"), true);
///
/// // Should NOT rewrite - absolute-path
/// assert_eq!(should_rewrite_location("/path", "httpbin.org"), false);
///
/// // Should NOT rewrite - different host
/// assert_eq!(should_rewrite_location("https://example.com/path", "httpbin.org"), false);
/// ```
pub fn should_rewrite_location(location: &str, upstream_host: &str) -> bool {
    // Must be absolute-URI
    if !is_absolute_uri(location) {
        return false;
    }
    
    // Extract host from location
    match extract_host_from_uri(location) {
        Some(location_host) => {
            // Compare: if both have explicit ports, they must match exactly (case-insensitive)
            // If one has a port and the other doesn't, we need to be careful
            let location_hostname = location_host.split(':').next().unwrap_or(&location_host);
            let upstream_hostname = upstream_host.split(':').next().unwrap_or(upstream_host);
            
            // Hostnames must match (case-insensitive)
            if location_hostname.to_lowercase() != upstream_hostname.to_lowercase() {
                return false;
            }
            
            // If both specify ports explicitly, they must match too
            let location_port = location_host.split(':').nth(1);
            let upstream_port = upstream_host.split(':').nth(1);
            
            match (location_port, upstream_port) {
                // Both have ports: must match
                (Some(lp), Some(up)) => lp == up,
                // Only one has port: still rewrite (common case - upstream without port, location with default port)
                _ => true,
            }
        }
        None => false,
    }
}

/// Rewrite a Location header to point back to the proxy
///
/// Takes an absolute-URI Location header and rewrites it to point to the proxy instead,
/// while preserving the path component. Uses the scheme and host from the proxy endpoint.
///
/// # Arguments
/// * `location` - Original Location header value (e.g., `https://httpbin.org:443/redirect/1`)
/// * `client_scheme` - Scheme from client request (http or https)
/// * `proxy_host` - Proxy hostname:port to use (e.g., `localhost:9999`)
///
/// # Returns
/// * Rewritten location (e.g., `http://localhost:9999/redirect/1`)
/// * Preserves path, query string, and fragments
/// * Uses client_scheme to maintain the scheme the client used
///
/// # Examples
/// ```ignore
/// let rewritten = rewrite_location_to_proxy(
///     "https://httpbin.org:443/redirect/1?param=value",
///     "http",
///     "localhost:9999"
/// );
/// assert_eq!(rewritten, "http://localhost:9999/redirect/1?param=value");
/// ```
pub fn rewrite_location_to_proxy(location: &str, client_scheme: &str, proxy_host: &str) -> String {
    // Extract path, query, and fragment from location
    let path_and_after = match location.find("://") {
        Some(scheme_end) => {
            let after_scheme = &location[scheme_end + 3..];
            match after_scheme.find('/') {
                Some(path_start) => &after_scheme[path_start..],
                None => "/",
            }
        }
        None => "/",
    };
    
    // Reconstruct as: scheme://proxy_host/path
    format!("{}://{}{}", client_scheme, proxy_host, path_and_after)
}

