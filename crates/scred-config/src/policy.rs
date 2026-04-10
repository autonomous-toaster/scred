//! Policy Configuration
//!
//! Single configuration system that combines:
//! - Placeholder replacement (policy)
//! - Secret detection and redaction
//!
//! Key feature: Per-header action control
//! - "Authorization: replace" - Replace placeholders with secrets
//! - "*: redact" - Redact detected secrets in all other headers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// ACTION TYPES
// =============================================================================

/// Action to take on a header
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeaderAction {
    /// Replace detected secrets with [REDACTED]
    Redact,

    /// Replace placeholders with real secrets
    Replace,

    /// Log detections but don't modify
    Detect,

    /// No processing (pass through as-is)
    Passthrough,
}

impl Default for HeaderAction {
    fn default() -> Self {
        Self::Redact
    }
}

impl std::fmt::Display for HeaderAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Redact => write!(f, "redact"),
            Self::Replace => write!(f, "replace"),
            Self::Detect => write!(f, "detect"),
            Self::Passthrough => write!(f, "passthrough"),
        }
    }
}

/// Action to take on request/response body
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyAction {
    /// Redact detected secrets
    Redact,

    /// Log detections but don't modify
    Detect,

    /// No processing
    Passthrough,
}

impl Default for BodyAction {
    fn default() -> Self {
        Self::Redact
    }
}

impl std::fmt::Display for BodyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Redact => write!(f, "redact"),
            Self::Detect => write!(f, "detect"),
            Self::Passthrough => write!(f, "passthrough"),
        }
    }
}

// =============================================================================
// HEADER RULES
// =============================================================================

/// Rules for processing headers
///
/// Supports three matching types:
/// 1. Exact match: `Authorization` matches only that header
/// 2. Prefix match: `X-*` matches all headers starting with `X-`
/// 3. Wildcard: `*` matches all headers
///
/// Precedence: exact > longest prefix > wildcard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HeaderRules {
    /// Header rules as (pattern, action) pairs
    /// Pattern can be:
    /// - Exact: "Authorization"
    /// - Prefix: "X-*"
    /// - Wildcard: "*"
    #[serde(flatten)]
    pub rules: HashMap<String, HeaderAction>,
}

impl Default for HeaderRules {
    fn default() -> Self {
        let mut rules = HashMap::new();
        rules.insert("Authorization".to_string(), HeaderAction::Replace);
        rules.insert("*".to_string(), HeaderAction::Redact);
        Self { rules }
    }
}

impl HeaderRules {
    /// Create empty rules (use defaults)
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Create rules with specific mappings
    pub fn with_rules(rules: HashMap<String, HeaderAction>) -> Self {
        Self { rules }
    }

    /// Resolve action for a header name
    ///
    /// Matching precedence:
    /// 1. Exact match (e.g., "Authorization")
    /// 2. Longest prefix match (e.g., "X-*" matches "X-Api-Key")
    /// 3. Wildcard "*" match
    /// 4. Default to Redact
    pub fn resolve(&self, header_name: &str) -> HeaderAction {
        // Normalize header name (HTTP headers are case-insensitive)
        let normalized = header_name.to_lowercase();

        // 1. Try exact match
        for (pattern, action) in &self.rules {
            if pattern.to_lowercase() == normalized {
                return *action;
            }
        }

        // 2. Try prefix matches (find longest match)
        let mut best_match: Option<(&str, HeaderAction)> = None;
        for (pattern, action) in &self.rules {
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                if normalized.starts_with(&prefix.to_lowercase()) {
                    match best_match {
                        None => best_match = Some((pattern, *action)),
                        Some((existing, _)) if pattern.len() > existing.len() => {
                            best_match = Some((pattern, *action));
                        }
                        _ => {}
                    }
                }
            }
        }
        if let Some((_, action)) = best_match {
            return action;
        }

        // 3. Try wildcard
        if let Some(action) = self.rules.get("*") {
            return *action;
        }

