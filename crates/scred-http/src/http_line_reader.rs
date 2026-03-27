/// HTTP Line Reader - Shared utility for reading HTTP request/response lines
///
/// This module provides utilities for reading HTTP lines byte-by-byte from async streams.
/// Used by both MITM and proxy implementations.
///
/// Features:
/// - Read HTTP request lines (e.g., "GET /path HTTP/1.1")
/// - Read HTTP response lines (e.g., "HTTP/1.1 200 OK")
/// - Handles CRLF line endings (\r\n)
/// - Async-compatible with tokio
/// - Zero-copy line reading

use tokio::io::AsyncReadExt;

/// Read a single HTTP request line from an async reader
///
/// Reads bytes until it encounters \r\n (CRLF) and returns the line without the line ending.
///
/// # Arguments
/// * `reader` - An async reader implementing AsyncReadExt
///
/// # Returns
/// * `Ok(line)` - The HTTP request line (without \r\n)
/// * `Err(UnexpectedEof)` - If EOF reached before finding \r\n
/// * `Err(...)` - Other I/O errors
///
/// # Example
/// ```ignore
/// let line = read_request_line(&mut stream).await?;
/// // line = "GET /anything HTTP/1.1"
/// ```
pub async fn read_request_line<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<String> {
    let mut line = String::new();
    let mut byte = [0u8; 1];

    loop {
        match reader.read_exact(&mut byte).await {
            Ok(0) => {
                // EOF reached
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "EOF while reading request line",
                ));
            }
            Ok(_) => {
                let ch = byte[0] as char;
                if ch == '\n' {
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    return Ok(line);
                }
                line.push(ch);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

/// Read a single HTTP response line from an async reader
///
/// Reads bytes until it encounters \r\n (CRLF) and returns the line without the line ending.
///
/// # Arguments
/// * `reader` - An async reader implementing AsyncReadExt
///
/// # Returns
/// * `Ok(line)` - The HTTP response line (without \r\n)
/// * `Err(UnexpectedEof)` - If EOF reached before finding \r\n
/// * `Err(...)` - Other I/O errors
///
/// # Example
/// ```ignore
/// let line = read_response_line(&mut stream).await?;
/// // line = "HTTP/1.1 200 OK"
/// ```
pub async fn read_response_line<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<String> {
    use tracing::debug;
    
    debug!("[read_response_line] Starting to read response line");
    let mut line = String::new();
    let mut byte = [0u8; 1];
    let mut bytes_read = 0;

    loop {
        match reader.read_exact(&mut byte).await {
            Ok(0) => {
                // EOF reached
                debug!("[read_response_line] EOF reached after {} bytes", bytes_read);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "EOF while reading response line",
                ));
            }
            Ok(_) => {
                bytes_read += 1;
                let ch = byte[0] as char;
                if ch == '\n' {
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    debug!("[read_response_line] Got response line after {} bytes: '{}'", bytes_read, line);
                    return Ok(line);
                }
                line.push(ch);
            }
            Err(e) => {
                debug!("[read_response_line] Error reading after {} bytes: {}", bytes_read, e);
                return Err(e);
            }
        }
    }
}

