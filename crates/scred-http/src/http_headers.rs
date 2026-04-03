use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, BufReader};

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

/// FAST: Parse HTTP headers without line-by-line overhead
///
/// Reads raw bytes until \r\n\r\n is found, then parses.
/// IMPORTANT: Reads byte-by-byte to avoid buffering body data.
///
/// # Arguments
/// * `reader` - Async reader
/// * `has_status_line` - true for response headers (first line is status line like "HTTP/1.1 200 OK")
///                       false for request headers (all lines are headers)
pub async fn parse_http_headers<R: AsyncReadExt + Unpin>(
    reader: &mut BufReader<R>,
    has_status_line: bool,
) -> Result<HttpHeaders> {
    // Read headers byte-by-byte to avoid buffering body
    let mut header_buffer = Vec::with_capacity(8192);
    let mut byte = [0u8; 1];
    let end_marker = b"\r\n\r\n";
    let max_header_size = 256 * 1024; // 256KB max headers

    loop {
        let n = reader.read(&mut byte).await?;
        if n == 0 {
            return Err(anyhow!("EOF before end of headers"));
        }
        header_buffer.push(byte[0]);

        // Check if we have the end marker
        if header_buffer.len() >= 4 {
            let len = header_buffer.len();
            if &header_buffer[len - 4..] == end_marker {
                // Found end of headers, parse them
                let headers_text = String::from_utf8_lossy(&header_buffer);
                return parse_headers_from_text(&headers_text, has_status_line);
            }
        }

        // Safety check: prevent reading forever
        if header_buffer.len() > max_header_size {
            return Err(anyhow!("Headers too large (>256KB)"));
        }
    }
}

/// Parse headers from raw text
fn parse_headers_from_text(text: &str, has_status_line: bool) -> Result<HttpHeaders> {
    let mut headers = Vec::new();
    let lines: Vec<&str> = text.lines().collect();

    // Skip first line only if it's a status line (for response headers)
    // For request headers, the first line IS a header
    let skip_first = if has_status_line { 1 } else { 0 };

    // Parse header lines
    for line in lines.iter().skip(skip_first) {
        if line.trim().is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            headers.push((key, value));
        }
    }

    // Extract special headers
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

    // raw_headers: only the header lines, not the status line
    // HTTP headers must end with blank line (\r\n\r\n)
    let raw_headers = lines
        .iter()
        .skip(skip_first)
        .take_while(|line| !line.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\r\n")
        + "\r\n\r\n"; // End of headers marker

    Ok(HttpHeaders {
        headers,
        content_length,
        transfer_encoding,
        connection,
        content_type,
        raw_headers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_response_headers() {
        let raw = "HTTP/1.1 200 OK\r\nContent-Length: 100\r\nContent-Type: text/plain\r\n\r\n";
        let mut reader = BufReader::new(raw.as_bytes());
        let headers = parse_http_headers(&mut reader, true).await.unwrap();
        assert_eq!(headers.content_length, Some(100));
        assert_eq!(headers.content_type, Some("text/plain".to_string()));
    }

    #[tokio::test]
    async fn test_parse_request_headers() {
        let raw = "Host: example.com\r\nUser-Agent: test\r\nAccept: */*\r\n\r\n";
        let mut reader = BufReader::new(raw.as_bytes());
        let headers = parse_http_headers(&mut reader, false).await.unwrap();
        assert_eq!(headers.get("host"), Some("example.com".to_string()));
        assert_eq!(headers.get("user-agent"), Some("test".to_string()));
        assert!(headers.raw_headers.contains("Host:"));
    }

    #[tokio::test]
    async fn test_parse_chunked_headers() {
        let raw = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n";
        let mut reader = BufReader::new(raw.as_bytes());
        let headers = parse_http_headers(&mut reader, true).await.unwrap();
        assert!(headers.is_chunked());
    }

    #[tokio::test]
    async fn test_headers_with_body_data() {
        // Headers + body in one buffer - should only parse headers
        let raw = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
        let mut reader = BufReader::new(raw.as_bytes());
        let headers = parse_http_headers(&mut reader, true).await.unwrap();
        assert_eq!(headers.content_length, Some(5));
        // The body "hello" should still be available in the reader
        let mut body = String::new();
        reader.read_to_string(&mut body).await.unwrap();
        assert_eq!(body, "hello");
    }
}
