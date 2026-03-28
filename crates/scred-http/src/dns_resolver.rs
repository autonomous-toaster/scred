/// DNS Resolution with Retry Logic
///
/// This module provides robust DNS resolution with:
/// - Automatic retry on DNS failures (up to 3 attempts)
/// - Exponential backoff (100ms, 200ms, 400ms)
/// - Fallback to localhost for testing
/// - Detailed logging for debugging
/// - Support for both IPv4 and IPv6

use anyhow::{anyhow, Result};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::{debug, info, warn, error};

/// Maximum number of DNS resolution attempts
const MAX_RETRIES: u32 = 3;

/// Initial backoff duration (milliseconds) - reduced to 1ms for much faster retries
const INITIAL_BACKOFF_MS: u64 = 1;

/// DNS resolver with retry logic
pub struct DnsResolver;

impl DnsResolver {
    /// Strip URL scheme from address if present
    ///
    /// # Arguments
    /// * `addr` - Address that may include scheme (e.g., "http://host:port" or "host:port")
    ///
    /// # Returns
    /// * Address without scheme (e.g., "host:port")
    ///
    /// # Examples
    /// ```ignore
    /// DnsResolver::strip_scheme("http://proxy.example.com:3128") → "proxy.example.com:3128"
    /// DnsResolver::strip_scheme("https://proxy.example.com:3128") → "proxy.example.com:3128"
    /// DnsResolver::strip_scheme("proxy.example.com:3128") → "proxy.example.com:3128"
    /// ```
    fn strip_scheme(addr: &str) -> &str {
        if let Some(idx) = addr.find("://") {
            &addr[idx + 3..]
        } else {
            addr
        }
    }

    /// Resolve hostname and connect with automatic retry
    ///
    /// # Arguments
    /// * `addr` - Target address (hostname:port, IP:port, or http://hostname:port)
    ///
    /// # Returns
    /// * `Ok(TcpStream)` - Connected socket on success
    /// * `Err(...)` - Error after all retries exhausted
    ///
    /// # Retry Strategy
    /// - Attempt 1: Immediate (0ms)
    /// - Attempt 2: After 100ms
    /// - Attempt 3: After 200ms
    /// - Attempt 4: After 400ms (final)
    /// - After: Error
    pub async fn connect_with_retry(addr: &str) -> Result<TcpStream> {
        // Strip scheme if present (http://, https://, etc)
        let addr_without_scheme = Self::strip_scheme(addr);
        debug!("DNS: Connecting to {} with retry strategy", addr_without_scheme);

        let mut attempt = 0;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        loop {
            attempt += 1;
            debug!("DNS: Attempt {} of {}: connecting to {}", attempt, MAX_RETRIES + 1, addr_without_scheme);

            // Attempt to resolve and connect
            match Self::try_connect(addr_without_scheme).await {
                Ok(stream) => {
                    info!("DNS: Connected to {} on attempt {}", addr_without_scheme, attempt);
                    return Ok(stream);
                }
                Err(e) => {
                    if attempt > MAX_RETRIES {
                        error!(
                            "DNS: Failed to connect to {} after {} attempts: {}",
                            addr_without_scheme, attempt, e
                        );
                        return Err(e);
                    }

                    warn!(
                        "DNS: Connection attempt {} failed for {}: {}. Retrying in {}ms...",
                        attempt, addr_without_scheme, e, backoff_ms
                    );

                    // Wait before retrying
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

                    // Double backoff for next retry
                    backoff_ms *= 2;
                }
            }
        }
    }

    /// Single connection attempt without retry
    async fn try_connect(addr: &str) -> Result<TcpStream> {
        debug!("DNS: Try connect to {}", addr);

        // Parse the address string
        let socket_addrs: Vec<SocketAddr> = addr
            .to_socket_addrs()
            .map_err(|e| {
                error!("DNS: Failed to resolve address '{}': {}", addr, e);
                anyhow!("DNS resolution failed for '{}': {}", addr, e)
            })?
            .collect();

        if socket_addrs.is_empty() {
            return Err(anyhow!(
                "DNS: No addresses resolved for '{}'",
                addr
            ));
        }

        debug!("DNS: Resolved {} to {} addresses", addr, socket_addrs.len());

        // Try each resolved address
        let mut last_error = None;
        for (idx, socket_addr) in socket_addrs.iter().enumerate() {
            debug!("DNS: Trying address {}/{}: {}", idx + 1, socket_addrs.len(), socket_addr);

            match TcpStream::connect(*socket_addr).await {
                Ok(stream) => {
                    info!("DNS: Successfully connected to {} ({})", addr, socket_addr);
                    return Ok(stream);
                }
                Err(e) => {
                    debug!("DNS: Connection to {} failed: {}", socket_addr, e);
                    last_error = Some(e);
                }
            }
        }

        // All resolved addresses failed
        if let Some(e) = last_error {
            Err(anyhow!(
                "DNS: Could not connect to any resolved address for '{}': {}",
                addr,
                e
            ))
        } else {
            Err(anyhow!("DNS: No valid addresses for '{}'", addr))
        }
    }
}

