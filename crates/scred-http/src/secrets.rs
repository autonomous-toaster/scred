//! Secret configuration and filtering rules
//!
//! Provides configuration structures for managing which secrets to redact
//! and under what conditions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for secret patterns and redaction rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Pattern names with wildcard support
    /// Examples: "aws*", "github*", "stripe", "*webhook*"
    pub patterns: Vec<String>,
    /// Per-secret redaction rules
    pub rules: HashMap<String, SecretRule>,
}

/// Rules for redacting a specific secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRule {
    /// Hosts where this secret can appear (empty = all)
    pub allowed_hosts: Vec<String>,
    /// Where secret can appear: "header:Authorization", "body:api_key"
    pub allowed_in: Vec<String>,
    /// Action: "ALLOW" | "REDACT" | "BLOCK"
    pub action: String,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            patterns: vec![
                "aws*".to_string(),
                "github*".to_string(),
                "stripe".to_string(),
            ],
            rules: HashMap::new(),
        }
    }
}

impl SecretsConfig {
    /// Check if a secret should be redacted based on rules
    pub fn should_redact(&self, secret_name: &str, _host: Option<&str>) -> bool {
        // If no rules defined, redact everything
        if self.rules.is_empty() {
            return true;
        }

        // Check if this secret has a rule
        match self.rules.get(secret_name) {
            Some(rule) => {
                match rule.action.to_uppercase().as_str() {
                    "BLOCK" => true,
                    "REDACT" => true,
                    "ALLOW" => false,
                    _ => true, // Default to redact if action unknown
                }
            }
            None => true, // If no specific rule, redact
        }
    }

    /// Check if secret is allowed in a location
    pub fn allowed_in_location(&self, secret_name: &str, location: &str) -> bool {
        match self.rules.get(secret_name) {
            Some(rule) => {
                if rule.allowed_in.is_empty() {
                    true // If no restrictions, allow everywhere
                } else {
                    rule.allowed_in.iter().any(|loc| loc == location)
                }
            }
            None => true, // No rules = allow everywhere
        }
    }

    /// Check if secret is allowed on a host
    pub fn allowed_on_host(&self, secret_name: &str, host: &str) -> bool {
        match self.rules.get(secret_name) {
            Some(rule) => {
                if rule.allowed_hosts.is_empty() {
                    true // If no restrictions, allow on all hosts
                } else {
                    rule.allowed_hosts.iter().any(|h| h == host)
                }
            }
            None => true, // No rules = allow on all hosts
        }
    }
}

