//! HTTP/2 MITM Handler - Using h2 crate with transparent redaction

use crate::mitm::config::RedactionMode;
use crate::mitm::h2_upstream_forwarder;
use anyhow::Result;
use bytes::Bytes;
use h2::server;
use http::Response;
use scred_policy::PolicyEngine;
use scred_redactor::RedactionEngine;
use std::sync::Arc;

/// Configuration for H2 MITM handler
#[derive(Clone, Debug)]
pub struct H2MitmConfig {
    pub max_concurrent_streams: u32,
    pub initial_connection_window_size: u32,
    pub initial_stream_window_size: u32,
    pub redaction_mode: RedactionMode,
    pub detect_patterns: scred_http::PatternSelector,
    pub redact_patterns: scred_http::PatternSelector,
}

impl Default for H2MitmConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 100,
            initial_connection_window_size: 65535,
            initial_stream_window_size: 65535,
            redaction_mode: RedactionMode::DetectOnly,
            detect_patterns: scred_http::PatternSelector::default_detect(),
            redact_patterns: scred_http::PatternSelector::default_redact(),
        }
    }
}

/// HTTP/2 MITM Handler
///
/// Manages bidirectional HTTP/2 with per-stream redaction using h2 crate
pub struct H2MitmHandler {
    /// Redaction engine for per-stream redaction
    engine: Arc<RedactionEngine>,
    /// Configuration
    config: H2MitmConfig,
    /// Upstream address
    upstream_addr: String,
    /// Policy engine for placeholder replacement and redaction
    policy: Option<Arc<PolicyEngine>>,
}

impl H2MitmHandler {
    /// Create new handler with policy support
    pub fn new(
        engine: Arc<RedactionEngine>,
        upstream_addr: String,
        config: H2MitmConfig,
        policy: Option<Arc<PolicyEngine>>,
    ) -> Self {
        tracing::info!("[H2MitmHandler] Created with automaton: {:?}", policy.is_some());
        Self {
            engine,
            config,
            upstream_addr,
            policy,
        }
    }

    /// Handle HTTP/2 connection from client
    pub async fn handle_connection<S>(&self, socket: S, host: &str) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        // Build h2 server
        let mut connection = server::handshake(socket).await?;
        tracing::info!("[H2] Server handshake complete, host={}", host);

        // Process incoming streams
        while let Some(result) = connection.accept().await {
            let (request, respond) = result?;

            let engine = self.engine.clone();
            let upstream_addr = self.upstream_addr.clone();
            let host = host.to_string();
            let redaction_mode = self.config.redaction_mode;
            let detect_patterns = self.config.detect_patterns.clone();
            let redact_patterns = self.config.redact_patterns.clone();
            let policy = self.policy.clone();

            // Handle each stream in background
            tokio::spawn(async move {
                if let Err(e) = Self::handle_stream(
                    request,
                    respond,
                    engine,
                    upstream_addr,
                    &host,
                    redaction_mode,
                    detect_patterns,
                    redact_patterns,
                    policy,
                )
                .await
                {
                    tracing::warn!("[H2] Stream error: {}", e);
                }
            });
        }

