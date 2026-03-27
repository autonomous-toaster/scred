//! Safe HTTP response reading with proper buffer management
//!
//! This module provides a unified way to read HTTP responses with:
//! - No uninitialized buffer garbage
//! - Proper truncation and memory safety
//! - Support for chunked/pipelined responses

use anyhow::Result;
use tokio::io::AsyncReadExt;
use std::time::Duration;

/// Read complete HTTP response from an async reader with timeout
/// 
/// # Benefits
/// - Starts with empty Vec, only extends with valid data
/// - No NUL-byte garbage from pre-allocated buffers
/// - Handles chunked responses with timeout
/// - Safe default: 100ms read timeout
pub async fn read_http_response<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    timeout_ms: u64,
) -> Result<Vec<u8>> {
    let mut response_data = Vec::new();
    let mut first_chunk = vec![0u8; 65536];
    
    // Read first chunk
    let n = reader.read(&mut first_chunk).await?;
    if n == 0 {
        return Ok(Vec::new()); // Empty response
    }
    
    first_chunk.truncate(n);
    response_data.extend_from_slice(&first_chunk);
    
    // Read additional chunks until EOF or timeout
    let timeout = Duration::from_millis(timeout_ms);
    loop {
        let mut chunk = vec![0u8; 65536];
        match tokio::time::timeout(timeout, reader.read(&mut chunk)).await {
            Ok(Ok(0)) => {
                // EOF - connection closed
                break;
            }
            Ok(Ok(n)) => {
                // Got more data
                chunk.truncate(n);
                response_data.extend_from_slice(&chunk);
            }
            Ok(Err(e)) => {
                // Read error
                return Err(e.into());
            }
            Err(_) => {
                // Timeout - no more data coming
                break;
            }
        }
    }
    
    Ok(response_data)
}

