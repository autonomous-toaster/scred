/// Upstream HTTP/2 Connection Pool
///
/// Manages HTTP/2 connections to upstream servers, pooling by hostname
/// to enable connection reuse across multiple streams.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// HTTP/2 upstream connection (placeholder)
///
/// In a full implementation, this would be an http2::SendRequest.
/// For now, we use a simple connection identifier.
#[derive(Clone, Debug)]
pub struct UpstreamH2Connection {
    /// Connection ID (for tracking)
    id: u32,
    
    /// Remote hostname
    hostname: String,
    
    /// Port
    port: u16,
    
    /// Active streams on this connection
    active_streams: Arc<Mutex<u32>>,
}

impl UpstreamH2Connection {
    /// Create new upstream connection
    pub fn new(id: u32, hostname: String, port: u16) -> Self {
        Self {
            id,
            hostname,
            port,
            active_streams: Arc::new(Mutex::new(0)),
        }
    }

    /// Get connection identifier
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get hostname
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Get port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get active stream count
    pub async fn active_stream_count(&self) -> u32 {
        *self.active_streams.lock().await
    }

    /// Increment active streams (stream created)
    pub async fn increment_streams(&self) {
        let mut count = self.active_streams.lock().await;
        *count += 1;
    }

    /// Decrement active streams (stream closed)
    pub async fn decrement_streams(&self) {
        let mut count = self.active_streams.lock().await;
        *count = count.saturating_sub(1);
    }
}

/// Upstream HTTP/2 connection pool
///
/// Manages connections per hostname. Multiple streams can share a single
/// HTTP/2 connection to the upstream server.
pub struct UpstreamH2Pool {
    /// Pools by hostname: HashMap<hostname, Vec<connection>>
    pools: Arc<Mutex<HashMap<String, Vec<UpstreamH2Connection>>>>,
    
    /// Connection ID counter
    next_conn_id: Arc<Mutex<u32>>,
    
    /// Maximum connections per host
    max_connections_per_host: u32,
    
    /// Maximum streams per connection
    max_streams_per_connection: u32,
}

impl UpstreamH2Pool {
    /// Create new upstream connection pool
    pub fn new() -> Self {
        Self {
            pools: Arc::new(Mutex::new(HashMap::new())),
            next_conn_id: Arc::new(Mutex::new(0)),
            max_connections_per_host: 4,
            max_streams_per_connection: 100,
        }
    }

    /// Get or create connection to upstream host
    ///
    /// Reuses existing connection if available, creates new one if needed.
    pub async fn get_or_create(
        &self,
        hostname: &str,
        port: u16,
    ) -> Result<UpstreamH2Connection> {
        let mut pools = self.pools.lock().await;
        
        let key = format!("{}:{}", hostname, port);
        
        // Try to reuse existing connection
        if let Some(conns) = pools.get_mut(&key) {
            for conn in conns.iter() {
                let stream_count = conn.active_stream_count().await;
                if stream_count < self.max_streams_per_connection {
                    debug!(
                        "UpstreamH2Pool: Reusing connection {} to {}:{} ({} active streams)",
                        conn.id(),
                        hostname,
                        port,
                        stream_count
                    );
                    return Ok(conn.clone());
                }
            }
            
            // All connections saturated, create new one if possible
            if conns.len() < self.max_connections_per_host as usize {
                let conn_id = {
                    let mut id = self.next_conn_id.lock().await;
                    let current = *id;
                    *id += 1;
                    current
                };
                
                let new_conn = UpstreamH2Connection::new(
                    conn_id,
                    hostname.to_string(),
                    port,
                );
                
                info!(
                    "UpstreamH2Pool: Created new connection {} to {}:{}",
                    conn_id, hostname, port
                );
                
                conns.push(new_conn.clone());
                return Ok(new_conn);
            } else {
                return Err(anyhow!(
                    "UpstreamH2Pool: Max connections reached for {}:{}",
                    hostname,
                    port
                ));
            }
        }
        
        // No pool exists for this host, create new one with first connection
        let conn_id = {
            let mut id = self.next_conn_id.lock().await;
            let current = *id;
            *id += 1;
            current
        };
        
        let new_conn = UpstreamH2Connection::new(
            conn_id,
            hostname.to_string(),
            port,
        );
        
        info!(
            "UpstreamH2Pool: Created new connection pool for {}:{}, connection {}",
            hostname, port, conn_id
        );
        
        pools.insert(key, vec![new_conn.clone()]);
        Ok(new_conn)
    }

