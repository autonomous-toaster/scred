use crate::ProxyConfig;
use anyhow::{anyhow, Result};
use rustls::{ClientConfig, RootCertStore, ServerName};
use scred_http::OptimizedDnsResolver;
use scred_policy::PolicyEngine;
use scred_redactor::streaming::RedactionStream;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
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
    use tokio::io::copy;

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

        let upstream_addr = config.upstream.authority();
        let rewritten_request_line = config.upstream.rewrite_request_line(&first_line)?;

        let tcp_stream = resolver.connect_with_retry(&upstream_addr).await?;

        if config.upstream.scheme == "https" {
            let mut upstream = connect_tls_upstream(tcp_stream, &config.upstream.host).await?;

            // Forward request with policy processing
            if let Some(ref engine) = policy {
                forward_with_policy(
                    &mut client_reader,
                    &mut upstream,
                    &rewritten_request_line,
                    engine,
                    &config.upstream.host,
                )
                .await?;
                // Forward request body with redaction
                forward_body_redacted(
                    &mut client_reader,
                    &mut upstream,
                    engine,
                    &config.upstream.host,
                )
                .await?;
            } else {
                // No policy - simple forwarding
                forward_simple(&mut client_reader, &mut upstream, &rewritten_request_line).await?;
                // Forward request body without redaction
                copy(&mut client_reader, &mut upstream).await?;
            }

            // Read and forward response with redaction
            let mut upstream_buf = BufReader::new(upstream);
            if let Some(ref engine) = policy {
                forward_response_redacted(
                    &mut upstream_buf,
                    &mut client_write,
                    engine,
                    &config.upstream.host,
                )
                .await?;
            } else {
                copy(&mut upstream_buf, &mut client_write).await?;
            }
        } else {
            let mut upstream = tcp_stream;

            if let Some(ref engine) = policy {
                forward_with_policy(
                    &mut client_reader,
                    &mut upstream,
                    &rewritten_request_line,
                    engine,
                    &config.upstream.host,
                )
                .await?;
                // Forward request body with redaction
                forward_body_redacted(
                    &mut client_reader,
                    &mut upstream,
                    engine,
                    &config.upstream.host,
                )
                .await?;
            } else {
                forward_simple(&mut client_reader, &mut upstream, &rewritten_request_line).await?;
                // Forward request body without redaction
                copy(&mut client_reader, &mut upstream).await?;
            }

            // Read and forward response with redaction
            let mut upstream_buf = BufReader::new(upstream);
            if let Some(ref engine) = policy {
                forward_response_redacted(
                    &mut upstream_buf,
                    &mut client_write,
                    engine,
                    &config.upstream.host,
                )
                .await?;
            } else {
                copy(&mut upstream_buf, &mut client_write).await?;
            }
        }

        client_write.flush().await?;
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

    let redaction_engine = engine.redaction_engine();
    let mut redaction_stream = RedactionStream::new(Arc::clone(redaction_engine));
    let mut buffer = vec![0u8; 65536]; // 64KB chunks

    loop {
        match client_reader.read(&mut buffer).await? {
            0 => break, // EOF
            n => {
                let redacted = redaction_stream.feed(&buffer[..n]);
                if !redacted.is_empty() {
                    upstream.write_all(&redacted).await?;
                }
            }
        }
    }

    // Finalize and flush remaining redacted data
    let (remaining, stats) = redaction_stream.finalize();
    if !remaining.is_empty() {
        upstream.write_all(&remaining).await?;
    }
    if stats.patterns_found > 0 {
        debug!(
            "[proxy] Redacted {} patterns in request body to {}",
            stats.patterns_found, host
        );
    }

    upstream.flush().await?;
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

    let redaction_engine = engine.redaction_engine();
    let mut redaction_stream = RedactionStream::new(Arc::clone(redaction_engine));
    let mut buffer = vec![0u8; 65536]; // 64KB chunks

    loop {
        match upstream_reader.read(&mut buffer).await? {
            0 => break, // EOF
            n => {
                let redacted = redaction_stream.feed(&buffer[..n]);
                if !redacted.is_empty() {
                    client_write.write_all(&redacted).await?;
                }
            }
        }
    }

    // Finalize and flush remaining redacted data
    let (remaining, stats) = redaction_stream.finalize();
    if !remaining.is_empty() {
        client_write.write_all(&remaining).await?;
    }
    if stats.patterns_found > 0 {
        debug!(
            "[proxy] Redacted {} patterns in response body from {}",
            stats.patterns_found, host
        );
    }

    client_write.flush().await?;
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
