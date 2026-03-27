/// MITM-specific HTTP proxy handler wrapper
///
/// Delegates to the shared scred-http proxy handler so MITM and scred-proxy
/// benefit from the same HTTP/proxy fixes.

use anyhow::Result;
use scred_http::http_proxy_handler::{handle_http_proxy as shared_handle_http_proxy, HttpProxyConfig};
use scred_readctor_framering::RedactionEngine;
use std::sync::Arc;
use tracing::debug;

pub async fn handle_http_proxy(
    client_read: tokio::net::tcp::OwnedReadHalf,
    client_write: tokio::net::tcp::OwnedWriteHalf,
    first_line: &str,
    redaction_engine: Arc<RedactionEngine>,
    upstream_resolver: Arc<scred_http::proxy_resolver::MitmConfig>,
    redact_selector: Option<scred_http::PatternSelector>,
) -> Result<()> {
    debug!("MITM HTTP proxy request: {}", first_line);

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!("Invalid HTTP request line"));
    }

    let url = parts[1];
    let (host, port, _path) = scred_http::http_proxy_handler::parse_proxy_url(url)?;
    let upstream_addr = upstream_resolver
        .get_proxy_for(&host, false)
        .unwrap_or_else(|| format!("{}:{}", host, port));

    let proxy_config = HttpProxyConfig {
        add_via_header: true,
        add_scred_header: true,
    };

    shared_handle_http_proxy(
        client_read,
        client_write,
        first_line,
        redaction_engine,
        &upstream_addr,
        Some(&host),
        redact_selector,
        proxy_config,
    ).await
}
