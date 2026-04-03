//! Policy Engine
//!
//! Combines placeholder replacement and secret redaction into a single engine.
//!
//! # Key Features
//! - Per-header action control (replace, redact, detect, passthrough)
//! - Streaming body processing
//! - Pattern-based redaction filtering
//! - Host-specific policy resolution

use aho_corasick::AhoCorasick;
use std::collections::HashMap;
use std::sync::Arc;

use scred_config::{
    BodyAction, ConfigSource, HeaderAction, HostPolicy, PatternFilter,
    ResolvedPolicy, PolicyConfig,
};
use scred_redactor::{PatternMatch, RedactionEngine, RedactionConfig};

use crate::placeholder::PlaceholderGenerator;
use crate::streaming::PlaceholderAutomaton;
use crate::PolicyError;

/// Direction of data flow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Request,
    Response,
}

/// Result of processing headers
#[derive(Debug, Default)]
pub struct HeaderProcessingResult {
    /// Number of headers processed
    pub headers_processed: usize,
    /// Number of placeholders replaced
    pub placeholders_replaced: usize,
    /// Number of secrets redacted
    pub secrets_redacted: usize,
    /// Number of secrets detected (detect mode)
    pub secrets_detected: usize,
    /// Detections (for logging/audit)
    pub detections: Vec<DetectionEvent>,
}

/// Detection event for audit trail
#[derive(Debug, Clone)]
pub struct DetectionEvent {
    pub location: String,
    pub pattern_name: String,
    pub action_taken: String,
}

/// Result of processing body
#[derive(Debug, Default)]
pub struct BodyProcessingResult {
    /// Bytes processed
    pub bytes_processed: usize,
    /// Number of placeholders replaced
    pub placeholders_replaced: usize,
    /// Number of secrets redacted
    pub secrets_redacted: usize,
    /// Number of secrets detected
    pub secrets_detected: usize,
}

/// Policy engine combining redaction and placeholder replacement
pub struct PolicyEngine {
    /// Configuration
    config: PolicyConfig,
    /// Placeholder generator
    generator: PlaceholderGenerator,
    /// Aho-Corasick automaton for placeholder matching
    automaton: AhoCorasick,
    /// Placeholder → (secret, name) mapping
    replacements: Vec<(String, String)>,
    /// Placeholder values for reverse lookup
    placeholder_values: Vec<String>,
    /// Redaction engine
    redaction_engine: Arc<RedactionEngine>,
    /// Secrets map (name → value)
    secrets: HashMap<String, String>,
}

