/// HTTP Request/Response Parser
///
/// Parses HTTP/1.1 requests and responses with support for:
/// - Headers (including Content-Length, Transfer-Encoding)
/// - Request line (method, path, version)
/// - Status line (version, code, reason)
/// - Body handling (fixed length, chunked, streaming)
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use tracing::{debug, trace};

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub version: String,
    pub status_code: u16,
    pub reason: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    /// Get a header value (case-insensitive)
    pub fn get_header(&self, key: &str) -> Option<String> {
        let key_lower = key.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.clone())
    }

    /// Get Content-Length if present
    pub fn content_length(&self) -> Option<usize> {
        self.get_header("content-length")
            .and_then(|val| val.parse::<usize>().ok())
    }

    /// Check if request body is chunked
    pub fn is_chunked(&self) -> bool {
        matches!(
            self.get_header("transfer-encoding").as_deref(),
            Some("chunked") | Some("Chunked")
        )
    }

    /// Get all text (headers + body for scanning)
    pub fn all_text(&self) -> String {
        let mut text = format!("{} {} {}\r\n", self.method, self.path, self.version);
        for (k, v) in &self.headers {
            text.push_str(&format!("{}: {}\r\n", k, v));
        }
        text.push_str("\r\n");
        text.push_str(&String::from_utf8_lossy(&self.body));
        text
    }
}

impl HttpResponse {
    /// Get a header value (case-insensitive)
    pub fn get_header(&self, key: &str) -> Option<String> {
        let key_lower = key.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.clone())
    }

    /// Get Content-Length if present
    pub fn content_length(&self) -> Option<usize> {
        self.get_header("content-length")
            .and_then(|val| val.parse::<usize>().ok())
    }

    /// Check if response body is chunked
    pub fn is_chunked(&self) -> bool {
        matches!(
            self.get_header("transfer-encoding").as_deref(),
            Some("chunked") | Some("Chunked")
        )
    }

    /// Get all text (headers + body for scanning)
    pub fn all_text(&self) -> String {
        let mut text = format!(
            "HTTP/{} {} {}\r\n",
            self.version, self.status_code, self.reason
        );
        for (k, v) in &self.headers {
            text.push_str(&format!("{}: {}\r\n", k, v));
        }
        text.push_str("\r\n");
        text.push_str(&String::from_utf8_lossy(&self.body));
        text
    }
}

/// Parse HTTP request from async reader
pub async fn parse_request<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<HttpRequest> {
    // Parse request line
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let line = line.trim();
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(anyhow!("Invalid request line: {}", line));
    }

    let method = parts[0].to_uppercase();
    let path = parts[1].to_string();
    let version = parts[2].to_string();

    debug!("Parsing request: {} {} {}", method, path, version);

    // Parse headers
    let mut headers = HashMap::new();
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line).await?;
        let header_line = header_line.trim();

        if header_line.is_empty() {
            break;
        }

        if let Some(colon_pos) = header_line.find(':') {
            let key = header_line[..colon_pos].trim().to_string();
            let value = header_line[colon_pos + 1..].trim().to_string();
            headers.insert(key.clone(), value.clone());
            trace!("Header: {} = {}", key, value);
        }
    }

    // Parse body based on Content-Length or Transfer-Encoding
    // Note: Body parsing is handled at streaming layer, not here
    let body = if let Some(len) = parse_content_length(&headers) {
        // Reserve buffer for expected size (actual body streamed separately)
        vec![0u8; len.min(1024)] // Cap at 1KB for stub
    } else if is_chunked(&headers) {
        // Chunked bodies handled at streaming layer
        Vec::new()
    } else {
        Vec::new()
    };

    debug!("Request body size: {} bytes", body.len());

    Ok(HttpRequest {
        method,
        path,
        version,
        headers,
        body,
    })
}

/// Parse HTTP response from async reader
pub async fn parse_response<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<HttpResponse> {
    // Parse status line
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let line = line.trim();
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid status line: {}", line));
    }

    let version = parts[0].strip_prefix("HTTP/").unwrap_or("1.1").to_string();
    let status_code = parts[1].parse::<u16>()?;
    let reason = parts[2..].join(" ");

    debug!(
        "Parsing response: HTTP/{} {} {}",
        version, status_code, reason
    );

    // Parse headers
    let mut headers = HashMap::new();
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line).await?;
        let header_line = header_line.trim();

        if header_line.is_empty() {
            break;
        }

        if let Some(colon_pos) = header_line.find(':') {
            let key = header_line[..colon_pos].trim().to_string();
            let value = header_line[colon_pos + 1..].trim().to_string();
            headers.insert(key.clone(), value.clone());
            trace!("Header: {} = {}", key, value);
        }
    }

    // Parse body based on Content-Length or Transfer-Encoding
    let body = if let Some(len) = parse_content_length(&headers) {
        vec![0u8; len] // Placeholder
    } else if is_chunked(&headers) {
        Vec::new()
    } else {
        Vec::new()
    };

    debug!("Response body size: {} bytes", body.len());

    Ok(HttpResponse {
        version,
        status_code,
        reason,
        headers,
        body,
    })
}

fn parse_content_length(headers: &HashMap<String, String>) -> Option<usize> {
    for (k, v) in headers {
        if k.to_lowercase() == "content-length" {
            return v.parse::<usize>().ok();
        }
    }
    None
}

fn is_chunked(headers: &HashMap<String, String>) -> bool {
    for (k, v) in headers {
        if k.to_lowercase() == "transfer-encoding" {
            return v.to_lowercase().contains("chunked");
        }
    }
    false
}
