//! Streaming policy integration for scred-proxy
//!
//! Integrates scred-policy into the streaming request/response path.

use anyhow::Result;
use scred_policy::{streaming::ReplacementTracker, PlaceholderAutomaton};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::debug;

/// Policy-aware streaming configuration
#[derive(Clone, Debug)]
pub struct StreamingPolicyConfig {
    /// Enable policy replacement
    pub enabled: bool,
    /// Target domain for domain restrictions
    pub domain: String,
    /// Debug logging
    pub debug: bool,
}

impl Default for StreamingPolicyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            domain: String::new(),
            debug: false,
        }
    }
}

/// Stream request with policy replacement
///
/// Buffers the request body, applies policy replacement, and streams to upstream.
/// Returns a tracker for response processing.
pub async fn stream_request_with_policy<R, W>(
    client_reader: &mut BufReader<R>,
    mut upstream_writer: W,
    content_length: Option<usize>,
    automaton: Arc<PlaceholderAutomaton>,
    config: StreamingPolicyConfig,
) -> Result<ReplacementTracker>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    if !config.enabled {
        return Ok(ReplacementTracker::new());
    }

    let domain = config.domain.as_str();

    // Read entire body into buffer
    let mut buffer = if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        client_reader.read_exact(&mut buf).await?;
        buf
    } else {
        let mut buf = Vec::new();
        client_reader.read_to_end(&mut buf).await?;
        buf
    };

    // Apply policy replacement with tracking
    let (tracker, replacements) = automaton.replace_placeholders(
        &mut buffer,
        domain,
        |_, _| true, // Domain checker - allow all for now
    );

    // Stream to upstream
    upstream_writer.write_all(&buffer).await?;
    upstream_writer.flush().await?;

    if config.debug && replacements > 0 {
        debug!("[policy] Request: {} placeholders replaced", replacements);
    }

    Ok(tracker)
}

/// Stream response with policy replacement
///
/// Buffers the response body, replaces policy secrets with placeholders.
pub async fn stream_response_with_policy<R, W>(
    upstream_reader: &mut BufReader<R>,
    mut client_writer: W,
    content_length: Option<usize>,
    automaton: Arc<PlaceholderAutomaton>,
    tracker: Arc<ReplacementTracker>,
    config: StreamingPolicyConfig,
) -> Result<usize>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    if !config.enabled || tracker.replacements().is_empty() {
        // Pass through without modification
        let mut buffer = if let Some(len) = content_length {
            let mut buf = vec![0u8; len];
            upstream_reader.read_exact(&mut buf).await?;
            buf
        } else {
            let mut buf = Vec::new();
            upstream_reader.read_to_end(&mut buf).await?;
            buf
        };
        client_writer.write_all(&buffer).await?;
        client_writer.flush().await?;
        return Ok(0);
    }

    // Read response body
    let mut buffer = if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        upstream_reader.read_exact(&mut buf).await?;
        buf
    } else {
        let mut buf = Vec::new();
        upstream_reader.read_to_end(&mut buf).await?;
        buf
    };

    // Replace tracked secrets with placeholders
    let replacements = automaton.replace_secrets(&mut buffer, &tracker);

    // Stream to client
    client_writer.write_all(&buffer).await?;
    client_writer.flush().await?;

    if config.debug && replacements > 0 {
        debug!("[policy] Response: {} secrets replaced", replacements);
    }

    Ok(replacements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scred_policy::PlaceholderGenerator;
    use std::collections::HashMap;
    use std::io::Cursor;

    fn build_test_automaton() -> (PlaceholderAutomaton, String) {
        let mut secrets = HashMap::new();
        secrets.insert("TEST_KEY".to_string(), "sk-test-secret-123".to_string());

        let mut generator = PlaceholderGenerator::new("test-seed");
        let automaton = PlaceholderAutomaton::build(&secrets, &mut generator).unwrap();

        let placeholder = generator.generate("TEST_KEY", "sk-test-secret-123");
        (automaton, placeholder.value.clone())
    }

    #[tokio::test]
    async fn test_streaming_request_replacement() {
        let (automaton, placeholder) = build_test_automaton();
        let automaton = Arc::new(automaton);

        let input = format!("Authorization: Bearer {}", placeholder);
        let mut reader = BufReader::new(Cursor::new(input.as_bytes()));
        let mut writer = Vec::new();

        let config = StreamingPolicyConfig {
            enabled: true,
            domain: "api.example.com".to_string(),
            debug: false,
        };

        let tracker = stream_request_with_policy(
            &mut reader,
            &mut writer,
            Some(input.len()),
            automaton,
            config,
        )
        .await
        .unwrap();

        let output = String::from_utf8_lossy(&writer);
        assert!(output.contains("sk-test-secret-123"));
        assert!(!output.contains(&placeholder));

        assert!(tracker.contains_secret("sk-test-secret-123"));
    }

    #[tokio::test]
    async fn test_streaming_response_replacement() {
        let (automaton, placeholder) = build_test_automaton();
        let automaton = Arc::new(automaton);

        let mut tracker = ReplacementTracker::new();
        tracker.track(
            "TEST_KEY".to_string(),
            "sk-test-secret-123".to_string(),
            placeholder.clone(),
        );
        let tracker = Arc::new(tracker);

        let input = "Your key: sk-test-secret-123".to_string();
        let mut reader = BufReader::new(Cursor::new(input.as_bytes()));
        let mut writer = Vec::new();

        let config = StreamingPolicyConfig {
            enabled: true,
            domain: "api.example.com".to_string(),
            debug: false,
        };

        let count = stream_response_with_policy(
            &mut reader,
            &mut writer,
            Some(input.len()),
            automaton,
            tracker,
            config,
        )
        .await
        .unwrap();

        assert_eq!(count, 1);

        let output = String::from_utf8_lossy(&writer);
        assert!(output.contains(&placeholder));
        assert!(!output.contains("sk-test-secret-123"));
    }
}
