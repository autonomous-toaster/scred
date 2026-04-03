/// Cached DNS Resolver - Adds result caching to DnsResolver
///
/// Combines DnsResolver with DnsCache to avoid repeated lookups.
/// Provides significant performance improvement for workloads with:
/// - Multiple connections to same upstreams
/// - Repeated DNS resolution calls
///
/// Performance Goal: Save 1-5ms per cached DNS hit
///
/// Zero-Copy Strategy:
/// - Cache key is String (one allocation per unique hostname)
/// - Cache values are Vec<SocketAddr> (moved, no cloning on hit)
/// - Arc<Mutex> for thread-safe sharing
/// - Minimal contention (fast get/set operations)
use crate::dns_cache::DnsCache;
use crate::dns_resolver::DnsResolver;
use anyhow::Result;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::{debug, info};

/// Configuration for cached DNS resolver
#[derive(Clone, Debug)]
pub struct CachedDnsConfig {
    /// TTL for DNS cache entries in seconds (default: 60)
    pub ttl_secs: u64,
    /// Enable caching (default: true)
    pub enabled: bool,
}

impl Default for CachedDnsConfig {
    fn default() -> Self {
        Self {
            ttl_secs: 60,
            enabled: true,
        }
    }
}

/// DNS resolver with caching wrapper
///
/// PERFORMANCE CRITICAL: Checks cache before expensive DNS lookup
pub struct CachedDnsResolver {
    cache: DnsCache,
    config: CachedDnsConfig,
}

impl CachedDnsResolver {
    /// Create new cached DNS resolver with given configuration
    pub fn new(config: CachedDnsConfig) -> Self {
        Self {
            cache: DnsCache::new(config.ttl_secs),
            config,
        }
    }

    /// Create with default configuration (60s TTL, enabled)
    pub fn default() -> Self {
        Self::new(CachedDnsConfig::default())
    }

    /// Connect to address, using cache for DNS lookups
    ///
    /// PERFORMANCE CRITICAL PATH:
    /// - Try cache.get() [fast, <1µs]
    /// - If miss → DnsResolver.try_connect() [slow, ~1-5ms]
    /// - Cache result on success
    /// - Return connection
    ///
    /// Cache behavior:
    /// - Hit: Skip DNS lookup entirely (~1-5ms saved)
    /// - Miss: Perform DNS lookup, cache result
    /// - Expired: Remove from cache, perform fresh lookup
    pub async fn connect_with_retry(&self, addr: &str) -> Result<TcpStream> {
        if !self.config.enabled {
            // Caching disabled - use DnsResolver directly
            debug!("DNS cache disabled, using direct resolver for {}", addr);
            return DnsResolver::connect_with_retry(addr).await;
        }

        // Strip scheme if present
        let addr_without_scheme = Self::strip_scheme(addr);
        let hostname = Self::extract_hostname(addr_without_scheme);

        // Try to get cached addresses
        if let Some(cached_addrs) = self.cache.get(&hostname) {
            debug!(
                "DNS cache hit for {} ({} addresses)",
                hostname,
                cached_addrs.len()
            );

            // Try each cached address
            let mut last_error = None;
            for socket_addr in &cached_addrs {
                match TcpStream::connect(*socket_addr).await {
                    Ok(stream) => {
                        info!("Connected via cached DNS: {} -> {}", hostname, socket_addr);
                        return Ok(stream);
                    }
                    Err(e) => {
                        debug!("Cached address {} failed: {}", socket_addr, e);
                        last_error = Some(e);
                    }
                }
            }

            // All cached addresses failed
            debug!(
                "All cached addresses for {} failed, falling back to fresh DNS lookup",
                hostname
            );
            if let Some(e) = last_error {
                debug!("Last error from cached addresses: {}", e);
            }
        }

        // Cache miss or stale → perform DNS lookup via DnsResolver
        debug!("DNS cache miss for {} - performing lookup", hostname);

        // Use DnsResolver with retry logic
        let stream = DnsResolver::connect_with_retry(addr_without_scheme).await?;

        // On success, extract and cache the resolved addresses
        // Note: We perform a fresh resolution to get the addresses to cache
        // This is unavoidable with the current DnsResolver API (it doesn't return addresses)
        if let Ok(resolved_addrs) = Self::resolve_addresses(addr_without_scheme) {
            debug!(
                "Caching {} resolved addresses for {}",
                resolved_addrs.len(),
                hostname
            );
            self.cache.set(hostname.clone(), resolved_addrs);
        }

        Ok(stream)
    }

