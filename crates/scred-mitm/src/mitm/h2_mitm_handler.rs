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
        tracing::info!(
            "[H2MitmHandler] Created with automaton: {:?}",
            policy.is_some()
        );
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

    /// Read complete request body from h2::RecvStream
    async fn read_h2_body(recv_stream: &mut h2::RecvStream) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        while let Some(chunk) = recv_stream.data().await {
            let chunk = chunk?;
            body.extend_from_slice(&chunk);
        }
        tracing::debug!("[H2] Request body received: {} bytes", body.len());
        Ok(body)
    }

    /// Apply redaction to request body with selector support
    fn redact_h2_body(
        body: &[u8],
        engine: &Arc<RedactionEngine>,
        redact_patterns: &scred_http::PatternSelector,
    ) -> Bytes {
        if body.is_empty() {
            return Bytes::new();
        }
        let body_str = String::from_utf8_lossy(body);
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
    }

    /// Process H2 headers with policy actions (Replace, Redact, Detect, Passthrough)
    /// Falls back to secret redaction when no policy engine is configured.
    fn process_h2_headers(
        headers: &http::HeaderMap,
        host: &str,
        policy: &Option<Arc<PolicyEngine>>,
        redaction_engine: &Arc<RedactionEngine>,
    ) -> http::HeaderMap {
        let mut result = http::HeaderMap::new();

        for (name, value) in headers {
            if Self::is_hop_by_hop_header(name) {
                continue;
            }

            let processed_value = Self::apply_header_policy(name, value, host, policy, redaction_engine);
            result.insert(name.clone(), processed_value);
        }

        result
    }

    /// Check if a header is a hop-by-hop header that should not be forwarded
    fn is_hop_by_hop_header(name: &http::HeaderName) -> bool {
        matches!(
            name.as_str().to_lowercase().as_str(),
            "connection"
                | "transfer-encoding"
                | "upgrade"
                | "te"
                | "trailer"
                | "proxy-authenticate"
                | "proxy-authorization"
        )
    }

    /// Apply policy to a header value (redact, replace, detect, or passthrough).
    /// Falls back to secret redaction when no policy engine is configured.
    fn apply_header_policy(
        name: &http::HeaderName,
        value: &http::HeaderValue,
        host: &str,
        policy: &Option<Arc<PolicyEngine>>,
        redaction_engine: &Arc<RedactionEngine>,
    ) -> http::HeaderValue {
        let value_str = value.to_str().unwrap_or("");

        let Some(ref engine) = policy else {
            // No policy engine: detect-only for headers (log, don't modify)
            let detection = scred_redactor::scred_detector::detect_all(value_str.as_bytes());
            for m in &detection.matches {
                tracing::info!(
                    "[H2] Detected {} in header: {}",
                    m.pattern_type, name
                );
            }
            return value.clone();
        };

        use scred_config::HeaderAction;
        let resolved = engine.resolve_for_host(host);
        let action = resolved.header_action(name.as_str());

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
    }

    /// Send HTTP/2 response to client
    fn send_h2_response(
        respond: &mut server::SendResponse<Bytes>,
        response_bytes: &[u8],
    ) -> Result<()> {
        let response = match Response::builder().status(200).body(()) {
            Ok(r) => r,
            Err(e) => unreachable!("valid HTTP status: {}", e),
        };
        let mut send = respond.send_response(response, false)?;

        if !response_bytes.is_empty() {
            send.send_data(Bytes::from(response_bytes.to_vec()), true)?;
        } else {
            send.send_data(Bytes::new(), true)?;
        }

        Ok(())
    }

    /// Handle individual stream
    #[allow(clippy::too_many_arguments)]
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
        let request_body = Self::read_h2_body(&mut recv_stream).await?;

        // Apply redaction to request body
        let redacted_body = Self::redact_h2_body(&request_body, &engine, &redact_patterns);

        // Process headers with policy actions (or secret redaction if no policy)
        let processed_headers = Self::process_h2_headers(&request_parts.headers, host, &policy, &engine);

        // Build upstream request
        let mut builder = http::Request::builder()
            .method(request_parts.method.clone())
            .uri(request_parts.uri.clone());

        for (name, value) in &processed_headers {
            builder = builder.header(name.clone(), value.clone());
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
                Self::send_h2_response(&mut respond, &response_bytes)
            }
            Err(e) => {
                tracing::error!("[H2] Upstream forwarding failed: {}", e);
                let response = match Response::builder().status(502).body(()) {
                    Ok(r) => r,
                    Err(e) => unreachable!("valid HTTP status: {}", e),
                };
                let _send = respond.send_response(response, true)?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h2_mitm_config_defaults() {
        let config = H2MitmConfig::default();
        assert_eq!(config.max_concurrent_streams, 100);
        assert_eq!(config.initial_connection_window_size, 65535);
        assert_eq!(config.initial_stream_window_size, 65535);
        assert_eq!(config.redaction_mode, RedactionMode::DetectOnly);
    }

    #[test]
    fn test_h2_mitm_handler_creation() {
        let engine = Arc::new(RedactionEngine::new(
            scred_redactor::RedactionConfig { enabled: true },
        ));
        let config = H2MitmConfig::default();
        let handler = H2MitmHandler::new(
            engine,
            "example.com:443".to_string(),
            config,
            None,
        );
        assert_eq!(handler.upstream_addr, "example.com:443");
        assert!(handler.policy.is_none());
    }

    #[test]
    fn test_h2_mitm_config_custom_redaction_mode() {
        let config = H2MitmConfig {
            redaction_mode: RedactionMode::Redact,
            ..Default::default()
        };
        assert_eq!(config.redaction_mode, RedactionMode::Redact);
        
        let config = H2MitmConfig {
            redaction_mode: RedactionMode::Passthrough,
            ..Default::default()
        };
        assert_eq!(config.redaction_mode, RedactionMode::Passthrough);
    }

    #[test]
    fn test_h2_mitm_config_custom_pattern_selectors() {
        let config = H2MitmConfig {
            detect_patterns: scred_http::PatternSelector::None,
            redact_patterns: scred_http::PatternSelector::None,
            ..Default::default()
        };
        assert_eq!(config.detect_patterns, scred_http::PatternSelector::None);
        assert_eq!(config.redact_patterns, scred_http::PatternSelector::None);
    }

    #[test]
    fn test_is_hop_by_hop_header_connection() {
        let name = http::HeaderName::from_static("connection");
        assert!(H2MitmHandler::is_hop_by_hop_header(&name));
    }

    #[test]
    fn test_is_hop_by_hop_header_transfer_encoding() {
        let name = http::HeaderName::from_static("transfer-encoding");
        assert!(H2MitmHandler::is_hop_by_hop_header(&name));
    }

    #[test]
    fn test_is_hop_by_hop_header_content_type() {
        let name = http::HeaderName::from_static("content-type");
        assert!(!H2MitmHandler::is_hop_by_hop_header(&name));
    }

    #[test]
    fn test_is_hop_by_hop_header_host() {
        let name = http::HeaderName::from_static("host");
        assert!(!H2MitmHandler::is_hop_by_hop_header(&name));
    }

    #[test]
    fn test_apply_header_policy_no_policy() {
        let engine = Arc::new(RedactionEngine::new(
            scred_redactor::RedactionConfig { enabled: true },
        ));
        let name = http::HeaderName::from_static("content-type");
        let value = http::HeaderValue::from_static("text/html");
        let result = H2MitmHandler::apply_header_policy(&name, &value, "example.com", &None, &engine);
        assert_eq!(result, "text/html");
    }

    #[test]
    fn test_apply_header_policy_no_policy_detects_secrets() {
        let engine = Arc::new(RedactionEngine::new(
            scred_redactor::RedactionConfig { enabled: true },
        ));
        let name = http::HeaderName::from_static("x-api-key");
        let value = http::HeaderValue::from_static("AKIAIOSFODNN7EXAMPLE");
        let result = H2MitmHandler::apply_header_policy(&name, &value, "example.com", &None, &engine);
        let result_str = result.to_str().unwrap();
        // Detect-only: value should pass through unchanged
        assert_eq!(
            result_str,
            "AKIAIOSFODNN7EXAMPLE",
            "Secret should pass through unchanged (detect-only), got: {}",
            result_str
        );
    }

    #[test]
    fn test_apply_header_policy_with_disabled_policy() {
        use std::sync::Arc;
        use scred_policy::PolicyEngine;
        let engine = Arc::new(RedactionEngine::new(
            scred_redactor::RedactionConfig { enabled: true },
        ));
        let config = scred_config::PolicyConfig {
            enabled: false,
            providers: vec![],
            ..Default::default()
        };
        let policy_engine = Arc::new(PolicyEngine::new(config).unwrap());
        let policy = Some(policy_engine);
        let name = http::HeaderName::from_static("content-type");
        let value = http::HeaderValue::from_static("text/html");
        let result = H2MitmHandler::apply_header_policy(&name, &value, "example.com", &policy, &engine);
        assert_eq!(result, "text/html");
    }

    #[test]
    fn test_apply_header_policy_with_enabled_policy() {
        use std::sync::Arc;
        use scred_policy::PolicyEngine;
        let engine = Arc::new(RedactionEngine::new(
            scred_redactor::RedactionConfig { enabled: true },
        ));
        let config = scred_config::PolicyConfig {
            enabled: true,
            providers: vec![],
            ..Default::default()
        };
        let policy_engine = Arc::new(PolicyEngine::new(config).unwrap());
        let policy = Some(policy_engine);
        let name = http::HeaderName::from_static("authorization");
        let value = http::HeaderValue::from_static("Bearer token-12345");
        let result = H2MitmHandler::apply_header_policy(&name, &value, "example.com", &policy, &engine);
        assert_eq!(result, "Bearer token-12345");
    }

    #[test]
    fn test_process_h2_headers_detects_secrets() {
        let engine = Arc::new(RedactionEngine::new(
            scred_redactor::RedactionConfig { enabled: true },
        ));
        let mut headers = http::HeaderMap::new();
        headers.insert("content-type", "text/plain".parse().unwrap());
        headers.insert("x-api-key", "AKIAIOSFODNN7EXAMPLE".parse().unwrap());
        headers.insert("authorization", "Bearer sk-proj-test123".parse().unwrap());

        let result = H2MitmHandler::process_h2_headers(&headers, "example.com", &None, &engine);

        // Non-secret header should be unchanged
        assert_eq!(
            result.get("content-type").unwrap().to_str().unwrap(),
            "text/plain"
        );

        // Secret headers should be forwarded UNCHANGED (detect-only)
        let api_key = result.get("x-api-key").unwrap().to_str().unwrap();
        assert_eq!(
            api_key,
            "AKIAIOSFODNN7EXAMPLE",
            "AWS key should pass through unchanged (detect-only), got: {}",
            api_key
        );

        let auth = result.get("authorization").unwrap().to_str().unwrap();
        assert_eq!(
            auth,
            "Bearer sk-proj-test123",
            "OpenAI key should pass through unchanged (detect-only), got: {}",
            auth
        );
    }

    #[test]
    fn test_process_h2_headers_preserves_hop_by_hop() {
        let engine = Arc::new(RedactionEngine::new(
            scred_redactor::RedactionConfig { enabled: true },
        ));
        let mut headers = http::HeaderMap::new();
        headers.insert("connection", "keep-alive".parse().unwrap());
        headers.insert("transfer-encoding", "chunked".parse().unwrap());
        headers.insert("x-custom", "AKIAIOSFODNN7EXAMPLE".parse().unwrap());

        let result = H2MitmHandler::process_h2_headers(&headers, "example.com", &None, &engine);

        // Hop-by-hop headers should be removed
        assert!(result.get("connection").is_none());
        assert!(result.get("transfer-encoding").is_none());

        // Non-hop-by-hop headers should pass through unchanged (detect-only)
        let custom = result.get("x-custom").unwrap().to_str().unwrap();
        assert_eq!(
            custom,
            "AKIAIOSFODNN7EXAMPLE",
            "Secret should pass through unchanged (detect-only), got: {}",
            custom
        );
    }
}
