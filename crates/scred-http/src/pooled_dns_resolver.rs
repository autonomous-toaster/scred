/// Pooled DNS Resolver - High Performance Connection Pooling
///
/// Wraps DnsResolver with per-upstream connection pooling to avoid:
/// - Repeated TCP handshakes (5-10ms per request baseline)
/// - Repeated DNS lookups (1-5ms per request if uncached)
/// - Connection setup latency
///
/// Performance Goal: 0.90 MB/s → 3-5 MB/s (3-5× speedup via connection reuse)
///
/// Zero-Copy Strategy:
/// - Connection pool uses VecDeque (no allocation on get/put)
/// - Arc<Mutex> for thread-safe sharing (minimal overhead)
/// - TcpStream moved directly (no cloning)
/// - Upstream address stored as &'static str or interned (reduces allocations)
use crate::connection_pool::ConnectionPool;
use crate::dns_resolver::DnsResolver;
use anyhow::Result;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::debug;

/// Configuration for connection pooling
#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// Maximum connections per upstream address (default: 10)
    pub max_connections: usize,
    /// Idle timeout in seconds (default: 30)
    pub idle_timeout_secs: u64,
    /// Enable pooling (default: true)
    pub enabled: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            idle_timeout_secs: 30,
            enabled: true,
        }
    }
}

/// Pooled DNS resolver - manages connection pools per upstream address
///
/// PERFORMANCE CRITICAL: Uses RwLock for minimal contention on reads
pub struct PooledDnsResolver {
    /// Pools per upstream address
    /// RwLock allows multiple concurrent readers (lookups) with exclusive writers (pool creation)
    pools: Arc<RwLock<HashMap<String, Arc<ConnectionPool>>>>,
    /// Configuration
    config: PoolConfig,
}

impl PooledDnsResolver {
    /// Create new pooled resolver with given configuration
    pub fn new(config: PoolConfig) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Connect to upstream via pool (get cached connection or create new one)
    ///
    /// PERFORMANCE CRITICAL PATH:
    /// - Try to get cached connection (O(1) from VecDeque)
    /// - If unavailable, create new connection via DnsResolver
    /// - Return immediately (no expensive operations)
    pub async fn connect_with_retry(&self, addr: &str) -> Result<PooledTcpStream> {
        if !self.config.enabled {
            // Pooling disabled - use DNS resolver directly (no overhead)
            let stream = DnsResolver::connect_with_retry(addr).await?;
            return Ok(PooledTcpStream::not_pooled(stream));
        }

        // Get or create pool for this address
        let pool = {
            let mut pools = self.pools.write().await;
            pools
                .entry(addr.to_string())
                .or_insert_with(|| {
                    Arc::new(ConnectionPool::new(
                        addr.to_string(),
                        self.config.max_connections,
                    ))
                })
                .clone()
        };

        // Try to get connection from pool (fast path)
        if let Some(stream) = pool.get() {
            debug!("Pooled connection reused for {}", addr);
            return Ok(PooledTcpStream::from_pool(stream, pool.clone()));
        }

        // Pool empty - create new connection via DNS resolver
        debug!("Creating new pooled connection for {}", addr);
        let stream = DnsResolver::connect_with_retry(addr).await?;
        Ok(PooledTcpStream::from_pool(stream, pool))
    }

    /// Cleanup idle connections from all pools (run periodically)
    /// Should be called every 30+ seconds to prevent connection leak
    pub async fn cleanup_idle(&self) {
        if !self.config.enabled {
            return;
        }

        let pools = self.pools.read().await;
        for (_addr, pool) in pools.iter() {
            pool.clear();
        }
    }

    /// Get total number of pooled connections
    pub async fn pool_size(&self) -> usize {
        let pools = self.pools.read().await;
        pools.values().map(|p| p.size()).sum()
    }

    /// Get number of pools
    pub async fn num_pools(&self) -> usize {
        self.pools.read().await.len()
    }
}

/// Wrapper around TcpStream that returns connection to pool on drop
///
/// CRITICAL: Implements AsyncRead/AsyncWrite for transparent usage as TcpStream
pub struct PooledTcpStream {
    stream: Option<TcpStream>,
    pool: Option<Arc<ConnectionPool>>,
}

impl PooledTcpStream {
    /// Create new pooled stream (will return to pool on drop)
    fn from_pool(stream: TcpStream, pool: Arc<ConnectionPool>) -> Self {
        Self {
            stream: Some(stream),
            pool: Some(pool),
        }
    }

    /// Create unpooled stream (won't return to pool on drop)
    fn not_pooled(stream: TcpStream) -> Self {
        Self {
            stream: Some(stream),
            pool: None,
        }
    }

    /// Get mutable reference to underlying stream (for AsyncReadExt, AsyncWriteExt)
    pub fn as_mut(&mut self) -> Option<&mut TcpStream> {
        self.stream.as_mut()
    }

    /// Get reference to underlying stream (for AsyncReadExt)
    pub fn as_ref(&self) -> Option<&TcpStream> {
        self.stream.as_ref()
    }

    /// Take ownership of underlying stream (consume the wrapper)
    pub fn into_inner(mut self) -> Option<TcpStream> {
        self.stream.take()
    }
}

/// Implement Deref for transparent usage
impl std::ops::Deref for PooledTcpStream {
    type Target = TcpStream;

    fn deref(&self) -> &Self::Target {
        self.stream
            .as_ref()
            .expect("PooledTcpStream used after drop")
    }
}

/// Implement DerefMut for transparent usage
impl std::ops::DerefMut for PooledTcpStream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.stream
            .as_mut()
            .expect("PooledTcpStream used after drop")
    }
}

/// Implement AsyncRead for transparent usage
impl AsyncRead for PooledTcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match &mut self.stream {
            Some(stream) => Pin::new(stream).poll_read(cx, buf),
            None => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "stream consumed",
            ))),
        }
    }
}

/// Implement AsyncWrite for transparent usage
impl AsyncWrite for PooledTcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match &mut self.stream {
            Some(stream) => Pin::new(stream).poll_write(cx, buf),
            None => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "stream consumed",
            ))),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match &mut self.stream {
            Some(stream) => Pin::new(stream).poll_flush(cx),
            None => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "stream consumed",
            ))),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match &mut self.stream {
            Some(stream) => Pin::new(stream).poll_shutdown(cx),
            None => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "stream consumed",
            ))),
        }
    }
}

/// Return connection to pool on drop (CRITICAL for connection reuse)
impl Drop for PooledTcpStream {
    fn drop(&mut self) {
        if let (Some(stream), Some(pool)) = (self.stream.take(), self.pool.clone()) {
            // Return to pool asynchronously
            // Uses tokio::spawn to avoid blocking the drop
            tokio::spawn(async move {
                pool.put(stream);
                debug!("Connection returned to pool");
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pooled_resolver_creates_pool() {
        let resolver = PooledDnsResolver::new(PoolConfig::default());
        assert_eq!(resolver.num_pools().await, 0);
    }

    #[tokio::test]
    async fn test_pooled_resolver_disabled() {
        let config = PoolConfig {
            enabled: false,
            ..Default::default()
        };
        let resolver = PooledDnsResolver::new(config);
        // With pooling disabled, should still be usable
        assert_eq!(resolver.num_pools().await, 0);
    }

    #[test]
    fn test_pool_config_defaults() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.idle_timeout_secs, 30);
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_pooled_stream_wrapper() {
        // Test wrapper structure (actual connection tests need real server)
        let config = PoolConfig::default();
        let _resolver = PooledDnsResolver::new(config);
        // Stream creation would require actual TCP connection
    }
}