    /// Strip URL scheme from address if present
    fn strip_scheme(addr: &str) -> &str {
        if let Some(idx) = addr.find("://") {
            &addr[idx + 3..]
        } else {
            addr
        }
    }

    /// Extract hostname from address (hostname:port format)
    fn extract_hostname(addr: &str) -> String {
        // Handle IPv6 addresses [::1]:port
        if let Some(bracket_idx) = addr.find(']') {
            // IPv6: [::1]:8080 → [::1]
            if let Some(addr_str) = addr.get(..=bracket_idx) {
                return addr_str.to_string();
            }
        }

        // Handle regular hostname:port
        if let Some(colon_idx) = addr.rfind(':') {
            if let Some(host) = addr.get(..colon_idx) {
                // Avoid treating IPv6 :: as port separator
                if !host.contains(':') || host.starts_with('[') {
                    return host.to_string();
                }
            }
        }

        // No port found, use whole address
        addr.to_string()
    }

    /// Resolve addresses for hostname (performs actual DNS lookup)
    /// This is a helper to populate the cache
    fn resolve_addresses(addr: &str) -> Result<Vec<SocketAddr>> {
        use std::net::ToSocketAddrs;

        let addrs: Vec<SocketAddr> = addr.to_socket_addrs()?.collect();

        Ok(addrs)
    }

    /// Get cache statistics
    pub fn cache_size(&self) -> usize {
        self.cache.size()
    }

    /// Clear all cached entries
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_scheme() {
        assert_eq!(
            CachedDnsResolver::strip_scheme("http://localhost:8080"),
            "localhost:8080"
        );
        assert_eq!(
            CachedDnsResolver::strip_scheme("https://localhost:443"),
            "localhost:443"
        );
        assert_eq!(
            CachedDnsResolver::strip_scheme("localhost:8080"),
            "localhost:8080"
        );
    }

    #[test]
    fn test_extract_hostname_ipv4() {
        assert_eq!(
            CachedDnsResolver::extract_hostname("127.0.0.1:8080"),
            "127.0.0.1"
        );
        assert_eq!(
            CachedDnsResolver::extract_hostname("localhost:8080"),
            "localhost"
        );
        assert_eq!(
            CachedDnsResolver::extract_hostname("example.com:443"),
            "example.com"
        );
    }

    #[test]
    fn test_extract_hostname_ipv6() {
        assert_eq!(CachedDnsResolver::extract_hostname("[::1]:8080"), "[::1]");
        assert_eq!(
            CachedDnsResolver::extract_hostname("[2001:db8::1]:443"),
            "[2001:db8::1]"
        );
    }

    #[test]
    fn test_cached_dns_config_default() {
        let config = CachedDnsConfig::default();
        assert_eq!(config.ttl_secs, 60);
        assert!(config.enabled);
    }

    #[test]
    fn test_cached_dns_resolver_disabled() {
        let config = CachedDnsConfig {
            enabled: false,
            ..Default::default()
        };
        let resolver = CachedDnsResolver::new(config);
        assert_eq!(resolver.cache_size(), 0);
    }

    #[test]
    fn test_resolve_addresses() {
        // This test requires network access, so we skip it in general test runs
        // In practice, this would be tested via integration tests
    }

    #[tokio::test]
    async fn test_cached_resolver_creation() {
        let resolver = CachedDnsResolver::default();
        assert!(resolver.config.enabled);
        assert_eq!(resolver.cache_size(), 0);
    }
}
