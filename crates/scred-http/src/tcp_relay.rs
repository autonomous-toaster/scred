//! Generic TCP relay with optional redaction
//!
//! Provides traits and utilities for building TCP proxies with request/response redaction.

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use anyhow::Result;
use tracing::debug;

/// Generic trait for TCP relay handlers
pub trait TcpRelayHandler: Send + Sync {
    /// Process inbound data (from client)
    fn process_inbound(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Process outbound data (from upstream)
    fn process_outbound(&self, data: &[u8]) -> Result<Vec<u8>>;
}

/// Simple TCP relay: accept connection, forward to upstream, relay responses back
pub async fn relay_tcp_bidirectional(
    mut client_socket: TcpStream,
    mut upstream_socket: TcpStream,
    handler: Option<Arc<dyn TcpRelayHandler>>,
) -> Result<()> {
    let mut buf_client = vec![0u8; 65536];
    let mut buf_upstream = vec![0u8; 65536];
    
    loop {
        tokio::select! {
            // Client → Upstream
            result = client_socket.read(&mut buf_client) => {
                match result? {
                    0 => {
                        debug!("Client connection closed");
                        return Ok(());
                    }
                    n => {
                        let mut data = buf_client[..n].to_vec();
                        
                        // Process if handler provided
                        if let Some(h) = &handler {
                            data = h.process_inbound(&data)?;
                        }
                        
                        upstream_socket.write_all(&data).await?;
                        upstream_socket.flush().await?;
                    }
                }
            }
            
            // Upstream → Client
            result = upstream_socket.read(&mut buf_upstream) => {
                match result? {
                    0 => {
                        debug!("Upstream connection closed");
                        return Ok(());
                    }
                    n => {
                        let mut data = buf_upstream[..n].to_vec();
                        
                        // Process if handler provided
                        if let Some(h) = &handler {
                            data = h.process_outbound(&data)?;
                        }
                        
                        client_socket.write_all(&data).await?;
                        client_socket.flush().await?;
                    }
                }
            }
        }
    }
}
