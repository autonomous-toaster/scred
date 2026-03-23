use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use scred_http::secrets::SecretsConfig;
use scred_http::PatternSelector;

/// Redaction mode for handling detected secrets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedactionMode {
    /// PASSTHROUGH: No detection, no redaction - just forward
    Passthrough,
    /// DETECT: Detect and log secrets, but don't redact (pass-through mode with logging)
    DetectOnly,
    /// REDACT: Detect, log, and redact secrets
    Redact,
}

impl RedactionMode {
    pub fn should_detect(&self) -> bool {
        matches!(self, RedactionMode::DetectOnly | RedactionMode::Redact)
    }

    pub fn should_redact(&self) -> bool {
        matches!(self, RedactionMode::Redact)
    }
}

/// MITM-specific configuration (TLS + upstream proxy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub proxy: UpstreamConfig,
    pub tls: TlsConfig,
    pub secrets: SecretsConfig,
    pub logging: LoggingConfig,
}

/// Upstream proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub listen: String,
    pub upstream_timeout: String,
    pub max_connections: usize,
    #[serde(default = "default_redaction_mode")]
    pub redaction_mode: RedactionMode,
    #[serde(default = "default_h2_redact_headers")]
    pub h2_redact_headers: bool,
    /// Pattern detection selector (which patterns to detect)
    /// NOTE: Not serialized/deserialized - handled via CLI flags and env vars
    #[serde(skip)]
    pub detect_patterns: PatternSelector,
    /// Pattern redaction selector (which patterns to redact)
    /// NOTE: Not serialized/deserialized - handled via CLI flags and env vars
    #[serde(skip)]
    pub redact_patterns: PatternSelector,
}

impl UpstreamConfig {
    /// Initialize pattern selectors from defaults
    /// Can be overridden by CLI flags or env vars
    pub fn init_patterns(&mut self) {
        self.detect_patterns = PatternSelector::default_detect();
        self.redact_patterns = PatternSelector::default_redact();
    }

    /// Update detect patterns from string (CLI flag or env var)
    pub fn set_detect_patterns(&mut self, input: &str) -> Result<(), String> {
        self.detect_patterns = PatternSelector::from_str(input)?;
        Ok(())
    }

    /// Update redact patterns from string (CLI flag or env var)
    pub fn set_redact_patterns(&mut self, input: &str) -> Result<(), String> {
        self.redact_patterns = PatternSelector::from_str(input)?;
        Ok(())
    }
}

/// TLS certificate configuration (MITM-specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub ca_key: PathBuf,
    pub ca_cert: PathBuf,
    pub cert_cache_dir: PathBuf,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
}

fn default_redaction_mode() -> RedactionMode {
    RedactionMode::DetectOnly  // Default: detect secrets, don't redact
}

