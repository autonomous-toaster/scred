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

#[cfg(test)]
mod tests {
    use super::*;

    mod is_absolute_uri {
        use super::*;

        #[test]
        fn test_absolute_uri_https() {
            assert!(is_absolute_uri("https://httpbin.org/path"));
        }

        #[test]
        fn test_absolute_uri_http() {
            assert!(is_absolute_uri("http://example.com/path"));
        }

        #[test]
        fn test_absolute_uri_with_port() {
            assert!(is_absolute_uri("https://httpbin.org:443/path"));
        }

        #[test]
        fn test_absolute_path_is_not_absolute_uri() {
            assert!(!is_absolute_uri("/absolute/path"));
        }

        #[test]
        fn test_relative_path_is_not_absolute_uri() {
            assert!(!is_absolute_uri("relative/path"));
        }

        #[test]
        fn test_empty_string() {
            assert!(!is_absolute_uri(""));
        }
    }

    mod extract_host_from_uri {
        use super::*;

        #[test]
        fn test_extract_host_with_path() {
            assert_eq!(
                extract_host_from_uri("https://httpbin.org/path"),
                Some("httpbin.org".to_string())
            );
        }

        #[test]
        fn test_extract_host_with_port() {
            assert_eq!(
                extract_host_from_uri("https://httpbin.org:443/path"),
                Some("httpbin.org:443".to_string())
            );
        }

        #[test]
        fn test_extract_host_with_query() {
            assert_eq!(
                extract_host_from_uri("https://httpbin.org/path?query=value"),
                Some("httpbin.org".to_string())
            );
        }

        #[test]
        fn test_extract_host_no_path() {
            assert_eq!(
                extract_host_from_uri("https://httpbin.org"),
                Some("httpbin.org".to_string())
            );
        }

        #[test]
        fn test_extract_host_malformed_no_scheme() {
            assert_eq!(extract_host_from_uri("httpbin.org/path"), None);
        }

        #[test]
        fn test_extract_host_empty_host() {
            assert_eq!(extract_host_from_uri("https:///path"), None);
        }

        #[test]
        fn test_extract_host_with_userinfo() {
            // Note: this handles user:pass@host format
            assert_eq!(
                extract_host_from_uri("https://user:pass@httpbin.org/path"),
                Some("user:pass@httpbin.org".to_string())
            );
        }
    }

    mod should_rewrite_location {
        use super::*;

        #[test]
        fn test_should_rewrite_matching_upstream() {
            assert!(should_rewrite_location(
                "https://httpbin.org/redirect/1",
                "httpbin.org"
            ));
        }

        #[test]
        fn test_should_rewrite_matching_with_port() {
            assert!(should_rewrite_location(
                "https://httpbin.org:443/redirect/1",
                "httpbin.org:443"
            ));
        }

        #[test]
        fn test_should_not_rewrite_different_host() {
            assert!(!should_rewrite_location(
                "https://example.com/path",
                "httpbin.org"
            ));
        }

        #[test]
        fn test_should_not_rewrite_absolute_path() {
            assert!(!should_rewrite_location("/absolute/path", "httpbin.org"));
        }

        #[test]
        fn test_should_not_rewrite_relative_path() {
            assert!(!should_rewrite_location("relative/path", "httpbin.org"));
        }

        #[test]
        fn test_should_rewrite_case_insensitive() {
            assert!(should_rewrite_location(
                "https://HTTPBIN.ORG/path",
                "httpbin.org"
            ));
        }

        #[test]
        fn test_should_not_rewrite_hostname_mismatch_with_ports() {
            // Both have ports but they're different
            assert!(!should_rewrite_location(
                "https://httpbin.org:8443/path",
                "httpbin.org:443"
            ));
        }
    }

    mod rewrite_location_to_proxy {
        use super::*;

        #[test]
        fn test_rewrite_basic_absolute_uri() {
            let result =
                rewrite_location_to_proxy("https://httpbin.org/redirect/1", "http", "localhost:9999");
            assert_eq!(result, "http://localhost:9999/redirect/1");
        }

        #[test]
        fn test_rewrite_preserves_query_string() {
            let result = rewrite_location_to_proxy(
                "https://httpbin.org/redirect/1?param=value",
                "http",
                "localhost:9999",
            );
            assert_eq!(result, "http://localhost:9999/redirect/1?param=value");
        }

        #[test]
        fn test_rewrite_preserves_fragment() {
            let result = rewrite_location_to_proxy(
                "https://httpbin.org/path#section",
                "http",
                "localhost:9999",
            );
            assert_eq!(result, "http://localhost:9999/path#section");
        }

        #[test]
        fn test_rewrite_preserves_port_in_proxy_host() {
            let result = rewrite_location_to_proxy(
                "https://httpbin.org/redirect/1",
                "https",
                "localhost:8443",
            );
            assert_eq!(result, "https://localhost:8443/redirect/1");
        }

        #[test]
        fn test_rewrite_with_upstream_port() {
            let result = rewrite_location_to_proxy(
                "https://httpbin.org:443/path",
                "http",
                "localhost:9999",
            );
            assert_eq!(result, "http://localhost:9999/path");
        }

        #[test]
        fn test_rewrite_uses_client_scheme() {
            // Same upstream path, different client schemes
            let result_http =
                rewrite_location_to_proxy("https://httpbin.org/path", "http", "localhost:9999");
            let result_https =
                rewrite_location_to_proxy("https://httpbin.org/path", "https", "localhost:9999");
            
            assert_eq!(result_http, "http://localhost:9999/path");
            assert_eq!(result_https, "https://localhost:9999/path");
        }

        #[test]
        fn test_rewrite_root_path() {
            let result =
                rewrite_location_to_proxy("https://httpbin.org/", "http", "localhost:9999");
            assert_eq!(result, "http://localhost:9999/");
        }

        #[test]
        fn test_rewrite_malformed_location() {
            // If location doesn't have scheme, find defaults to None, so we get /
            let result = rewrite_location_to_proxy("/already/a/path", "http", "localhost:9999");
            // Without :// in location, path_and_after defaults to /
            assert_eq!(result, "http://localhost:9999/");
        }
    }

    mod integration {
        use super::*;

        #[test]
        fn test_full_redirect_rewrite_workflow() {
            let location = "https://httpbin.org:443/redirect/1?next=/get";
            let upstream_host = "httpbin.org:443";
            let client_scheme = "http";
            let proxy_host = "localhost:9999";
            
            // Check if should rewrite
            assert!(should_rewrite_location(location, upstream_host));
            
            // Rewrite location
            let rewritten = rewrite_location_to_proxy(location, client_scheme, proxy_host);
            
            // Verify result
            assert_eq!(rewritten, "http://localhost:9999/redirect/1?next=/get");
        }

        #[test]
        fn test_no_rewrite_for_different_upstream() {
            let location = "https://example.com/path";
            let upstream_host = "httpbin.org";
            
            assert!(!should_rewrite_location(location, upstream_host));
        }

        #[test]
        fn test_no_rewrite_for_relative_path() {
            let location = "/relative/path";
            let upstream_host = "httpbin.org";
            
            assert!(!should_rewrite_location(location, upstream_host));
        }
    }
}
