# P1-1: Wire Connection Pool into Proxy - Detailed Spec

## Overview
Enable existing `connection_pool.rs` in scred-proxy to reuse TCP connections to upstream, achieving 3-5 MB/s throughput via concurrent connection pooling.

## Current State

### Existing Code
- **Location**: `crates/scred-http/src/connection_pool.rs` (150+ LOC)
- **Status**: Fully implemented, NOT USED
- **Interface**:
  ```rust
  pub struct ConnectionPool<T> { ... }
  impl<T: Send + 'static> ConnectionPool<T> {
      pub fn new(capacity: usize) -> Self
      pub async fn get(&self) -> Result<PooledConnection<T>>
      pub async fn put(&self, conn: T)
      pub async fn clear(&self)
  }
  ```

### Current Bottleneck in Proxy
```rust
// In crates/scred-proxy/src/main.rs:
// CURRENT (sequential):
for request in incoming_requests {
    let tcp_stream = DnsResolver::connect_with_retry(&upstream_addr).await?;
    // ^ Creates NEW connection every request
    // Typical: 5ms to establish, 200ms to get response
    // Result: ~206ms per request = 0.97 MB/s
}

// DESIRED (pooled):
for request in incoming_requests {
    let tcp_stream = pool.get(&upstream_addr).await?;
    // ^ Reuses connection or creates if needed
    // With 5 pooled connections: ~40ms per request = 4.85 MB/s
}
```

## Implementation Plan

### Step 1: Understand Current DnsResolver Implementation
**File**: `crates/scred-http/src/dns_resolver.rs`

Current signature:
```rust
impl DnsResolver {
    pub async fn connect_with_retry(addr: &str) -> Result<TcpStream> {
        // 1. Parse address
        // 2. Resolve DNS (exponential backoff on failure)
        // 3. Create new TcpStream every call
        // 4. Return stream
    }
}
```

**Assessment**: This is where we hook the pool.

---

### Step 2: Create Connection Pool Wrapper

**File to create**: `crates/scred-http/src/pooled_dns_resolver.rs`

```rust
use crate::connection_pool::ConnectionPool;
use tokio::net::TcpStream;

pub struct PooledDnsResolver {
    /// Stores pools by upstream address
    pools: Arc<tokio::sync::RwLock<HashMap<String, ConnectionPool<TcpStream>>>>,
    /// Configuration
    config: PoolConfig,
}

#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// Maximum connections per upstream (default: 10)
    pub max_connections: usize,
    /// Idle timeout before dropping connection (default: 30s)
    pub idle_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            idle_timeout: Duration::from_secs(30),
        }
    }
}

impl PooledDnsResolver {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            pools: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Get or create a pooled connection
    pub async fn connect_with_retry(&self, addr: &str) -> Result<PooledTcpStream> {
        // 1. Get pool for this address (create if needed)
        // 2. Try to get existing connection
        // 3. If none available, create new (up to max)
        // 4. Return wrapped stream with drop logic
        
        let mut pools = self.pools.write().await;
        let pool = pools.entry(addr.to_string())
            .or_insert_with(|| ConnectionPool::new(self.config.max_connections));
        
        // Try to get from pool
        match pool.get().await {
            Ok(conn) => Ok(PooledTcpStream::from_pool(conn)),
            Err(_) => {
                // Pool empty, create new connection
                let stream = TcpStream::connect(addr).await?;
                Ok(PooledTcpStream::new(stream))
            }
        }
    }

    /// Cleanup idle connections (run periodically)
    pub async fn cleanup_idle(&self) {
        for (_, pool) in self.pools.read().await.iter() {
            pool.clear().await;
        }
    }
}

/// Wrapper that returns connection to pool on drop
pub struct PooledTcpStream {
    stream: Option<TcpStream>,
    pool: Option<Arc<ConnectionPool<TcpStream>>>,
}

impl Drop for PooledTcpStream {
    fn drop(&mut self) {
        if let (Some(stream), Some(pool)) = (self.stream.take(), self.pool.take()) {
            // Return to pool (async, need spawn)
            let pool = pool.clone();
            tokio::spawn(async move {
                let _ = pool.put(stream).await;
            });
        }
    }
}
```