impl PolicyEngine {
    /// Build a new policy engine
    pub fn new(config: PolicyConfig) -> Result<Self, PolicyError> {
        // Expand seed
        let seed = config.expand_seed();

        // Create placeholder generator
        let mut generator = PlaceholderGenerator::new(&seed);

        // Collect secrets from providers
        let secrets = Self::collect_secrets(&config.providers)?;

        // Build Aho-Corasick automaton for placeholders
        let mut patterns: Vec<String> = Vec::new();
        let mut replacements: Vec<(String, String)> = Vec::new();
        let mut placeholder_values: Vec<String> = Vec::new();

        for (name, value) in &secrets {
            let placeholder = generator.generate(name, value);
            patterns.push(placeholder.value.clone());
            replacements.push((value.clone(), name.clone()));
            placeholder_values.push(placeholder.value.clone());
        }

        let automaton = if patterns.is_empty() {
            AhoCorasick::new(&[""])
                .map_err(|e| PolicyError::PatternError(e.to_string()))?
        } else {
            AhoCorasick::builder()
                .ascii_case_insensitive(false)
                .build(&patterns)
                .map_err(|e| PolicyError::PatternError(e.to_string()))?
        };

        // Create redaction engine
        let redaction_engine = Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }));

        Ok(Self {
            config,
            generator,
            automaton,
            replacements,
            placeholder_values,
            redaction_engine,
            secrets,
        })
    }

    /// Collect secrets from providers
    /// Collect secrets from providers
    fn collect_secrets(
        providers: &[scred_config::ProviderConfig],
    ) -> Result<HashMap<String, String>, PolicyError> {
        let mut secrets = HashMap::new();
        let mut value_to_keys: HashMap<String, Vec<String>> = HashMap::new();

        for provider in providers {
            match provider {
                scred_config::ProviderConfig::Env { keys } => {
                    for pattern in keys {
                        // Glob match environment variables
                        for (key, value) in std::env::vars() {
                            if glob_match(pattern, &key) {
                                // Track which keys have each value (for collision detection)
                                value_to_keys
                                    .entry(value.clone())
                                    .or_insert_with(Vec::new)
                                    .push(key.clone());
                                secrets.insert(key, value);
                            }
                        }
                    }
                }
            }
        }

        // Check for value collisions (different keys with same value)
        for (_value, keys) in &value_to_keys {
            if keys.len() > 1 {
                return Err(PolicyError::ValueCollision(format!(
                    "Multiple secrets have the same value: {}",
                    keys.join(", ")
                )));
            }
        }

        Ok(secrets)
    }

    /// Get secrets map for discovery API
    pub fn secrets(&self) -> &HashMap<String, String> {
        &self.secrets
    }

    /// Get placeholders for discovery API
    pub fn placeholders(&self) -> HashMap<String, String> {
        // Regenerate placeholders
        let mut generator = self.generator.clone();
        self.secrets
            .iter()
            .map(|(name, value)| {
                let placeholder = generator.generate(name, value);
                (name.clone(), placeholder.value.clone())
            })
            .collect()
    }

    /// Get discovery port
    pub fn discovery_port(&self) -> u16 {
        self.config.discovery.port
    }

    /// Check if discovery is enabled
    pub fn discovery_enabled(&self) -> bool {
        self.config.discovery.enabled
    }

    /// Run the discovery server (spawns a background task)
    /// Returns a handle to update placeholders
    pub fn run_discovery(&self) -> Option<crate::discovery::DiscoveryUpdater> {
        if !self.config.discovery.enabled {
            return None;
        }

        use crate::discovery::{DiscoveryConfig, DiscoveryServer};
        use crate::placeholder::Placeholder;

        let server = DiscoveryServer::new(DiscoveryConfig {
            port: self.config.discovery.port,
            bind: "0.0.0.0".to_string(),
        });

        let updater = server.updater();

        // Update placeholders
        let placeholders: std::collections::HashMap<String, Placeholder> = self
            .placeholders()
            .into_iter()
            .map(|(k, v)| {
                (k.clone(), Placeholder {
                    name: k,
                    value: v,
                    prefix: "".to_string(),
                })
            })
            .collect();
        updater.update(placeholders);

        // Spawn the server
        let server_clone = server.clone();
        tokio::spawn(async move {
            if let Err(e) = server_clone.run().await {
                tracing::error!("Discovery server error: {}", e);
            }
        });

        Some(updater)
    }

    /// Resolve effective policy for a host
    pub fn resolve_for_host(&self, host: &str) -> ResolvedPolicy {
        self.config.resolve_for_host(host)
    }

    /// Get the automaton for streaming replacement
    pub fn automaton(&self) -> &AhoCorasick {
        &self.automaton
    }

    /// Create a PlaceholderAutomaton for use with streaming APIs
    ///
    /// This allows using the `replace_placeholders` method with domain checking.
    pub fn create_placeholder_automaton(&self) -> PlaceholderAutomaton {
        PlaceholderAutomaton::from_parts(
            self.automaton.clone(),
            self.replacements.clone(),
            self.placeholder_values.clone(),
        )
    }

    /// Process HTTP headers according to policy
    ///
    /// For each header:
    /// - `Replace`: Replace placeholders with real secrets
    /// - `Redact`: Redact detected secrets
    /// - `Detect`: Log detections without modifying
    /// - `Passthrough`: No processing
    pub fn process_headers(
        &self,
        headers: &mut http::HeaderMap,
        host: &str,
    ) -> Result<HeaderProcessingResult, PolicyError> {
        let resolved = self.resolve_for_host(host);
        let mut result = HeaderProcessingResult::default();

        for (header_name, header_value) in headers.iter_mut() {
            let action = resolved.header_action(header_name.as_str());
            let value_str = header_value.to_str().unwrap_or("");

            match action {
                HeaderAction::Replace => {
                    // Replace placeholders with real secrets
                    let mut value_bytes = value_str.as_bytes().to_vec();
                    let count = self.replace_placeholders(&mut value_bytes);

                    if count > 0 {
                        *header_value = http::HeaderValue::from_bytes(&value_bytes)
                            .unwrap_or_else(|_| header_value.clone());
                        result.placeholders_replaced += count;
                        tracing::info!(
                            "[policy] Replaced {} placeholder(s) in header: {}",
                            count,
                            header_name
                        );
                    }
                }
                HeaderAction::Redact => {
                    // Redact detected secrets
                    let redacted = self.redaction_engine.redact(value_str);
                    let match_count = redacted.matches.len();

                    if match_count > 0 {
                        *header_value = http::HeaderValue::from_str(&redacted.redacted)
                            .unwrap_or_else(|_| header_value.clone());
                        result.secrets_redacted += match_count;

                        for m in &redacted.matches {
                            result.detections.push(DetectionEvent {
                                location: format!("header:{}", header_name),
                                pattern_name: m.pattern_type.clone(),
                                action_taken: "redacted".to_string(),
                            });
                        }

                        tracing::debug!(
                            "[policy] Redacted {} secret(s) in header: {}",
                            match_count,
                            header_name
                        );
                    }
                }
                HeaderAction::Detect => {
                    // Detect without modifying
                    let redacted = self.redaction_engine.redact(value_str);
                    for m in &redacted.matches {
                        result.secrets_detected += 1;
                        result.detections.push(DetectionEvent {
                            location: format!("header:{}", header_name),
                            pattern_name: m.pattern_type.clone(),
                            action_taken: "detected".to_string(),
                        });

                        tracing::info!(
                            "[policy] Detected {} in header: {}",
                            m.pattern_type,
                            header_name
                        );
                    }
                }
                HeaderAction::Passthrough => {
                    // No processing
                }
            }

            result.headers_processed += 1;
        }

        Ok(result)
    }

    /// Process body according to policy
    pub fn process_body(
        &self,
        body: &mut [u8],
        host: &str,
        direction: Direction,
    ) -> Result<BodyProcessingResult, PolicyError> {
        let resolved = self.resolve_for_host(host);
        let action = match direction {
            Direction::Request => resolved.request_body_action(),
            Direction::Response => resolved.response_body_action(),
        };

        let mut result = BodyProcessingResult::default();
        result.bytes_processed = body.len();

        match action {
            BodyAction::Redact => {
                // Redact detected secrets
                let body_str = String::from_utf8_lossy(body);
                let redacted = self.redaction_engine.redact(&body_str);
                let match_count = redacted.matches.len();

                if match_count > 0 {
                    let redacted_bytes = redacted.redacted.into_bytes();
                    let copy_len = std::cmp::min(body.len(), redacted_bytes.len());
                    body[..copy_len].copy_from_slice(&redacted_bytes[..copy_len]);
                    result.secrets_redacted = match_count;

                    tracing::debug!(
                        "[policy] Redacted {} secret(s) in {} body",
                        match_count,
                        direction_str(direction)
                    );
                }
            }
            BodyAction::Detect => {
                // Detect without modifying
                let body_str = String::from_utf8_lossy(body);
                let redacted = self.redaction_engine.redact(&body_str);

                result.secrets_detected = redacted.matches.len();
                for m in &redacted.matches {
                    tracing::info!(
                        "[policy] Detected {} in {} body",
                        m.pattern_type,
                        direction_str(direction)
                    );
                }
            }
            BodyAction::Passthrough => {
                // No processing
            }
        }

        Ok(result)
    }

    /// Replace placeholders in a buffer (in-place)
    ///
    /// Returns number of replacements made
    fn replace_placeholders(&self, data: &mut Vec<u8>) -> usize {
        if self.placeholder_values.is_empty() {
            return 0;
        }

        // Find all matches
        let data_slice: &[u8] = data.as_slice();
        let matches: Vec<_> = self.automaton.find_iter(data_slice).collect();

        if matches.is_empty() {
            return 0;
        }

        // Build replacement string
        let data_str = String::from_utf8_lossy(data);
        let mut result = String::new();
        let mut last_end = 0;

        for m in &matches {
            // Add content before match
            result.push_str(&data_str[last_end..m.start()]);

            // Add replacement
            let idx = m.pattern().as_usize();
            if let Some((secret, _name)) = self.replacements.get(idx) {
                result.push_str(secret);
            } else {
                // Fallback: keep original
                result.push_str(&data_str[m.start()..m.end()]);
            }

            last_end = m.end();
        }

        // Add remaining content
        result.push_str(&data_str[last_end..]);

        *data = result.into_bytes();
        matches.len()
    }

    /// Check if engine has any placeholders
    pub fn has_placeholders(&self) -> bool {
        !self.placeholder_values.is_empty()
    }

    /// Get redaction engine
    pub fn redaction_engine(&self) -> &Arc<RedactionEngine> {
        &self.redaction_engine
    }
}

