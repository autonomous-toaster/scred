/// Shared HTTP models and data structures
use std::collections::HashMap;

/// HTTP request representation
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
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

    /// Set a header (replaces existing case-insensitively)
    pub fn set_header(&mut self, key: String, value: String) {
        let key_lower = key.to_lowercase();
        self.headers.retain(|k, _| k.to_lowercase() != key_lower);
        self.headers.insert(key, value);
    }

    /// Remove a header (case-insensitive)
    pub fn remove_header(&mut self, key: &str) {
        let key_lower = key.to_lowercase();
        self.headers.retain(|k, _| k.to_lowercase() != key_lower);
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
            Some(te) if te.to_lowercase().contains("chunked")
        )
    }

    /// Serialize request to HTTP format
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        
        // Request line
        buf.extend_from_slice(format!("{} {} {}\r\n", self.method, self.path, self.version).as_bytes());
        
        // Headers
        for (key, value) in &self.headers {
            buf.extend_from_slice(format!("{}: {}\r\n", key, value).as_bytes());
        }
        
        // Empty line
        buf.extend_from_slice(b"\r\n");
        
        // Body
        buf.extend_from_slice(&self.body);
        
        buf
    }
}

/// HTTP response representation
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub version: String,
    pub status_code: u16,
    pub reason: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
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

    /// Set a header (replaces existing case-insensitively)
    pub fn set_header(&mut self, key: String, value: String) {
        let key_lower = key.to_lowercase();
        self.headers.retain(|k, _| k.to_lowercase() != key_lower);
        self.headers.insert(key, value);
    }

    /// Remove a header (case-insensitive)
    pub fn remove_header(&mut self, key: &str) {
        let key_lower = key.to_lowercase();
        self.headers.retain(|k, _| k.to_lowercase() != key_lower);
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
            Some(te) if te.to_lowercase().contains("chunked")
        )
    }

    /// Serialize response to HTTP format
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        
        // Status line
        buf.extend_from_slice(
            format!("{} {} {}\r\n", self.version, self.status_code, self.reason).as_bytes()
        );
        
        // Headers
        for (key, value) in &self.headers {
            buf.extend_from_slice(format!("{}: {}\r\n", key, value).as_bytes());
        }
        
        // Empty line
        buf.extend_from_slice(b"\r\n");
        
        // Body
        buf.extend_from_slice(&self.body);
        
        buf
    }
}

/// Pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub pattern_name: String,
    pub matched_text: String,
    pub byte_offset: usize,
    pub redacted_as: String,
}

