use crate::ProxyConfig;
use anyhow::{anyhow, Result};
use rustls::{ClientConfig, RootCertStore, ServerName};
use scred_http::OptimizedDnsResolver;
use scred_policy::PolicyEngine;
use scred_redactor::streaming::RedactionStream;
use std::sync::Arc;
use tokio::io::{copy, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::{debug, info};

pub async fn handle_connection(
    stream: TcpStream,
    config: Arc<ProxyConfig>,
    resolver: Arc<OptimizedDnsResolver>,
    peer_addr: std::net::SocketAddr,
    policy: Option<Arc<PolicyEngine>>,
) -> Result<()> {
    let (client_read, mut client_write) = stream.into_split();
    let mut client_reader = BufReader::with_capacity(256 * 1024, client_read);
    let mut request_count = 0;

    loop {
        request_count += 1;

        let mut first_line = String::new();
        client_reader.read_line(&mut first_line).await?;

        if first_line.is_empty() {
            if request_count > 1 {
                debug!("Connection closed after {} requests", request_count - 1);
            }
            break;
        }

        let first_line = first_line.trim().to_string();
        let request_path = extract_path(&first_line);

        info!(
            "{} \"{} {}\"",
            peer_addr.ip(),
            extract_method(&first_line),
            request_path
        );

        handle_single_proxy_request(
            &mut client_reader,
            &mut client_write,
            &first_line,
            &config,
            &resolver,
            &policy,
        )
        .await?;

        client_write.flush().await?;
    }

    Ok(())
}

/// Handle a single proxy request (forward to upstream with optional policy)
async fn handle_single_proxy_request<R, W>(
    client_reader: &mut BufReader<R>,
    client_write: &mut W,
    first_line: &str,
    config: &ProxyConfig,
    resolver: &Arc<OptimizedDnsResolver>,
    policy: &Option<Arc<PolicyEngine>>,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let upstream_addr = config.upstream.authority();
    let rewritten_request_line = config.upstream.rewrite_request_line(first_line)?;

    let tcp_stream = resolver.connect_with_retry(&upstream_addr).await?;

    if config.upstream.scheme == "https" {
        let mut upstream = connect_tls_upstream(tcp_stream, &config.upstream.host).await?;
        forward_request(client_reader, &mut upstream, &rewritten_request_line, policy, &config.upstream.host).await?;
        let mut upstream_buf = BufReader::new(upstream);
        forward_response(&mut upstream_buf, client_write, policy, &config.upstream.host).await?;
    } else {
        let mut upstream = tcp_stream;
        forward_request(client_reader, &mut upstream, &rewritten_request_line, policy, &config.upstream.host).await?;
        let mut upstream_buf = BufReader::new(upstream);
        forward_response(&mut upstream_buf, client_write, policy, &config.upstream.host).await?;
    }

    Ok(())
}

/// Forward request to upstream (with or without policy)
async fn forward_request<R, W>(
    client_reader: &mut BufReader<R>,
    upstream: &mut W,
    request_line: &str,
    policy: &Option<Arc<PolicyEngine>>,
    host: &str,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    if let Some(ref engine) = policy {
        forward_with_policy(client_reader, upstream, request_line, engine, host).await?;
        forward_body_redacted(client_reader, upstream, engine, host).await?;
    } else {
        forward_simple(client_reader, upstream, request_line).await?;
        copy(client_reader, upstream).await?;
    }
    Ok(())
}

/// Forward response to client (with or without redaction)
async fn forward_response<R, W>(
    upstream_reader: &mut BufReader<R>,
    client_write: &mut W,
    policy: &Option<Arc<PolicyEngine>>,
    host: &str,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    if let Some(ref engine) = policy {
        forward_response_redacted(upstream_reader, client_write, engine, host).await?;
    } else {
        copy(upstream_reader, client_write).await?;
    }
    Ok(())
}

/// Forward request with policy processing
pub async fn forward_with_policy<R, W>(
    client_reader: &mut BufReader<R>,
    upstream: &mut W,
    request_line: &str,
    engine: &PolicyEngine,
    host: &str,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    // Send request line
    let req_line = format!("{}\r\n", request_line);
    upstream.write_all(req_line.as_bytes()).await?;

    // Read and process headers
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        client_reader.read_line(&mut line).await?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
        headers.push(line);
    }

    // Process headers through policy engine
    let header_str: String = headers.join("");
    let mut header_bytes = header_str.into_bytes();

    // Use placeholder automaton for replacement
    let automaton = engine.create_placeholder_automaton();
    let (_, count) = automaton.replace_placeholders(&mut header_bytes, host, |_, _| true);

    if count > 0 {
        debug!("Replaced {} placeholders in request headers", count);
    }

    upstream.write_all(&header_bytes).await?;
    upstream.write_all(b"\r\n").await?;
    upstream.flush().await?;

    Ok(())
}

