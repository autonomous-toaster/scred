/// Shared configuration types and utilities
use serde::{Deserialize, Serialize};
use std::env;

/// Pattern selection mode for redaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PatternSelection {
    /// Only redact explicitly listed patterns
    #[serde(rename = "whitelist")]
    Whitelist(Vec<String>),
    
    /// Redact everything except explicitly listed patterns
    #[serde(rename = "blacklist")]
    Blacklist(Vec<String>),
    
    /// Redact all 244+ patterns (strictest)
    #[serde(rename = "all")]
    #[default]
    All,
    
    /// Don't redact anything (for testing)
    #[serde(rename = "none")]
    None,
}

impl PatternSelection {
    /// Check if a pattern should be redacted
    pub fn should_redact(&self, pattern_name: &str) -> bool {
        match self {
            PatternSelection::All => true,
            PatternSelection::None => false,
            PatternSelection::Whitelist(patterns) => {
                patterns.iter().any(|p| p == pattern_name)
            }
            PatternSelection::Blacklist(patterns) => {
                !patterns.iter().any(|p| p == pattern_name)
            }
        }
    }
}

/// Redaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionConfig {
    /// Mode: "all" | "none" | "configured"
    #[serde(default)]
    pub mode: String,
    
    /// Pattern selection (if mode == "configured")
    #[serde(default)]
    pub patterns: PatternSelection,
}

impl RedactionConfig {
    /// Create a new redaction config that redacts everything
    pub fn all() -> Self {
        Self {
            mode: "all".to_string(),
            patterns: PatternSelection::All,
        }
    }
    
    /// Create a new redaction config that redacts nothing
    pub fn none() -> Self {
        Self {
            mode: "none".to_string(),
            patterns: PatternSelection::None,
        }
    }
}


impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            mode: "all".to_string(),
            patterns: PatternSelection::All,
        }
    }
}

/// Parse environment variable as comma-separated list
pub fn parse_env_list(env_var: &str, default: Vec<String>) -> Vec<String> {
    env::var(env_var)
        .ok()
        .and_then(|v| {
            if v.is_empty() {
                None
            } else {
                Some(v.split(',').map(|s| s.trim().to_string()).collect())
            }
        })
        .unwrap_or(default)
}

/// Parse environment variable as string
pub fn parse_env_string(env_var: &str, default: &str) -> String {
    env::var(env_var).unwrap_or_else(|_| default.to_string())
}

/// Parse environment variable as boolean
pub fn parse_env_bool(env_var: &str, default: bool) -> bool {
    env::var(env_var)
        .ok()
        .and_then(|v| match v.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

/// Parse environment variable as integer
pub fn parse_env_int(env_var: &str, default: i32) -> i32 {
    env::var(env_var)
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(default)
}