        // 4. Default to redact
        HeaderAction::Redact
    }

    /// Add a rule
    pub fn add(&mut self, pattern: impl Into<String>, action: HeaderAction) {
        self.rules.insert(pattern.into(), action);
    }

    /// Merge with another set of rules (other takes precedence)
    pub fn merge(&self, other: &HeaderRules) -> HeaderRules {
        let mut merged = self.clone();
        for (pattern, action) in &other.rules {
            merged.rules.insert(pattern.clone(), *action);
        }
        merged
    }
}

// =============================================================================
// BODY RULES
// =============================================================================

/// Rules for processing request/response bodies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BodyRules {
    /// Action for request body
    #[serde(default)]
    pub request: BodyAction,

    /// Action for response body
    #[serde(default)]
    pub response: BodyAction,
}

impl Default for BodyRules {
    fn default() -> Self {
        Self {
            request: BodyAction::Redact,
            response: BodyAction::Redact,
        }
    }
}

impl BodyRules {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_request(mut self, action: BodyAction) -> Self {
        self.request = action;
        self
    }

    pub fn with_response(mut self, action: BodyAction) -> Self {
        self.response = action;
        self
    }
}

// =============================================================================
// PATTERN FILTER
// =============================================================================

/// Pattern filter for redaction
///
/// Controls which detected patterns are redacted vs kept.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PatternFilter {
    /// Patterns to redact (glob supported)
    /// ["*"] = redact all patterns
    /// ["aws-*"] = redact only AWS patterns
    #[serde(default = "default_redact_all")]
    pub redact: Vec<String>,

    /// Patterns to keep visible (not redacted)
    /// Applied after redact filter
    #[serde(default)]
    pub keep: Vec<String>,
}

fn default_redact_all() -> Vec<String> {
    vec!["*".to_string()]
}

impl Default for PatternFilter {
    fn default() -> Self {
        Self {
            redact: vec!["*".to_string()],
            keep: Vec::new(),
        }
    }
}

impl PatternFilter {
    /// Match all patterns
    pub fn all() -> Self {
        Self {
            redact: vec!["*".to_string()],
            keep: Vec::new(),
        }
    }

    /// Match no patterns
    pub fn none() -> Self {
        Self {
            redact: Vec::new(),
            keep: Vec::new(),
        }
    }

    /// Check if a pattern name matches this filter
    pub fn matches(&self, pattern_name: &str) -> bool {
        // Check keep first (patterns to preserve)
        for keep_pattern in &self.keep {
            if glob_match(keep_pattern, pattern_name) {
                return false;
            }
        }

        // Check redact patterns
        for redact_pattern in &self.redact {
            if glob_match(redact_pattern, pattern_name) {
                return true;
            }
        }

        false
    }

    /// Merge with another filter
    pub fn merge(&self, other: &PatternFilter) -> PatternFilter {
        let mut merged = self.clone();
        for pattern in &other.redact {
            if !merged.redact.contains(pattern) {
                merged.redact.push(pattern.clone());
            }
        }
        for pattern in &other.keep {
            if !merged.keep.contains(pattern) {
                merged.keep.push(pattern.clone());
            }
        }
        merged
    }
}

/// Simple glob matching: * matches any chars
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

    // Handle prefix*suffix
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 2 {
        return name.starts_with(parts[0]) && name.ends_with(parts[1]);
    }

    // Exact match
    pattern == name
}

// =============================================================================
// HOST POLICY
// =============================================================================

/// Merge strategy for host-specific overrides
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    /// Merge with defaults (additive)
    Merge,

    /// Replace defaults completely
    Replace,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        Self::Merge
    }
}

/// Policy for a specific host
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HostPolicy {
    /// How to merge with defaults
    #[serde(default)]
    pub merge: MergeStrategy,

    /// Header processing rules
    #[serde(default)]
    pub headers: HeaderRules,

    /// Body processing rules
    #[serde(default)]
    pub body: BodyRules,

    /// Pattern filter for redaction
    #[serde(default)]
    pub patterns: PatternFilter,
}

impl Default for HostPolicy {
    fn default() -> Self {
        Self {
            merge: MergeStrategy::Merge,
            headers: HeaderRules::default(),
            body: BodyRules::default(),
            patterns: PatternFilter::default(),
        }
    }
}

