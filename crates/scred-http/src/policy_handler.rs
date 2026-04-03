//! Policy Handler - Common placeholder replacement for HTTP proxies
//!
//! This module provides shared policy replacement functionality for both
//! scred-mitm and scred-proxy. It handles:
//! - Reading HTTP request headers and body
//! - Replacing placeholders with real secrets
//! - Forwarding modified request to upstream
//! - Reading response and replacing secrets with placeholders

use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

#[cfg(feature = "policy")]
use scred_policy::{PolicyIntegration, ReplacementTracker};

/// Result of policy-based HTTP handling
#[derive(Debug)]
#[cfg(feature = "policy")]
pub struct PolicyResult {
    /// Number of placeholders replaced in request
    pub request_replacements: usize,
    /// Number of secrets replaced in response
    pub response_replacements: usize,
    /// Tracker for replacements
    pub tracker: ReplacementTracker,
}

/// Handle HTTP request/response with policy replacement
///
/// Buffers request/response and applies placeholder/secret replacement.
#[cfg(feature = "policy")]
pub async fn handle_http_with_policy<C, U>(
    client_reader: &mut BufReader<C>,
    client_writer: &mut U,
    upstream: &mut BufReader<U>,
    request_line: &str,
    headers: &crate::http_headers::HttpHeaders,
    target_host: &str,
    policy: Arc<PolicyIntegration>,
) -> Result<PolicyResult>
where
    C: AsyncReadExt + Unpin,
    U: AsyncReadExt + AsyncWriteExt + Unpin,
{
    use tracing::{debug, info};

    info!("[policy] Processing request to {}", target_host);

    // Step 1: Read request body
    let mut body_bytes = Vec::new();
    if let Some(content_length) = headers.content_length {
        if content_length > 0 {
            body_bytes.resize(content_length, 0);
            client_reader.read_exact(&mut body_bytes).await?;
        }
    } else {
        let mut buf = vec![0u8; 8192];
        loop {
            match client_reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => body_bytes.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
    }

    debug!("[policy] Request body: {} bytes", body_bytes.len());

    // Step 2: Replace placeholders with real secrets
    let tracker = policy.process_request(&mut body_bytes, target_host).await;
    let request_replacements = tracker.replacements().len();

    if request_replacements > 0 {
        info!("[policy] Replaced {} placeholders in request", request_replacements);
    }

    // Step 3: Forward request to upstream
    let request_data = format!("{}\r\n{}\r\n", request_line, headers.raw_headers);
    upstream.get_mut().write_all(request_data.as_bytes()).await?;

    if !body_bytes.is_empty() {
        upstream.get_mut().write_all(&body_bytes).await?;
    }
    upstream.get_mut().flush().await?;

    info!("[policy] Request forwarded to upstream");

    // Step 4: Read response from upstream
    let response_line = crate::http_line_reader::read_response_line(upstream.get_mut()).await?;
    if response_line.is_empty() {
        return Err(anyhow::anyhow!("Empty response from upstream"));
    }

    debug!("[policy] Response line: {}", response_line);

    let response_headers = crate::http_headers::parse_http_headers(upstream, true)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse response headers: {}", e))?;

    // Step 5: Read response body
    let mut response_body = Vec::new();
    if let Some(content_length) = response_headers.content_length {
        if content_length > 0 {
            response_body.resize(content_length, 0);
            upstream.read_exact(&mut response_body).await?;
        }
    } else {
        let mut buf = vec![0u8; 8192];
        loop {
            match upstream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => response_body.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
    }

    debug!("[policy] Response body: {} bytes", response_body.len());

    // Step 6: Replace secrets with placeholders in response
    let response_replacements = policy.process_response(&mut response_body, &tracker).await;

    if response_replacements > 0 {
        info!("[policy] Replaced {} secrets in response", response_replacements);
    }

    // Step 7: Forward response to client
    client_writer
        .write_all(format!("{}\r\n", response_line).as_bytes())
        .await?;
    client_writer
        .write_all(response_headers.raw_headers.as_bytes())
        .await?;

    if response_replacements > 0 {
        client_writer
            .write_all(format!("Content-Length: {}\r\n", response_body.len()).as_bytes())
            .await?;
    }
    client_writer.write_all(b"\r\n").await?;
    client_writer.write_all(&response_body).await?;
    client_writer.flush().await?;

    info!("[policy] Response sent to client");

    Ok(PolicyResult {
        request_replacements,
        response_replacements,
        tracker,
    })
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "policy")]
    #[test]
    fn test_policy_result_debug() {
        use scred_policy::ReplacementTracker;

        let tracker = ReplacementTracker::new();
        let _ = format!("replacements: {}", tracker.replacements().len());
    }
}
