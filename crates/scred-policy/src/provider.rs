//! Secret providers - trait and implementations for secret sources

use std::collections::HashMap;
use std::env;

use crate::validation::{validate_secret, OnInvalid};
use crate::{PolicyError, PolicyResult};

/// Trait for secret providers
pub trait SecretProvider: Send + Sync {
    /// Load secrets and return a map of name -> value
    fn load(&self) -> PolicyResult<HashMap<String, String>>;

    /// Get the provider name for logging
    fn name(&self) -> &str;
}

/// Environment variable provider
///
/// Supports:
/// - Explicit variable names
/// - Prefix matching (e.g., "APP_")
/// - Glob patterns (e.g., "*_API_KEY")
pub struct EnvProvider {
    /// Explicit variable names to load
    vars: Vec<String>,
    /// Prefix to match (e.g., "APP_")
    prefix: Option<String>,
    /// Glob patterns to match (e.g., "*_API_KEY")
    globs: Vec<String>,
    /// Behavior when env var doesn't exist
    on_missing: OnMissing,
    /// Behavior when secret contains dangerous chars
    on_invalid: OnInvalid,
}

/// Behavior when referenced env var doesn't exist
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnMissing {
    /// Fail startup with error
    #[default]
    Fail,
    /// Log warning, continue without the secret
    Warn,
    /// Silently ignore
    Ignore,
}

impl EnvProvider {
    /// Create a new EnvProvider with default settings
    pub fn new() -> Self {
        Self {
            vars: Vec::new(),
            prefix: None,
            globs: Vec::new(),
            on_missing: OnMissing::Warn,
            on_invalid: OnInvalid::Fail,
        }
    }

    /// Add explicit variable names
    pub fn with_vars(mut self, vars: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.vars.extend(vars.into_iter().map(|v| v.into()));
        self
    }

    /// Set prefix to match
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Add glob patterns to match
    pub fn with_globs(mut self, globs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.globs.extend(globs.into_iter().map(|g| g.into()));
        self
    }

    /// Set behavior for missing env vars
    pub fn with_on_missing(mut self, on_missing: OnMissing) -> Self {
        self.on_missing = on_missing;
        self
    }

    /// Set behavior for invalid secrets
    pub fn with_on_invalid(mut self, on_invalid: OnInvalid) -> Self {
        self.on_invalid = on_invalid;
        self
    }

    /// Check if a variable name matches any configured pattern
    fn matches(&self, name: &str) -> bool {
        // Check explicit vars
        if self.vars.contains(&name.to_string()) {
            return true;
        }

        // Check prefix
        if let Some(ref prefix) = self.prefix {
            if name.starts_with(prefix) {
                return true;
            }
        }

        // Check globs using simple wildcard matching
        for glob in &self.globs {
            if self.glob_matches(glob, name) {
                return true;
            }
        }

        false
    }

    /// Simple glob matching: * matches any chars, ? matches single char
    fn glob_matches(&self, pattern: &str, name: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if !pattern.contains('*') && !pattern.contains('?') {
            return pattern == name;
        }

        // Use wax for proper glob matching if available, otherwise simple impl
        // For now, use simple matching
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 1 {
            return pattern == name;
        }

        // First part must match start
        if !parts[0].is_empty() && !name.starts_with(parts[0]) {
            return false;
        }

        // Last part must match end
        if !parts.last().unwrap().is_empty() && !name.ends_with(parts.last().unwrap()) {
            return false;
        }

        // Middle parts must be found in order
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if i == 0 {
                pos = part.len();
            } else if i == parts.len() - 1 {
                // Already checked end
            } else if let Some(idx) = name[pos..].find(*part) {
                pos += idx + part.len();
            } else {
                return false;
            }
        }

        true
    }

    /// Get all matching variable names from environment
    fn get_matching_vars(&self) -> Vec<String> {
        let mut matches = Vec::new();

        for (key, _) in env::vars() {
            if self.matches(&key) {
                matches.push(key);
            }
        }

        // Sort for deterministic ordering
        matches.sort();
        matches
    }
}

