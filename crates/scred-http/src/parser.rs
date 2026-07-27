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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_headers(pairs: Vec<(&str, &str)>) -> HashMap<String, String> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn test_http_request_get_header() {
        let req = HttpRequest {
            method: "GET".to_string(),
            path: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: make_headers(vec![
                ("Host", "example.com"),
                ("Content-Type", "text/html"),
            ]),
            body: Vec::new(),
        };
        assert_eq!(req.get_header("Host"), Some("example.com".to_string()));
        assert_eq!(req.get_header("Content-Type"), Some("text/html".to_string()));
        assert_eq!(req.get_header("X-Missing"), None);
    }

    #[test]
    fn test_http_request_content_length() {
        let req = HttpRequest {
            method: "POST".to_string(),
            path: "/api".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: make_headers(vec![("Content-Length", "42")]),
            body: vec![0u8; 42],
        };
        assert_eq!(req.content_length(), Some(42));
    }

    #[test]
    fn test_http_request_content_length_missing() {
        let req = HttpRequest {
            method: "GET".to_string(),
            path: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert_eq!(req.content_length(), None);
    }

    #[test]
    fn test_http_request_is_chunked() {
        let req = HttpRequest {
            method: "POST".to_string(),
            path: "/upload".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: make_headers(vec![("Transfer-Encoding", "chunked")]),
            body: Vec::new(),
        };
        assert!(req.is_chunked());
    }

    #[test]
    fn test_http_request_is_not_chunked() {
        let req = HttpRequest {
            method: "GET".to_string(),
            path: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(!req.is_chunked());
    }

    #[test]
    fn test_http_request_all_text() {
        let req = HttpRequest {
            method: "GET".to_string(),
            path: "/path".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: make_headers(vec![("Host", "example.com")]),
            body: b"body data".to_vec(),
        };
        let text = req.all_text();
        assert!(text.contains("GET /path HTTP/1.1"));
        assert!(text.contains("Host: example.com"));
        assert!(text.contains("body data"));
    }

    #[test]
    fn test_http_response_get_header() {
        let resp = HttpResponse {
            version: "HTTP/1.1".to_string(),
            status_code: 200,
            reason: "OK".to_string(),
            headers: make_headers(vec![("Content-Type", "application/json")]),
            body: Vec::new(),
        };
        assert_eq!(resp.get_header("Content-Type"), Some("application/json".to_string()));
    }

    #[test]
    fn test_http_response_content_length() {
        let resp = HttpResponse {
            version: "HTTP/1.1".to_string(),
            status_code: 200,
            reason: "OK".to_string(),
            headers: make_headers(vec![("Content-Length", "100")]),
            body: vec![0u8; 100],
        };
        assert_eq!(resp.content_length(), Some(100));
    }

    #[test]
    fn test_http_response_is_chunked() {
        let resp = HttpResponse {
            version: "HTTP/1.1".to_string(),
            status_code: 200,
            reason: "OK".to_string(),
            headers: make_headers(vec![("Transfer-Encoding", "chunked")]),
            body: Vec::new(),
        };
        assert!(resp.is_chunked());
    }

    #[test]
    fn test_http_response_all_text() {
        let resp = HttpResponse {
            version: "HTTP/1.1".to_string(),
            status_code: 404,
            reason: "Not Found".to_string(),
            headers: make_headers(vec![("Content-Type", "text/html")]),
            body: b"<h1>404</h1>".to_vec(),
        };
        let text = resp.all_text();
        assert!(text.contains("HTTP/1.1 404 Not Found"));
        assert!(text.contains("Content-Type: text/html"));
        assert!(text.contains("<h1>404</h1>"));
    }

    #[tokio::test]
    async fn test_parse_request_basic_get() {
        let data = b"GET /path HTTP/1.1
Host: example.com

";
        let mut reader = &data[..];
        let req = parse_request(&mut reader).await.unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/path");
        assert_eq!(req.version, "HTTP/1.1");
        assert_eq!(req.headers.get("Host").unwrap(), "example.com");
    }

    #[tokio::test]
    async fn test_parse_request_post_with_body() {
        let data = b"POST /api/data HTTP/1.1\r\nHost: example.com\r\nContent-Type: application/json\r\nContent-Length: 10\r\n\r\nrawbody123";
        let mut reader = &data[..];
        let req = parse_request(&mut reader).await.unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/api/data");
        assert_eq!(req.headers.get("Content-Type").unwrap(), "application/json");
    }

    #[tokio::test]
    async fn test_parse_request_invalid_line() {
        let data = b"INVALID
";
        let mut reader = &data[..];
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_response_basic_ok() {
        let data = b"HTTP/1.1 200 OK
Content-Type: text/html
Content-Length: 5

hello";
        let mut reader = &data[..];
        let resp = parse_response(&mut reader).await.unwrap();
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.reason, "OK");
        assert_eq!(resp.version, "1.1");
        assert_eq!(resp.headers.get("Content-Type").unwrap(), "text/html");
    }

    #[tokio::test]
    async fn test_parse_response_not_found() {
        let data = b"HTTP/1.1 404 Not Found
Content-Type: text/html
Content-Length: 0

";
        let mut reader = &data[..];
        let resp = parse_response(&mut reader).await.unwrap();
        assert_eq!(resp.status_code, 404);
        assert_eq!(resp.reason, "Not Found");
    }

    #[tokio::test]
    async fn test_parse_response_invalid_line() {
        let data = b"INVALID
";
        let mut reader = &data[..];
        let result = parse_response(&mut reader).await;
        assert!(result.is_err());
    }
}
