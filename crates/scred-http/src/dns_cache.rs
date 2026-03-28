/// DNS Result Cache with TTL
/// Caches successful DNS resolutions to avoid repeated lookups

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

#[derive(Clone)]
struct CachedDnsEntry {
    addresses: Vec<SocketAddr>,
    expires_at: Instant,
}

/// Thread-safe DNS cache
pub struct DnsCache {
    cache: Arc<Mutex<HashMap<String, CachedDnsEntry>>>,
    ttl: Duration,
}

impl DnsCache {
    /// Create a new DNS cache with TTL (default: 60 seconds)
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Default: 60 second TTL
    pub fn default_ttl() -> Self {
        Self::new(60)
    }

    /// Try to get cached addresses for a hostname
    pub fn get(&self, hostname: &str) -> Option<Vec<SocketAddr>> {
        let mut cache = self.cache.lock().unwrap();
        
        if let Some(entry) = cache.get(hostname) {
            if Instant::now() < entry.expires_at {
                return Some(entry.addresses.clone());
            } else {
                // Expired, remove it
                cache.remove(hostname);
            }
        }
        
        None
    }

    /// Store addresses in cache
    pub fn set(&self, hostname: String, addresses: Vec<SocketAddr>) {
        if !addresses.is_empty() {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(hostname, CachedDnsEntry {
                addresses,
                expires_at: Instant::now() + self.ttl,
            });
        }
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        self.cache.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_cache_basic() {
        let cache = DnsCache::new(60);
        let addr = "127.0.0.1:8080".parse().unwrap();
        
        // Initially empty
        assert!(cache.get("localhost").is_none());
        
        // Store entry
        cache.set("localhost".to_string(), vec![addr]);
        
        // Retrieve entry
        let addrs = cache.get("localhost").unwrap();
        assert_eq!(addrs.len(), 1);
        assert_eq!(addrs[0], addr);
    }

    #[test]
    fn test_dns_cache_expiry() {
        let cache = DnsCache::new(0);  // 0 second TTL - immediate expiry
        let addr = "127.0.0.1:8080".parse().unwrap();
        
        cache.set("localhost".to_string(), vec![addr]);
        
        // Small sleep to ensure expiry
        std::thread::sleep(Duration::from_millis(10));
        
        // Should be expired
        assert!(cache.get("localhost").is_none());
    }

    #[test]
    fn test_dns_cache_multiple_addresses() {
        let cache = DnsCache::new(60);
        let addr1 = "127.0.0.1:8080".parse().unwrap();
        let addr2 = "127.0.0.1:8081".parse().unwrap();
        
        cache.set("localhost".to_string(), vec![addr1, addr2]);
        
        let addrs = cache.get("localhost").unwrap();
        assert_eq!(addrs.len(), 2);
    }

    #[test]
    fn test_dns_cache_clear() {
        let cache = DnsCache::new(60);
        let addr = "127.0.0.1:8080".parse().unwrap();
        
        cache.set("localhost".to_string(), vec![addr]);
        assert_eq!(cache.size(), 1);
        
        cache.clear();
        assert_eq!(cache.size(), 0);
    }
}