impl Default for EnvProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretProvider for EnvProvider {
    fn load(&self) -> PolicyResult<HashMap<String, String>> {
        let mut secrets = HashMap::new();
        let matching_vars = self.get_matching_vars();

        for var_name in matching_vars {
            match env::var(&var_name) {
                Ok(value) => {
                    // Validate secret
                    match validate_secret(&value, self.on_invalid) {
                        Ok(validated) => {
                            secrets.insert(var_name, validated.into_owned());
                        }
                        Err(e) => {
                            match self.on_invalid {
                                OnInvalid::Fail => {
                                    return Err(PolicyError::Validation(e));
                                }
                                OnInvalid::Warn => {
                                    tracing::warn!(
                                        "Secret {} contains dangerous characters, skipping: {}",
                                        var_name,
                                        e
                                    );
                                    // Don't add, but continue
                                }
                                OnInvalid::Sanitize => {
                                    // Sanitize already handled in validate_secret
                                    unreachable!()
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    match self.on_missing {
                        OnMissing::Fail => {
                            return Err(PolicyError::NotFound(format!(
                                "Environment variable {} not found",
                                var_name
                            )));
                        }
                        OnMissing::Warn => {
                            tracing::warn!("Environment variable {} not found, skipping", var_name);
                        }
                        OnMissing::Ignore => {
                            // Silently skip
                        }
                    }
                }
            }
        }

        Ok(secrets)
    }

    fn name(&self) -> &str {
        "env"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_explicit_vars() {
        env::set_var("TEST_EXPLICIT_KEY", "test-value-123");

        let provider = EnvProvider::new()
            .with_vars(["TEST_EXPLICIT_KEY"])
            .with_on_missing(OnMissing::Ignore);

        let secrets = provider.load().unwrap();
        assert_eq!(
            secrets.get("TEST_EXPLICIT_KEY"),
            Some(&"test-value-123".to_string())
        );

        env::remove_var("TEST_EXPLICIT_KEY");
    }

    #[test]
    fn test_prefix_matching() {
        env::set_var("APP_SECRET_1", "value1");
        env::set_var("APP_SECRET_2", "value2");
        env::set_var("OTHER_SECRET", "value3");

        let provider = EnvProvider::new()
            .with_prefix("APP_")
            .with_on_missing(OnMissing::Ignore);

        let secrets = provider.load().unwrap();
        assert_eq!(secrets.len(), 2);
        assert!(secrets.contains_key("APP_SECRET_1"));
        assert!(secrets.contains_key("APP_SECRET_2"));
        assert!(!secrets.contains_key("OTHER_SECRET"));

        env::remove_var("APP_SECRET_1");
        env::remove_var("APP_SECRET_2");
        env::remove_var("OTHER_SECRET");
    }

    #[test]
    fn test_glob_matching() {
        env::set_var("SCRED_GLOB_API_KEY1", "sk-test1");
        env::set_var("SCRED_GLOB_API_KEY2", "sk-test2");
        env::set_var("SCRED_GLOB_OTHER", "value");

        let provider = EnvProvider::new()
            .with_globs(["SCRED_GLOB_*_KEY*"])
            .with_on_missing(OnMissing::Ignore);

        let secrets = provider.load().unwrap();
        assert_eq!(secrets.len(), 2);
        assert!(secrets.contains_key("SCRED_GLOB_API_KEY1"));
        assert!(secrets.contains_key("SCRED_GLOB_API_KEY2"));
        assert!(!secrets.contains_key("SCRED_GLOB_OTHER"));

        env::remove_var("SCRED_GLOB_API_KEY1");
        env::remove_var("SCRED_GLOB_API_KEY2");
        env::remove_var("SCRED_GLOB_OTHER");
    }

    #[test]
    fn test_glob_wildcard() {
        let provider = EnvProvider::new();

        // Test glob matching patterns
        assert!(provider.glob_matches("*", "anything"));
        assert!(provider.glob_matches("OPENAI_*", "OPENAI_API_KEY"));
        assert!(provider.glob_matches("*_KEY", "OPENAI_KEY"));

        // Negative cases
        assert!(!provider.glob_matches("OPENAI_*", "MISTRAL_KEY"));
        assert!(!provider.glob_matches("*_KEY", "API_VALUE"));
    }

    #[test]
    fn test_validation_in_provider() {
        env::set_var("TEST_BAD_SECRET", "value\r\nX-Admin: true");

        let provider = EnvProvider::new()
            .with_vars(["TEST_BAD_SECRET"])
            .with_on_invalid(OnInvalid::Fail);

        let result = provider.load();
        assert!(result.is_err());

        env::remove_var("TEST_BAD_SECRET");
    }

    #[test]
    fn test_warn_on_invalid() {
        env::set_var("TEST_WARN_SECRET", "value\r\nX-Admin: true");
        env::set_var("TEST_GOOD_SECRET", "good-value");

        let provider = EnvProvider::new()
            .with_vars(["TEST_WARN_SECRET", "TEST_GOOD_SECRET"])
            .with_on_invalid(OnInvalid::Warn)
            .with_on_missing(OnMissing::Ignore);

        let secrets = provider.load().unwrap();
        assert!(!secrets.contains_key("TEST_WARN_SECRET"));
        assert!(secrets.contains_key("TEST_GOOD_SECRET"));

        env::remove_var("TEST_WARN_SECRET");
        env::remove_var("TEST_GOOD_SECRET");
    }
}
