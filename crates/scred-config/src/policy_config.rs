use crate::policy_types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    #![allow(clippy::unwrap_used)]
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

    #[test]
    fn test_per_host_wildcard_overrides_defaults() {
        // BUG FIX: Per-host wildcard should completely override defaults
        let defaults = HostPolicy::new()
            .with_header("Authorization", HeaderAction::Replace)
            .with_header("X-Api-Key", HeaderAction::Replace)
            .with_header("*", HeaderAction::Redact);

        let mut per_host_headers = HeaderRules::new();
        per_host_headers.add("*", HeaderAction::Redact);
        
        let per_host = HostPolicy {
            merge: MergeStrategy::Merge,
            headers: per_host_headers,
            body: BodyRules::default(),
            patterns: PatternFilter::default(),
        };

        let resolved = per_host.resolve(&defaults);

        // All headers should be redacted, not replaced
        assert_eq!(resolved.headers.resolve("Authorization"), HeaderAction::Redact);
        assert_eq!(resolved.headers.resolve("X-Api-Key"), HeaderAction::Redact);
        assert_eq!(resolved.headers.resolve("X-Something"), HeaderAction::Redact);
    }

    #[test]
    fn test_per_host_specific_rules_merge_with_defaults() {
        // Per-host with specific rules (no wildcard) should merge with defaults
        let defaults = HostPolicy::new()
            .with_header("Authorization", HeaderAction::Replace)
            .with_header("*", HeaderAction::Redact);

        let mut per_host_headers = HeaderRules::new();
        per_host_headers.add("X-Custom", HeaderAction::Passthrough);
        
        let per_host = HostPolicy {
            merge: MergeStrategy::Merge,
            headers: per_host_headers,
            body: BodyRules::default(),
            patterns: PatternFilter::default(),
        };

        let resolved = per_host.resolve(&defaults);

        // Should have both default and per-host rules
        assert_eq!(resolved.headers.resolve("Authorization"), HeaderAction::Replace);
        assert_eq!(resolved.headers.resolve("X-Custom"), HeaderAction::Passthrough);
        assert_eq!(resolved.headers.resolve("Content-Type"), HeaderAction::Redact);
    }
}
