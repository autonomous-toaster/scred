//! Discovery API - expose placeholders without secret values

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{debug, info};

use crate::placeholder::Placeholder;

/// Configuration for discovery API
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Port to listen on
    pub port: u16,
    /// Bind address
    pub bind: String,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            port: 9998,
            bind: "0.0.0.0".to_string(),
        }
    }
}

/// Discovery API server
pub struct DiscoveryServer {
    config: DiscoveryConfig,
    pub(crate) placeholders: Arc<std::sync::Mutex<HashMap<String, Placeholder>>>,
}

impl Clone for DiscoveryServer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            placeholders: self.placeholders.clone(),
        }
    }
}

impl DiscoveryServer {
    /// Create a new discovery server
    pub fn new(config: DiscoveryConfig) -> Self {
        Self {
            config,
            placeholders: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Update placeholders
    pub fn update_placeholders(&self, placeholders: HashMap<String, Placeholder>) {
        let mut current = self.placeholders.lock().unwrap();
        *current = placeholders;
    }

    /// Get a clone of current placeholders
    fn get_placeholders(&self) -> HashMap<String, Placeholder> {
        self.placeholders.lock().unwrap().clone()
    }

    /// Run the discovery server
    pub async fn run(&self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.bind, self.config.port).parse()?;
        let listener = TcpListener::bind(addr).await?;
        info!("Discovery API listening on {}", addr);

        loop {
            let (stream, peer) = listener.accept().await?;
            let server = self.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, &server).await {
                    debug!("Connection error from {}: {}", peer, e);
                }
            });
        }
    }

    /// Create a handle for updating placeholders from another task
    pub fn updater(&self) -> DiscoveryUpdater {
        DiscoveryUpdater {
            placeholders: self.placeholders.clone(),
        }
    }
}

/// Handle for updating placeholders
#[derive(Clone)]
pub struct DiscoveryUpdater {
    placeholders: Arc<std::sync::Mutex<HashMap<String, Placeholder>>>,
}

impl DiscoveryUpdater {
    /// Update all placeholders
    pub fn update(&self, placeholders: HashMap<String, Placeholder>) {
        let mut current = self.placeholders.lock().unwrap();
        *current = placeholders;
    }
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    server: &DiscoveryServer,
) -> Result<()> {
    let mut buffer = [0u8; 4096];
    let n = stream.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let request = std::str::from_utf8(&buffer[..n])?;

    // Parse request line
    let request_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 3 {
        send_error(&mut stream, 400, "Bad Request").await?;
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    // Only support GET /placeholders
    if method != "GET" {
        send_error(&mut stream, 405, "Method Not Allowed").await?;
        return Ok(());
    }

    if !path.starts_with("/placeholders") {
        send_error(&mut stream, 404, "Not Found").await?;
        return Ok(());
    }

    // Check Accept header for JSON
    let accept_json = request.lines().any(|line| {
        line.to_lowercase().starts_with("accept:") && line.contains("application/json")
    });

    // Build response using the method
    let placeholders_map = server.get_placeholders();
    if accept_json {
        send_json_response(&mut stream, &placeholders_map).await?;
    } else {
        send_text_response(&mut stream, &placeholders_map).await?;
    }

    Ok(())
}

async fn send_text_response(
    stream: &mut tokio::net::TcpStream,
    placeholders: &HashMap<String, Placeholder>,
) -> Result<()> {
    let mut body = String::new();

    // Sort for deterministic output
    let mut keys: Vec<_> = placeholders.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(placeholder) = placeholders.get(key) {
            body.push_str(&format!("{}={}\\n", key, placeholder.value));
        }
    }

    let response = format!(
        "HTTP/1.1 200 OK\\r\\nContent-Type: text/plain\\r\\nContent-Length: {}\\r\\n\\r\\n{}",
        body.len(),
        body
    );

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn send_json_response(
    stream: &mut tokio::net::TcpStream,
    placeholders: &HashMap<String, Placeholder>,
) -> Result<()> {
    let mut entries: Vec<_> = placeholders
        .iter()
        .map(|(k, v)| (k.clone(), v.value.clone()))
        .collect();
    entries.sort_by_key(|(k, _)| k.clone());

    let json = serde_json::to_string(
        &entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<std::collections::HashMap<_, _>>(),
    )?;

    let response = format!(
        "HTTP/1.1 200 OK\\r\\nContent-Type: application/json\\r\\nContent-Length: {}\\r\\n\\r\\n{}",
        json.len(),
        json
    );

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn send_error(stream: &mut tokio::net::TcpStream, code: u16, message: &str) -> Result<()> {
    let body = format!("{} {}\\n", code, message);
    let response = format!(
        "HTTP/1.1 {} {}\\r\\nContent-Type: text/plain\\r\\nContent-Length: {}\\r\\n\\r\\n{}",
        code,
        message,
        body.len(),
        body
    );

    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_text_response() {
        let config = DiscoveryConfig {
            port: 19999,
            bind: "127.0.0.1".to_string(),
        };
        let server = DiscoveryServer::new(config);
        let updater = server.updater();

        // Add a placeholder
        let mut placeholders = HashMap::new();
        placeholders.insert(
            "TEST_KEY".to_string(),
            Placeholder {
                name: "TEST_KEY".to_string(),
                value: "sk-abc123".to_string(),
                prefix: "sk-".to_string(),
            },
        );
        updater.update(placeholders);

        // Verify placeholders are stored
        let retrieved = server.placeholders.lock().unwrap();
        assert!(retrieved.contains_key("TEST_KEY"));
    }
}