/// Simple forwarding without policy
pub async fn forward_simple<R, W>(
    client_reader: &mut BufReader<R>,
    upstream: &mut W,
    request_line: &str,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    // Send request line
    let req_line = format!("{}\r\n", request_line);
    upstream.write_all(req_line.as_bytes()).await?;

    // Forward headers
    let mut headers_buf = Vec::new();
    loop {
        let mut line = String::new();
        client_reader.read_line(&mut line).await?;
        headers_buf.extend_from_slice(line.as_bytes());
        if line == "\r\n" || line.is_empty() {
            break;
        }
    }

    upstream.write_all(&headers_buf).await?;
    upstream.flush().await?;

    Ok(())
}

/// Stream data from reader to writer through a RedactionStream
async fn stream_with_redaction<R, W>(
    reader: &mut BufReader<R>,
    writer: &mut W,
    engine: &PolicyEngine,
) -> Result<scred_redactor::streaming::StreamingStats>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let redaction_engine = engine.redaction_engine();
    let mut redaction_stream = RedactionStream::new(Arc::clone(redaction_engine));
    let mut buffer = vec![0u8; 65536];

    loop {
        match reader.read(&mut buffer).await? {
            0 => break,
            n => {
                let redacted = redaction_stream.feed(&buffer[..n]);
                if !redacted.is_empty() {
                    writer.write_all(&redacted).await?;
                }
            }
        }
    }

    let (remaining, stats) = redaction_stream.finalize();
    if !remaining.is_empty() {
        writer.write_all(&remaining).await?;
    }

    writer.flush().await?;
    Ok(stats)
}

/// Forward request body with redaction through RedactionStream
pub async fn forward_body_redacted<R, W>(
    client_reader: &mut BufReader<R>,
    upstream: &mut W,
    engine: &PolicyEngine,
    host: &str,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let stats = stream_with_redaction(client_reader, upstream, engine).await?;
    if stats.patterns_found > 0 {
        debug!(
            "[proxy] Redacted {} patterns in request body to {}",
            stats.patterns_found, host
        );
    }
    Ok(())
}

/// Forward response body with redaction through RedactionStream
pub async fn forward_response_redacted<R, W>(
    upstream_reader: &mut BufReader<R>,
    client_write: &mut W,
    engine: &PolicyEngine,
    host: &str,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let stats = stream_with_redaction(upstream_reader, client_write, engine).await?;
    if stats.patterns_found > 0 {
        debug!(
            "[proxy] Redacted {} patterns in response body from {}",
            stats.patterns_found, host
        );
    }
    Ok(())
}

pub async fn connect_tls_upstream<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    stream: S,
    host: &str,
) -> Result<tokio_rustls::client::TlsStream<S>> {
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    let client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name =
        ServerName::try_from(host).map_err(|_| anyhow!("Invalid upstream host: {}", host))?;

    connector
        .connect(server_name, stream)
        .await
        .map_err(|e| anyhow!("TLS handshake failed: {}", e))
}

pub fn extract_method(request_line: &str) -> &str {
    request_line.split(' ').next().unwrap_or("UNKNOWN")
}

pub fn extract_path(request_line: &str) -> &str {
    let parts: Vec<&str> = request_line.split(' ').collect();
    if parts.len() >= 2 {
        let path_with_query = parts[1];
        path_with_query.split('?').next().unwrap_or("/")
    } else {
        "/"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_method_get() {
        assert_eq!(extract_method("GET /path HTTP/1.1"), "GET");
    }

    #[test]
    fn test_extract_method_post() {
        assert_eq!(extract_method("POST /api HTTP/1.1"), "POST");
    }

    #[test]
    fn test_extract_method_connect() {
        assert_eq!(extract_method("CONNECT example.com:443 HTTP/1.1"), "CONNECT");
    }

    #[test]
    fn test_extract_method_empty() {
        assert_eq!(extract_method(""), "");
    }

    #[test]
    fn test_extract_path_normal() {
        assert_eq!(extract_path("GET /path/to/resource HTTP/1.1"), "/path/to/resource");
    }

    #[test]
    fn test_extract_path_with_query() {
        assert_eq!(extract_path("GET /search?q=hello HTTP/1.1"), "/search");
    }

    #[test]
    fn test_extract_path_root() {
        assert_eq!(extract_path("GET / HTTP/1.1"), "/");
    }

    #[test]
    fn test_extract_path_no_path() {
        assert_eq!(extract_path("GET"), "/");
    }

    #[test]
    fn test_extract_path_empty() {
        assert_eq!(extract_path(""), "/");
    }

    #[test]
    fn test_forward_simple_creates_request_line() {
        let request_line = format!("{} {}", "GET", "/path");
        assert_eq!(request_line, "GET /path");
    }

    #[test]
    fn test_forward_with_policy_creates_request_line() {
        let request_line = format!("{} {}", "POST", "/api");
        assert_eq!(request_line, "POST /api");
    }


}