---

### Step 3: Modify Proxy Configuration

**File**: `crates/scred-proxy/src/main.rs`

Add to `ProxyConfig`:
```rust
pub struct ProxyConfig {
    // ... existing fields ...
    
    /// Connection pool configuration
    pub pool_config: PoolConfig,
}

impl ProxyConfig {
    pub fn from_config_file() -> Result<Self> {
        // ... existing logic ...
        
        // Read pool config from file if present
        // Otherwise use defaults
        let pool_config = if let Some(pool_section) = config.get("pool") {
            PoolConfig {
                max_connections: pool_section.get("max_connections")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(10) as usize,
                idle_timeout: Duration::from_secs(
                    pool_section.get("idle_timeout_secs")
                        .and_then(|v| v.as_integer())
                        .unwrap_or(30) as u64
                ),
            }
        } else {
            PoolConfig::default()
        };
        
        Ok(ProxyConfig {
            // ... existing fields ...
            pool_config,
        })
    }
}
```

**Config file format** (in `scred-proxy.toml`):
```toml
[pool]
# Maximum concurrent connections per upstream (default: 10)
max_connections = 10

# Idle timeout in seconds (default: 30)
idle_timeout_secs = 30
```

---

### Step 4: Modify Connection Handler in Proxy

**File**: `crates/scred-proxy/src/main.rs` → `handle_connection()` function

Change from:
```rust
async fn handle_connection(stream: TcpStream, config: Arc<ProxyConfig>) -> Result<()> {
    let upstream_addr = config.upstream.authority();
    
    // OLD: Create new connection every request
    let tcp_stream = DnsResolver::connect_with_retry(&upstream_addr).await?;
    
    // ... rest of handler ...
}
```

To:
```rust
async fn handle_connection(
    stream: TcpStream,
    config: Arc<ProxyConfig>,
    pool: Arc<PooledDnsResolver>,  // ← NEW parameter
) -> Result<()> {
    let upstream_addr = config.upstream.authority();
    
    // NEW: Get connection from pool
    let tcp_stream = pool.connect_with_retry(&upstream_addr).await?;
    
    // ... rest of handler (unchanged) ...
}
```

Update main loop spawn:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ... setup ...
    
    // Create pool once, share across all connections
    let pool = Arc::new(PooledDnsResolver::new(config.pool_config.clone()));
    
    // Clone for spawn
    let pool_clone = pool.clone();
    
    // Listen for connections
    loop {
        let (stream, _) = listener.accept().await?;
        let config = config.clone();
        let pool = pool_clone.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, config, pool).await {
                eprintln!("Error: {}", e);
            }
        });
    }
}
```

---

### Step 5: Update Dependencies

**File**: `crates/scred-http/Cargo.toml`

Add if not present:
```toml
[dependencies]
tokio = { version = "1.0", features = ["rt", "sync", "time"] }
```

---

### Step 6: Update Exports

**File**: `crates/scred-http/src/lib.rs`

Add:
```rust
pub mod pooled_dns_resolver;
pub use pooled_dns_resolver::{PooledDnsResolver, PoolConfig, PooledTcpStream};

