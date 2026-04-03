/// Simple HTTP connection pool for upstream proxies
/// Reuses TCP connections with Keep-Alive to avoid repeated DNS/TCP handshakes
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;

/// Pooled connection with timeout
struct PooledConnection {
    stream: TcpStream,
    last_used: Instant,
}

impl PooledConnection {
    fn is_idle_timeout(&self, max_idle: Duration) -> bool {
        self.last_used.elapsed() > max_idle
    }
}

/// Connection pool for a single upstream address
pub struct ConnectionPool {
    pool: Arc<Mutex<VecDeque<PooledConnection>>>,
    addr: String,
    max_connections: usize,
    max_idle_time: Duration,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(addr: String, max_connections: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(VecDeque::with_capacity(max_connections))),
            addr,
            max_connections,
            max_idle_time: Duration::from_secs(30), // 30 second timeout
        }
    }

    /// Try to get a reusable connection from the pool
    pub fn get(&self) -> Option<TcpStream> {
        let mut pool = self.pool.lock().unwrap();

        // Remove idle connections
        pool.retain(|conn| !conn.is_idle_timeout(self.max_idle_time));

        // Try to get a connection
        if let Some(conn) = pool.pop_front() {
            // Verify connection is still alive with a simple check
            // In a real implementation, we'd test with a TCP_KEEPALIVE option
            return Some(conn.stream);
        }

        None
    }

    /// Return a connection to the pool for reuse
    pub fn put(&self, stream: TcpStream) {
        let mut pool = self.pool.lock().unwrap();

        // Only keep if pool not full
        if pool.len() < self.max_connections {
            pool.push_back(PooledConnection {
                stream,
                last_used: Instant::now(),
            });
        }
    }

    /// Clear all connections
    pub fn clear(&self) {
        self.pool.lock().unwrap().clear();
    }

    /// Get pool size
    pub fn size(&self) -> usize {
        self.pool.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_basic() {
        let pool = ConnectionPool::new("127.0.0.1:8080".to_string(), 3);

        // Pool should be empty initially
        assert_eq!(pool.size(), 0);
        assert!(pool.get().is_none());
    }

    #[tokio::test]
    async fn test_connection_pool_lifecycle() {
        let pool = ConnectionPool::new("127.0.0.1:8080".to_string(), 3);

        // Create a mock stream (would be a real TcpStream in production)
        // For now, just test the logic
        assert_eq!(pool.size(), 0);

        pool.clear();
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_connection_pool_capacity() {
        let pool = ConnectionPool::new("127.0.0.1:8080".to_string(), 2);
        assert_eq!(pool.max_connections, 2);
    }
}
