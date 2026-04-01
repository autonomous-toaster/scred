use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use crate::PooledDnsResolver;

/// Multi-key pool - one pool per upstream
pub struct MultiUpstreamPool {
    pools: Arc<RwLock<HashMap<String, Arc<PooledDnsResolver>>>>,
}

impl MultiUpstreamPool {
    /// Create new multi-upstream pool
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create pool for upstream address
    pub async fn get_or_create(
        &self,
        upstream_addr: &str,
        max_connections: usize,
    ) -> Arc<PooledDnsResolver> {
        let mut pools = self.pools.write().await;

        if let Some(pool) = pools.get(upstream_addr) {
            debug!("[MultiPool] Reusing pool for {}", upstream_addr);
            return Arc::clone(pool);
        }

        debug!("[MultiPool] Creating new pool for {}", upstream_addr);

        let pool = Arc::new(PooledDnsResolver::new(crate::PoolConfig {
            max_connections,
            idle_timeout_secs: 30,
            enabled: true,
        }));

        pools.insert(upstream_addr.to_string(), Arc::clone(&pool));
        Arc::clone(&pool)
    }

    /// Clear all pools
    pub async fn clear(&self) {
        self.pools.write().await.clear();
    }

    /// Get current upstream count
    pub async fn upstream_count(&self) -> usize {
        self.pools.read().await.len()
    }
}

impl Default for MultiUpstreamPool {
    fn default() -> Self {
        Self::new()
    }
}