fn default_h2_redact_headers() -> bool {
    true
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Self {
            proxy: UpstreamConfig {
                listen: "127.0.0.1:8080".to_string(),
                upstream_timeout: "30s".to_string(),
                max_connections: 1000,
                redaction_mode: RedactionMode::DetectOnly,
                h2_redact_headers: true,
                detect_patterns: PatternSelector::default_detect(),
                redact_patterns: PatternSelector::default_redact(),
            },
            tls: TlsConfig {
                ca_key: PathBuf::from(home_dir()).join(".scred/ca.key"),
                ca_cert: PathBuf::from(home_dir()).join(".scred/ca.pem"),
                cert_cache_dir: PathBuf::from("/tmp/scred-certs"),
            },
            secrets: SecretsConfig::default(),
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                output: "stderr".to_string(),
            },
        };
        config.proxy.init_patterns();
        config
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Self::default();

        // Try to load from config file
        if let Ok(file_config) = Self::try_load_from_file() {
            config = file_config;
        }

        // Override with environment variables
        config.apply_env_overrides();

        Ok(config)
    }

    fn try_load_from_file() -> anyhow::Result<Self> {
        let home = home_dir();

        // Check for SCRED_CONFIG env var first (for testing)
        let config_path = std::env::var("SCRED_CONFIG")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("SCRED_CONFIG_FILE").map(PathBuf::from))
            .unwrap_or_else(|_| home.join(".scred/proxy.yaml"));

        let content = fs::read_to_string(&config_path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Apply environment variable overrides to config
    fn apply_env_overrides(&mut self) {
        // Proxy config overrides
        if let Ok(listen) = std::env::var("SCRED_LISTEN") {
            self.proxy.listen = listen;
        }
        if let Ok(timeout) = std::env::var("SCRED_UPSTREAM_TIMEOUT") {
            self.proxy.upstream_timeout = timeout;
        }
        if let Ok(max_conn) = std::env::var("SCRED_MAX_CONNECTIONS") {
            if let Ok(num) = max_conn.parse::<usize>() {
                self.proxy.max_connections = num;
            }
        }
        if let Ok(mode_str) = std::env::var("SCRED_REDACTION_MODE") {
            self.proxy.redaction_mode = match mode_str.to_lowercase().as_str() {
                "passthrough" => RedactionMode::Passthrough,
                "detect" | "detect-only" => RedactionMode::DetectOnly,
                "redact" => RedactionMode::Redact,
                _ => RedactionMode::DetectOnly, // Default
            };
        }
        if let Ok(h2_redact) = std::env::var("SCRED_H2_REDACT_HEADERS") {
            self.proxy.h2_redact_headers =
                h2_redact.to_lowercase() != "false" && h2_redact != "0";
        }

        // TLS config overrides
        if let Ok(ca_key) = std::env::var("SCRED_CA_KEY") {
            self.tls.ca_key = PathBuf::from(ca_key);
        }
        if let Ok(ca_cert) = std::env::var("SCRED_CA_CERT") {
            self.tls.ca_cert = PathBuf::from(ca_cert);
        }
        if let Ok(cert_cache) = std::env::var("SCRED_CERT_CACHE_DIR") {
            self.tls.cert_cache_dir = PathBuf::from(cert_cache);
        }

        // Logging config overrides
        if let Ok(level) = std::env::var("SCRED_LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(format) = std::env::var("SCRED_LOG_FORMAT") {
            self.logging.format = format;
        }
        if let Ok(output) = std::env::var("SCRED_LOG_OUTPUT") {
            self.logging.output = output;
        }

        // Secret patterns override
        if let Ok(patterns) = std::env::var("SCRED_PATTERNS") {
            self.secrets.patterns = patterns
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }
    }

    /// Debug: return all active SCRED_ environment variables
    pub fn debug_env_vars() -> std::collections::HashMap<String, String> {
        let mut vars = std::collections::HashMap::new();
        for (key, value) in std::env::vars() {
            if key.starts_with("SCRED_") {
                vars.insert(key, value);
            }
        }
        vars
    }

    /// Resolve wildcard patterns to concrete pattern names
    pub fn resolve_pattern_names(
        config_patterns: Vec<String>,
        all_pattern_names: &[&str],
    ) -> Vec<String> {
        let mut resolved = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for pattern in config_patterns {
            for name in all_pattern_names {
                if matches_pattern(&pattern, name) && !seen.contains(*name) {
                    resolved.push(name.to_string());
                    seen.insert(*name);
                }
            }
        }

        resolved
    }
}

/// Match a pattern (with wildcards) against a name
fn matches_pattern(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if !pattern.contains('*') && !pattern.contains('?') {
        return pattern == name;
    }

    // Simple wildcard matching
    let parts: Vec<&str> = pattern.split('*').collect();

    if parts.len() == 1 {
        return pattern == name;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            // First part must match at the beginning
            if !part.is_empty() && !name.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
            // Last part must match at the end
            if !part.is_empty() && !name[pos..].ends_with(part) {
                return false;
            }
        } else {
            // Middle parts must match in order
            if let Some(idx) = name[pos..].find(part) {
                pos += idx + part.len();
            } else {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern("stripe", "stripe"));
        assert!(!matches_pattern("stripe", "stripe-payment"));
    }

    #[test]
    fn test_matches_pattern_prefix() {
        assert!(matches_pattern("aws*", "aws-access-token"));
        assert!(matches_pattern("aws*", "aws-secret-access-key"));
        assert!(!matches_pattern("aws*", "amazons3"));
    }

    #[test]
    fn test_matches_pattern_suffix() {
        assert!(matches_pattern("*webhook", "discordwebhook"));
        assert!(matches_pattern("*webhook", "slackwebhook"));
        assert!(!matches_pattern("*webhook", "webhook-service"));
    }

    #[test]
    fn test_matches_pattern_infix() {
        assert!(matches_pattern("*webhook*", "discordwebhook"));
        assert!(matches_pattern("*webhook*", "webhook-service"));
        assert!(matches_pattern("*webhook*", "my-webhooks"));
        assert!(!matches_pattern("*webhook*", "discord"));
    }
}