impl HostPolicy {
    /// Create a new host policy
    pub fn new() -> Self {
        Self::default()
    }

    /// Set merge strategy
    pub fn with_merge(mut self, strategy: MergeStrategy) -> Self {
        self.merge = strategy;
        self
    }

    /// Add header rule
    pub fn with_header(mut self, pattern: impl Into<String>, action: HeaderAction) -> Self {
        self.headers.add(pattern, action);
        self
    }

    /// Set request body action
    pub fn with_request_body(mut self, action: BodyAction) -> Self {
        self.body.request = action;
        self
    }

    /// Set response body action
    pub fn with_response_body(mut self, action: BodyAction) -> Self {
        self.body.response = action;
        self
    }

    /// Resolve effective policy by merging with defaults
    pub fn resolve(&self, defaults: &HostPolicy) -> HostPolicy {
        match self.merge {
            MergeStrategy::Replace => self.clone(),
            MergeStrategy::Merge => {
                // Merge headers: self takes precedence for specific rules
                let headers = if self.headers.rules.is_empty() {
                    defaults.headers.clone()
                } else {
                    defaults.headers.merge(&self.headers)
                };
                
                HostPolicy {
                    merge: MergeStrategy::Merge,
                    headers,
                    body: BodyRules {
                        request: self.body.request,
                        response: self.body.response,
                    },
                    patterns: defaults.patterns.merge(&self.patterns),
                }
            }
        }
    }
}

// =============================================================================
// PROVIDER CONFIG
// =============================================================================

/// Secret provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProviderConfig {
    /// Environment variable provider
    Env {
        /// Keys to expose (supports glob patterns)
        #[serde(default)]
        keys: Vec<String>,
    },
}

// =============================================================================
// DISCOVERY CONFIG
// =============================================================================

fn default_true() -> bool { true }

/// Discovery API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DiscoveryConfig {
    /// Enable discovery API
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Port for discovery API
    #[serde(default = "default_discovery_port")]
    pub port: u16,
}

fn default_discovery_port() -> u16 {
    9998
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9998,
        }
    }
}

// =============================================================================
// UNIFIED POLICY CONFIG
// =============================================================================

/// Policy configuration
///
/// Single configuration for both:
/// - Placeholder replacement (policy)
/// - Secret detection and redaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PolicyConfig {
    /// Enable policy feature
    #[serde(default)]
    pub enabled: bool,

    /// Stable seed for deterministic placeholder generation
    #[serde(default = "default_seed")]
    pub seed: String,

    /// Secret providers
    #[serde(default = "default_providers")]
    pub providers: Vec<ProviderConfig>,

    /// Discovery API configuration
    #[serde(default)]
    pub discovery: DiscoveryConfig,

    /// Default rules (applied to all hosts)
    #[serde(default)]
    pub defaults: HostPolicy,

    /// Host-specific overrides
    #[serde(default)]
    pub hosts: HashMap<String, HostPolicy>,
}

fn default_seed() -> String {
    "scred-policy-seed".to_string()
}

fn default_providers() -> Vec<ProviderConfig> {
    vec![ProviderConfig::Env {
        keys: vec![
            "*_API_KEY".to_string(),
            "*_SECRET".to_string(),
            "*_TOKEN".to_string(),
        ],
    }]
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            seed: default_seed(),
            providers: default_providers(),
            discovery: DiscoveryConfig::default(),
            defaults: HostPolicy::default(),
            hosts: HashMap::new(),
        }
    }
}