// Export existing pool
pub mod connection_pool;
pub use connection_pool::ConnectionPool;
```

---

## Testing Strategy

### Unit Tests (in `pooled_dns_resolver.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_creates_connection() {
        // Test that first request creates connection
    }

    #[tokio::test]
    async fn test_pool_reuses_connection() {
        // Test that second request reuses pooled connection
    }

    #[tokio::test]
    async fn test_pool_respects_max_connections() {
        // Test that pool doesn't exceed max_connections
    }

    #[tokio::test]
    async fn test_pool_idle_timeout() {
        // Test that idle connections are cleaned up
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        // Test 10 concurrent requests reuse pool
    }
}
```

### Integration Tests (new file: `crates/scred-proxy/tests/pooling_test.rs`)
```rust
#[tokio::test]
async fn test_proxy_with_pooled_connections() {
    // 1. Start scred-debug-server
    // 2. Start proxy with pool config (max 5)
    // 3. Send 10 concurrent requests
    // 4. Verify:
    //    - All requests succeed
    //    - Connection reuse happens (monitor pool metrics)
    //    - Throughput is 3-5 MB/s
}

#[tokio::test]
async fn test_proxy_without_pooling() {
    // Regression test: verify it still works with pool_max=1
}

#[tokio::test]
async fn test_multiple_upstream_servers() {
    // Test with 2 upstream servers, verify separate pools
}

#[tokio::test]
async fn test_downstream_connection_pool_separation() {
    // Each downstream client has independent pool
}
```

### Benchmark Test (new file: `benches/pooling_benchmark.rs`)
```rust
#[bench]
fn benchmark_proxy_with_pooling(b: &mut Bencher) {
    // Send 1000 requests, measure throughput
    // Expected: 3-5 MB/s (vs 0.90 MB/s baseline)
}

#[bench]
fn benchmark_pool_connection_reuse(b: &mut Bencher) {
    // Time how long pool.get() takes
    // First call: ~5ms (create)
    // Subsequent: <0.1ms (reuse)
}
```

---

## Rollout Strategy

### Phase 1a: Feature Flag
```rust
// Disable pool by default initially
pub struct PoolConfig {
    pub enabled: bool,  // default: false
    pub max_connections: usize,
    pub idle_timeout: Duration,
}
```

This allows safe rollout:
```toml
[pool]
enabled = true  # Enable when confident
max_connections = 5  # Start conservative, increase if stable
```

### Phase 1b: Staged Deployment
1. **Week 1**: Deploy with `pool.enabled = false` (no-op)
2. **Week 2**: Deploy with `pool.enabled = true, max = 5`
3. **Week 3**: Increase to `max = 10` if stable
4. **Week 4**: Remove feature flag once confident

---

## Verification Checklist

- [ ] Connection pool code exists and compiles
- [ ] PooledDnsResolver compiles and has no warnings
- [ ] Proxy can start with pool enabled
- [ ] Single request works (doesn't regress)
- [ ] Multiple concurrent requests work
- [ ] Throughput increases to 3-5 MB/s
- [ ] Memory usage reasonable (10 connections × ~1KB = ~10KB overhead)
- [ ] CPU usage doesn't increase significantly
- [ ] DNS resolution still works with pool
- [ ] Connection timeout/cleanup works
- [ ] All existing tests pass
- [ ] New pooling tests pass
- [ ] Benchmark shows 3-5× speedup

---

## Success Criteria

✓ Proxy can accept connections in pool config
✓ Connections are reused (5-10 per upstream)
✓ Throughput: 0.90 MB/s → 3-5 MB/s (measured)
✓ Latency: per-request drops from 206ms → 40ms
✓ No regressions in existing functionality
✓ All tests pass
✓ Downstream clients see 3-5× speedup

---

## Files to Modify

```
crates/scred-http/
├── src/
│   ├── lib.rs (add exports)
│   ├── connection_pool.rs (already exists)
│   ├── pooled_dns_resolver.rs (NEW)
│   └── dns_resolver.rs (no changes needed)
crates/scred-proxy/
├── src/
│   └── main.rs (add pool parameter, use pooled resolver)
├── Cargo.toml (no changes)
└── tests/
    └── pooling_test.rs (NEW - integration test)
```

## Effort Estimate
- **Core implementation**: 4-6 hours
- **Testing**: 4-6 hours
- **Integration**: 2-3 hours
- **Benchmarking**: 2-3 hours
- **Total**: 12-18 hours = ~2-3 working days

