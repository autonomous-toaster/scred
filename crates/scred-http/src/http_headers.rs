/// Phase 3a: HTTP Header Parser (Non-Streaming)
///
/// Parses HTTP headers incrementally without buffering the body.
/// Extracts: Content-Length, Transfer-Encoding, Connection headers.

use anyhow::{anyhow, Result};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

/// Parsed HTTP headers info
#[derive(Debug, Clone)]
pub struct HttpHeaders {
    /// Headers as key-value pairs
    pub headers: Vec<(String, String)>,
    /// Content-Length if present
    pub content_length: Option<usize>,
    /// Transfer-Encoding value if present
    pub transfer_encoding: Option<String>,
    /// Connection header value
    pub connection: Option<String>,
    /// Content-Type header value
    pub content_type: Option<String>,
    /// Full headers text (for forwarding)
    pub raw_headers: String,
}

impl HttpHeaders {
    /// Get a header value (case-insensitive)
    pub fn get(&self, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.clone())
    }

    /// Check if Transfer-Encoding is chunked
    pub fn is_chunked(&self) -> bool {
        self.transfer_encoding
            .as_ref()
            .map(|te| te.to_lowercase().contains("chunked"))
            .unwrap_or(false)
    }

    /// Check if Connection should be kept alive
    pub fn is_keep_alive(&self) -> bool {
        match &self.connection {
            Some(c) => !c.to_lowercase().contains("close"),
            None => true, // Default: keep-alive
        }
    }
}

/// Parse HTTP headers from a reader
///
/// Reads until `\r\n\r\n` (end of headers) is found.
/// Does NOT read the body.
///
/// # Arguments
/// * `reader` - Reader to read headers from
///
/// # Returns
/// Tuple of (parsed_headers, bytes_consumed)
pub async fn parse_http_headers<R: AsyncReadExt + Unpin>(
    reader: &mut BufReader<R>,
) -> Result<HttpHeaders> {
    use tracing::debug;
    
    debug!("[parse_http_headers] ENTRY");
    let mut headers = Vec::new();
    let mut raw_headers = String::new();
    let mut line = String::new();
    let mut line_count = 0;

    // Read headers until blank line
    loop {
        line.clear();
        debug!("[parse_http_headers] Reading line {}...", line_count + 1);
        let n = reader.read_line(&mut line).await?;
        debug!("[parse_http_headers] Read {} bytes", n);

        if n == 0 {
            debug!("[parse_http_headers] ERROR: EOF before end of headers");
            return Err(anyhow!("EOF before end of headers"));
        }

        raw_headers.push_str(&line);

        // Check for end of headers (blank line)
        if line.trim().is_empty() {
            debug!("[parse_http_headers] Found blank line, end of headers");
            break;
        }

        line_count += 1;
        debug!("[parse_http_headers] Line {}: '{}'", line_count, line.trim());

        // Parse header line (key: value)
        if let Some((key, value)) = parse_header_line(&line) {
            debug!("[parse_http_headers] Parsed header: {}={}", key, value);
            headers.push((key, value));
        }
    }

    debug!("[parse_http_headers] Total headers: {}", headers.len());

    // Extract special headers
    let _headers_str = headers
        .iter()
        .map(|(k, v)| format!("{}: {}\r\n", k, v))
        .collect::<Vec<_>>()
        .join("");

    let content_length = headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok());

    let transfer_encoding = headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "transfer-encoding")
        .map(|(_, v)| v.clone());

    let connection = headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "connection")
        .map(|(_, v)| v.clone());

    let content_type = headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "content-type")
        .map(|(_, v)| v.clone());

    Ok(HttpHeaders {
        headers,
        content_length,
        transfer_encoding,
        connection,
        content_type,
        raw_headers,
    })
}

/// Parse a single HTTP header line (e.g., "Content-Type: application/json\r\n")
fn parse_header_line(line: &str) -> Option<(String, String)> {
    let line = line.trim_end_matches('\n').trim_end_matches('\r');
    if line.is_empty() {
        return None;
    }

    match line.split_once(':') {
        Some((key, value)) => Some((key.to_string(), value.trim().to_string())),
        None => None,
    }
}

/// Read exactly N bytes from a reader (for Content-Length bodies)
pub async fn read_exact_body<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    size: usize,
) -> Result<Vec<u8>> {
    let mut body = vec![0u8; size];
    reader.read_exact(&mut body).await?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::io::{BufReader, AsyncReadExt};

    #[tokio::test]
    async fn test_parse_simple_headers() {
        let headers_text = "GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 10\r\n\r\n";
        let cursor = Cursor::new(headers_text.as_bytes());
        let mut reader = BufReader::new(cursor);

        // Skip first line (request line) - caller responsibility
        let mut first_line = String::new();
        reader.read_line(&mut first_line).await.unwrap();

        let headers = parse_http_headers(&mut reader).await.unwrap();

        assert_eq!(headers.get("Host").unwrap(), "example.com");
        assert_eq!(headers.content_length, Some(10));
        assert!(!headers.is_chunked());
        assert!(headers.is_keep_alive());
    }

    #[tokio::test]
    async fn test_parse_chunked_headers() {
        let headers_text = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: keep-alive\r\n\r\n";
        let cursor = Cursor::new(headers_text.as_bytes());
        let mut reader = BufReader::new(cursor);

        let mut first_line = String::new();
        reader.read_line(&mut first_line).await.unwrap();

        let headers = parse_http_headers(&mut reader).await.unwrap();

        assert!(headers.is_chunked());
        assert_eq!(headers.content_length, None);
        assert!(headers.is_keep_alive());
    }

    #[tokio::test]
    async fn test_parse_connection_close() {
        let headers_text = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n";
        let cursor = Cursor::new(headers_text.as_bytes());
        let mut reader = BufReader::new(cursor);

        let mut first_line = String::new();
        reader.read_line(&mut first_line).await.unwrap();

        let headers = parse_http_headers(&mut reader).await.unwrap();

        assert_eq!(headers.content_length, Some(5));
        assert!(!headers.is_keep_alive());
    }
}
