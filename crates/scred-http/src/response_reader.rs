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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_single_read() {
        let data = b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nHello World";
        let mut reader = Cursor::new(data.to_vec());
        
        let response = read_http_response(&mut reader, 100).await.unwrap();
        assert_eq!(response, data);
    }

    #[tokio::test]
    async fn test_no_garbage_bytes() {
        // Verify no NUL padding
        let data = b"OK";
        let mut reader = Cursor::new(data.to_vec());
        
        let response = read_http_response(&mut reader, 100).await.unwrap();
        assert_eq!(response.len(), 2);
        assert_eq!(response, data);
        // Would fail with vec![0u8; 65536] approach
    }
}