    /// Get pool statistics
    pub async fn stats(&self) -> UpstreamH2PoolStats {
        let pools = self.pools.lock().await;
        
        let mut total_connections = 0;
        let mut total_streams = 0;
        let mut hosts = Vec::new();
        
        for (host, conns) in pools.iter() {
            total_connections += conns.len();
            
            let mut host_streams = 0;
            for conn in conns {
                let streams = conn.active_stream_count().await;
                host_streams += streams;
                total_streams += streams;
            }
            
            hosts.push((host.clone(), conns.len(), host_streams));
        }
        
        UpstreamH2PoolStats {
            total_hosts: hosts.len(),
            total_connections,
            total_active_streams: total_streams as u32,
            hosts,
        }
    }

    /// Close and remove a connection
    pub async fn close_connection(&self, hostname: &str, port: u16, conn_id: u32) -> Result<()> {
        let mut pools = self.pools.lock().await;
        
        let key = format!("{}:{}", hostname, port);
        
        if let Some(conns) = pools.get_mut(&key) {
            if let Some(pos) = conns.iter().position(|c| c.id() == conn_id) {
                conns.remove(pos);
                debug!(
                    "UpstreamH2Pool: Closed connection {} to {}:{}",
                    conn_id, hostname, port
                );
                
                // Remove empty pool
                if conns.is_empty() {
                    pools.remove(&key);
                }
                
                return Ok(());
            }
        }
        
        warn!(
            "UpstreamH2Pool: Connection {} not found for {}:{}",
            conn_id, hostname, port
        );
        Ok(())
    }
}

impl Default for UpstreamH2Pool {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for upstream connection pool
#[derive(Debug, Clone)]
pub struct UpstreamH2PoolStats {
    /// Total number of hosts
    pub total_hosts: usize,
    
    /// Total number of connections
    pub total_connections: usize,
    
    /// Total active streams across all connections
    pub total_active_streams: u32,
    
    /// Per-host statistics: (hostname, connection_count, active_stream_count)
    pub hosts: Vec<(String, usize, u32)>,
}

impl std::fmt::Display for UpstreamH2PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UpstreamH2Pool: {} hosts, {} connections, {} active streams",
            self.total_hosts, self.total_connections, self.total_active_streams
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_creation() {
        let pool = UpstreamH2Pool::new();
        let conn = pool
            .get_or_create("example.com", 443)
            .await
            .unwrap();

        assert_eq!(conn.hostname(), "example.com");
        assert_eq!(conn.port(), 443);
    }

    #[tokio::test]
    async fn test_connection_reuse() {
        let pool = UpstreamH2Pool::new();
        
        let conn1 = pool
            .get_or_create("example.com", 443)
            .await
            .unwrap();
        
        let conn2 = pool
            .get_or_create("example.com", 443)
            .await
            .unwrap();

        // Should return same connection
        assert_eq!(conn1.id(), conn2.id());
    }

    #[tokio::test]
    async fn test_different_hosts_separate_connections() {
        let pool = UpstreamH2Pool::new();
        
        let conn1 = pool
            .get_or_create("example.com", 443)
            .await
            .unwrap();
        
        let conn2 = pool
            .get_or_create("other.com", 443)
            .await
            .unwrap();

        // Should have different connections
        assert_ne!(conn1.id(), conn2.id());
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let pool = UpstreamH2Pool::new();
        
        pool.get_or_create("example.com", 443).await.ok();
        pool.get_or_create("other.com", 443).await.ok();
        
        let stats = pool.stats().await;
        assert_eq!(stats.total_hosts, 2);
        assert_eq!(stats.total_connections, 2);
    }

    #[tokio::test]
    async fn test_stream_counting() {
        let pool = UpstreamH2Pool::new();
        
        let conn = pool
            .get_or_create("example.com", 443)
            .await
            .unwrap();

        assert_eq!(conn.active_stream_count().await, 0);
        
        conn.increment_streams().await;
        assert_eq!(conn.active_stream_count().await, 1);
        
        conn.increment_streams().await;
        assert_eq!(conn.active_stream_count().await, 2);
        
        conn.decrement_streams().await;
        assert_eq!(conn.active_stream_count().await, 1);
    }

    #[tokio::test]
    async fn test_close_connection() {
        let pool = UpstreamH2Pool::new();
        
        let conn = pool
            .get_or_create("example.com", 443)
            .await
            .unwrap();
        
        let conn_id = conn.id();
        
        pool.close_connection("example.com", 443, conn_id)
            .await
            .ok();
        
        let stats = pool.stats().await;
        assert_eq!(stats.total_connections, 0);
    }
}