/// Convert direction to string
fn direction_str(direction: Direction) -> &'static str {
    match direction {
        Direction::Request => "request",
        Direction::Response => "response",
    }
}

/// Simple glob matching
fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Handle prefix*
    if pattern.ends_with('*') && !pattern[..pattern.len() - 1].contains('*') {
        let prefix = &pattern[..pattern.len() - 1];
        return name.starts_with(prefix);
    }

    // Handle *suffix
    if pattern.starts_with('*') && !pattern[1..].contains('*') {
        let suffix = &pattern[1..];
        return name.ends_with(suffix);
    }

    // Exact match
    pattern == name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*_API_KEY", "OPENAI_API_KEY"));
        assert!(glob_match("APP_*", "APP_SECRET"));
        assert!(!glob_match("APP_*", "OTHER_SECRET"));
    }

    #[test]
    #[test]
    fn test_process_headers_passthrough() {
        let config = PolicyConfig {
            enabled: false,
            providers: vec![],
            ..Default::default()
        };
        let engine = PolicyEngine::new(config).unwrap();

        let mut headers = http::HeaderMap::new();
        headers.insert("Content-Type", http::HeaderValue::from_static("application/json"));

        // Default policy should process headers
        let result = engine.process_headers(&mut headers, "example.com").unwrap();
        assert!(result.headers_processed > 0);
    }

    #[test]
    fn test_process_body_passthrough() {
        let config = PolicyConfig {
            enabled: false,
            providers: vec![],
            ..Default::default()
        };
        let engine = PolicyEngine::new(config).unwrap();

        let mut body = b"{\"message\": \"hello\"}".to_vec();
        let result = engine.process_body(&mut body, "example.com", Direction::Request).unwrap();
        assert!(result.bytes_processed > 0);
    }

    #[test]
    fn test_resolve_policy_default() {
        let config = PolicyConfig {
            enabled: false,
            providers: vec![],
            ..Default::default()
        };
        let engine = PolicyEngine::new(config).unwrap();

        // Should resolve to default for unknown host
        let resolved = engine.resolve_for_host("unknown.example.com");
        assert!(matches!(resolved.source, ConfigSource::Default));
    }

    #[test]
    fn test_header_action_resolution() {
        use scred_config::{HeaderRules, HostPolicy};

        // Create config with specific header rules
        let defaults = HostPolicy {
            merge: scred_config::MergeStrategy::Merge,
            headers: HeaderRules::default(),
            body: scred_config::BodyRules::default(),
            patterns: PatternFilter::default(),
        };

        let config = PolicyConfig {
            enabled: false,
            providers: vec![],
            defaults,
            ..Default::default()
        };

        let engine = PolicyEngine::new(config).unwrap();

        // Check Authorization gets Replace action by default
        let resolved = engine.resolve_for_host("example.com");
        assert_eq!(resolved.header_action("Authorization"), HeaderAction::Replace);
        assert_eq!(resolved.header_action("Content-Type"), HeaderAction::Redact);
    }

    #[test]
    fn test_value_collision_detection() {
        std::env::set_var("SCRED_UNIFIED_TEST_KEY_A", "same-collision-value");
        std::env::set_var("SCRED_UNIFIED_TEST_KEY_B", "same-collision-value");

        let config = PolicyConfig {
            enabled: true,
            providers: vec![scred_config::ProviderConfig::Env {
                keys: vec![
                    "SCRED_UNIFIED_TEST_KEY_A".to_string(),
                    "SCRED_UNIFIED_TEST_KEY_B".to_string(),
                ],
            }],
            ..Default::default()
        };

        let result = PolicyEngine::new(config);
        assert!(result.is_err(), "Expected error for value collision");

        if let Err(e) = result {
            let err_str = e.to_string();
            assert!(
                err_str.contains("SCRED_UNIFIED_TEST_KEY_A") || err_str.contains("same-collision-value"),
                "Error should mention the colliding keys: {}",
                err_str
            );
        }

        std::env::remove_var("SCRED_UNIFIED_TEST_KEY_A");
        std::env::remove_var("SCRED_UNIFIED_TEST_KEY_B");
    }
}