impl PolicyConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set default policy
    pub fn with_defaults(mut self, defaults: HostPolicy) -> Self {
        self.defaults = defaults;
        self
    }

    /// Enable policy
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Set seed
    pub fn with_seed(mut self, seed: impl Into<String>) -> Self {
        self.seed = seed.into();
        self
    }

    /// Add provider
    pub fn with_provider(mut self, provider: ProviderConfig) -> Self {
        self.providers.push(provider);
        self
    }

    /// Add host policy
    pub fn with_host(mut self, pattern: impl Into<String>, policy: HostPolicy) -> Self {
        self.hosts.insert(pattern.into(), policy);
        self
    }

    /// Expand environment variables in seed
    pub fn expand_seed(&self) -> String {
        if self.seed.starts_with("${") && self.seed.ends_with('}') {
            let var_name = &self.seed[2..self.seed.len() - 1];
            std::env::var(var_name).unwrap_or_else(|_| default_seed())
        } else if self.seed.starts_with("$") {
            let var_name = &self.seed[1..];
            std::env::var(var_name).unwrap_or_else(|_| default_seed())
        } else {
            self.seed.clone()
        }
    }

    /// Resolve effective policy for a host
    pub fn resolve_for_host(&self, host: &str) -> ResolvedPolicy {
        // Find matching host policy
        for (pattern, policy) in &self.hosts {
            if glob_match(pattern, host) {
                let resolved = policy.resolve(&self.defaults);
                return ResolvedPolicy {
                    policy: resolved,
                    source: ConfigSource::HostPattern(pattern.clone()),
                };
            }
        }

        // Use defaults
        ResolvedPolicy {
            policy: self.defaults.clone(),
            source: ConfigSource::Default,
        }
    }
}

// =============================================================================
// RESOLVED POLICY
// =============================================================================

/// Source of resolved policy
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Default policy
    Default,
    /// Host-specific policy
    HostPattern(String),
}

/// Resolved policy for a host
#[derive(Debug, Clone)]
pub struct ResolvedPolicy {
    /// Effective policy
    pub policy: HostPolicy,
    /// Source of the policy
    pub source: ConfigSource,
}

impl ResolvedPolicy {
    /// Get header action for a header name
    pub fn header_action(&self, header_name: &str) -> HeaderAction {
        self.policy.headers.resolve(header_name)
    }

    /// Get request body action
    pub fn request_body_action(&self) -> BodyAction {
        self.policy.body.request
    }

    /// Get response body action
    pub fn response_body_action(&self) -> BodyAction {
        self.policy.body.response
    }