        tracing::info!("[H2] Connection closed");
        Ok(())
    }

    /// Handle individual stream
    async fn handle_stream(
        request: http::Request<h2::RecvStream>,
        mut respond: server::SendResponse<Bytes>,
        engine: Arc<RedactionEngine>,
        upstream_addr: String,
        host: &str,
        redaction_mode: RedactionMode,
        detect_patterns: scred_http::PatternSelector,
        redact_patterns: scred_http::PatternSelector,
        policy: Option<Arc<PolicyEngine>>,
    ) -> Result<()> {
        let method = request.method().clone();
        let uri = request.uri().clone();
        tracing::debug!("[H2] Stream: {} {}", method, uri);

        // Extract request parts and body
        let (request_parts, mut recv_stream) = request.into_parts();
        let method = request_parts.method.clone();
        let uri = request_parts.uri.clone();

        // Extract authority from headers (HTTP/2 pseudo-header or regular header)
        let authority = request_parts
            .headers
            .get("authority")
            .or_else(|| request_parts.headers.get(":authority"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        tracing::debug!("[H2 Stream] {} {} (authority: {})", method, uri, authority);

        // Read complete request body from h2::RecvStream
        let mut request_body = Vec::new();
        while let Some(chunk) = recv_stream.data().await {
            let chunk = chunk?;
            request_body.extend_from_slice(&chunk);
            tracing::debug!("[H2] Received body chunk: {} bytes", chunk.len());
        }
        tracing::debug!("[H2] Request body received: {} bytes", request_body.len());

        // Apply redaction to request body if present with selector support
        let redacted_body = if !request_body.is_empty() {
            let body_str = String::from_utf8_lossy(&request_body);
            let redacted = if !matches!(redact_patterns, scred_http::PatternSelector::None) {
                let selective_engine = Arc::new(RedactionEngine::with_selector(
                    engine.config().clone(),
                    redact_patterns.clone(),
                ));
                selective_engine.redact(&body_str)
            } else {
                engine.redact(&body_str)
            };
            Bytes::from(redacted.redacted.into_bytes())
        } else {
            Bytes::new()
        };

        // Build upstream request with ALL client headers
        let mut builder = http::Request::builder()
            .method(request_parts.method.clone())
            .uri(request_parts.uri.clone());

        // Process each header according to its HeaderAction from policy
    // Uses per-header action: Replace, Redact, Detect, or Passthrough
    for (name, value) in &request_parts.headers {
        let name_str = name.as_str().to_lowercase();

        // Skip hop-by-hop headers (RFC 7230)
        if matches!(
            name_str.as_str(),
            "connection" | "transfer-encoding" | "upgrade" | "te" | "trailer"
                | "proxy-authenticate" | "proxy-authorization"
        ) {
            tracing::debug!("[H2] Skipping hop-by-hop header: {}", name);
            continue;
        }

        // Determine header action from policy or default to Passthrough
        let processed_value = if let Some(ref engine) = policy {
            use scred_config::HeaderAction;
            let resolved = engine.resolve_for_host(host);
            let action = resolved.header_action(name.as_str());
            let value_str = value.to_str().unwrap_or("");

            match action {
                HeaderAction::Replace => {
                    let mut value_bytes = value_str.as_bytes().to_vec();
                    let (_, count) = engine
                        .create_placeholder_automaton()
                        .replace_placeholders(&mut value_bytes, host, |_, _| true);
                    if count > 0 {
                        tracing::info!(
                            "[H2 policy] Replaced {} placeholder(s) in header: {}",
                            count, name
                        );
                    }
                    http::HeaderValue::from_bytes(&value_bytes).unwrap_or(value.clone())
                }
                HeaderAction::Redact => {
                    let redacted = engine.redaction_engine().redact(value_str);
                    if !redacted.matches.is_empty() {
                        tracing::debug!(
                            "[H2 policy] Redacted {} secret(s) in header: {}",
                            redacted.matches.len(), name
                        );
                    }
                    http::HeaderValue::from_str(&redacted.redacted).unwrap_or(value.clone())
                }
                HeaderAction::Detect => {
                    let redacted = engine.redaction_engine().redact(value_str);
                    if !redacted.matches.is_empty() {
                        for m in &redacted.matches {
                            tracing::info!(
                                "[H2 policy] Detected {} in header: {}",
                                m.pattern_type, name
                            );
                        }
                    }
                    value.clone()
                }
                HeaderAction::Passthrough => value.clone(),
            }
        } else {
            value.clone()
        };

        builder = builder.header(name.clone(), processed_value);
        tracing::debug!("[H2] Forwarding header: {} (value hidden for security)", name);
    }

    let upstream_request = builder
            .body(redacted_body)
            .map_err(|e| anyhow::anyhow!("Failed to build upstream request: {}", e))?;

        // Forward to upstream
        match h2_upstream_forwarder::handle_upstream_h2_connection(
            upstream_request,
            engine,
            upstream_addr,
            host,
            redaction_mode,
            detect_patterns,
            redact_patterns,
        )
        .await
        {
            Ok(response_bytes) => {
                tracing::debug!(
                    "[H2 MITM] Got response from upstream: {} bytes",
                    response_bytes.len()
                );

                // Build HTTP/2 response
                let response = Response::builder().status(200).body(()).unwrap();
                let mut send = respond.send_response(response, false)?;

                if !response_bytes.is_empty() {
                    tracing::debug!(
                        "[H2 MITM] Sending response data: {} bytes",
                        response_bytes.len()
                    );
                    send.send_data(Bytes::from(response_bytes), true)?;
                } else {
                    tracing::warn!("[H2 MITM] WARNING: Response body is empty!");
                    send.send_data(Bytes::new(), true)?;
                }

                Ok(())
            }
            Err(e) => {
                // Send error response with diagnostic info
                tracing::error!("[H2] Upstream forwarding failed: {}", e);
                let error_msg = format!("502 Bad Gateway: {}", e);
                tracing::error!("[H2] Sending {} to client", error_msg);

                let response = Response::builder().status(502).body(()).unwrap();
                let _send = respond.send_response(response, true)?;

                Ok(())
            }
        }
    }
}