    /// Check if pattern should be redacted
    pub fn should_redact(&self, pattern_name: &str) -> bool {
        self.policy.patterns.matches(pattern_name)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_action_resolve_exact() {
        let mut rules = HeaderRules::new();
        rules.add("Authorization", HeaderAction::Replace);
        rules.add("X-Api-Key", HeaderAction::Replace);
        rules.add("*", HeaderAction::Redact);

        assert_eq!(rules.resolve("Authorization"), HeaderAction::Replace);
        assert_eq!(rules.resolve("authorization"), HeaderAction::Replace); // case-insensitive
        assert_eq!(rules.resolve("X-Api-Key"), HeaderAction::Replace);
        assert_eq!(rules.resolve("Content-Type"), HeaderAction::Redact);
    }

    #[test]
    fn test_header_action_resolve_prefix() {
        let mut rules = HeaderRules::new();
        rules.add("X-*", HeaderAction::Passthrough);
        rules.add("X-Secret-*", HeaderAction::Redact);
        rules.add("*", HeaderAction::Detect);

        // More specific prefix wins
        assert_eq!(rules.resolve("X-Secret-Key"), HeaderAction::Redact);
        assert_eq!(rules.resolve("X-Public-Key"), HeaderAction::Passthrough);
        assert_eq!(rules.resolve("Content-Type"), HeaderAction::Detect);
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*_API_KEY", "OPENAI_API_KEY"));
        assert!(glob_match("APP_*", "APP_SECRET"));
        assert!(glob_match("*.openai.com", "api.openai.com"));
        assert!(glob_match("*.openai.com", "v1.api.openai.com"));
        assert!(!glob_match("*.openai.com", "api.github.com"));
    }

    #[test]
    fn test_pattern_filter() {
        let filter = PatternFilter {
            redact: vec!["*".to_string()],
            keep: vec!["public-*".to_string()],
        };

        assert!(filter.matches("aws-secret"));
        assert!(!filter.matches("public-key"));
        assert!(filter.matches("github-token"));
    }

    #[test]
    fn test_host_policy_merge() {
        let defaults = HostPolicy::new()
            .with_header("Authorization", HeaderAction::Replace)
            .with_header("*", HeaderAction::Redact);

        let override_policy = HostPolicy::new()
            .with_merge(MergeStrategy::Merge)
            .with_header("X-Custom", HeaderAction::Passthrough);

        let resolved = override_policy.resolve(&defaults);

        // Should have both default and override rules
        assert_eq!(resolved.headers.resolve("Authorization"), HeaderAction::Replace);
        assert_eq!(resolved.headers.resolve("X-Custom"), HeaderAction::Passthrough);
        assert_eq!(resolved.headers.resolve("Other"), HeaderAction::Redact);
    }

    #[test]
    fn test_host_policy_replace() {
        let defaults = HostPolicy::new()
            .with_header("Authorization", HeaderAction::Replace)
            .with_header("*", HeaderAction::Redact);

        // Replace strategy with explicit empty headers
        let mut override_headers = HeaderRules::new();
        override_headers.add("*", HeaderAction::Passthrough);
        
        let override_policy = HostPolicy {
            merge: MergeStrategy::Replace,
            headers: override_headers,
            body: BodyRules::new(),
            patterns: PatternFilter::default(),
        };

        let resolved = override_policy.resolve(&defaults);

        // Should only have override rules
        assert_eq!(resolved.headers.resolve("Authorization"), HeaderAction::Passthrough);
        assert_eq!(resolved.headers.resolve("Anything"), HeaderAction::Passthrough);
    }

    #[test]
    fn test_resolve_for_host() {
        // Create defaults with specific headers
        let defaults = HostPolicy {
            merge: MergeStrategy::Merge,
            headers: HeaderRules::default(),
            body: BodyRules::default(),
            patterns: PatternFilter::default(),
        };
        
        let config = PolicyConfig::new()
            .enable()
            .with_defaults(defaults)
            .with_host(
                "*.openai.com",
                HostPolicy::new().with_header("Authorization", HeaderAction::Replace),
            )
            .with_host(
                "api.github.com",
                HostPolicy {
                    merge: MergeStrategy::Replace,
                    headers: {
                        let mut h = HeaderRules::new();
                        h.add("*", HeaderAction::Detect);
                        h
                    },
                    body: BodyRules::new(),
                    patterns: PatternFilter::default(),
                },
            );

        // Match openai
        let resolved = config.resolve_for_host("api.openai.com");
        assert!(matches!(resolved.source, ConfigSource::HostPattern(_)));
        assert_eq!(resolved.header_action("Authorization"), HeaderAction::Replace);

        // Match github exactly - should use REPLACE strategy
        let resolved = config.resolve_for_host("api.github.com");
        assert_eq!(resolved.header_action("Authorization"), HeaderAction::Detect);

        // No match - use defaults
        let resolved = config.resolve_for_host("api.example.com");
        assert!(matches!(resolved.source, ConfigSource::Default));
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml = r#"
enabled: true
seed: "my-seed"
providers:
  - type: env
    keys:
      - "*_API_KEY"
      - "*_SECRET"
defaults:
  headers:
    Authorization: replace
    "X-*": passthrough
    "*": redact
  body:
    request: redact
    response: redact
  patterns:
    redact: ["*"]
    keep: ["public-*"]
hosts:
  "*.openai.com":
    merge: merge
    headers:
      Authorization: replace
    body:
      request: redact
      response: redact
"#;

        let config: PolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.seed, "my-seed");
        assert!(config.hosts.contains_key("*.openai.com"));

        // Test resolve
        let resolved = config.resolve_for_host("api.openai.com");
        assert_eq!(resolved.header_action("Authorization"), HeaderAction::Replace);
        assert_eq!(resolved.header_action("X-Custom"), HeaderAction::Passthrough);
        assert_eq!(resolved.header_action("Content-Type"), HeaderAction::Redact);
    }

    #[test]
    fn test_case_insensitive_headers() {
        let mut rules = HeaderRules::new();
        rules.add("Authorization", HeaderAction::Replace);

        // HTTP headers are case-insensitive
        assert_eq!(rules.resolve("authorization"), HeaderAction::Replace);
        assert_eq!(rules.resolve("AUTHORIZATION"), HeaderAction::Replace);
        assert_eq!(rules.resolve("Authorization"), HeaderAction::Replace);
    }
}
