//! File-based configuration system for SCRED applications
//!
//! Supports YAML and TOML configuration files with:
//! - Multiple file locations with precedence
//! - Environment variable overrides
//! - Schema validation
//! - Hot-reload support
//! - Policy system (placeholder replacement + redaction)

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

pub mod hot_reload;
pub mod loader;
pub mod policy_config;
pub mod policy_types;

pub use hot_reload::{setup_sighup_handler, HotReloadHandler};
pub use loader::*;
pub use policy_config::*;
pub use policy_types::*;

/// Configuration file with environment variable interpolation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FileConfig {
    /// Policy configuration (placeholder replacement + redaction)
    #[serde(default)]
    pub policy: PolicyConfig,

    /// scred-cli specific configuration
    #[serde(default)]
    pub scred_cli: Option<CliConfig>,

    /// scred-proxy specific configuration
    #[serde(default)]
    pub scred_proxy: Option<ProxyConfig>,

    /// scred-mitm specific configuration
    #[serde(default)]
    pub scred_mitm: Option<MitmConfig>,
}

/// CLI application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CliConfig {
    /// Processing mode: auto | env | text
    #[serde(default = "default_cli_mode")]
    pub mode: String,

    /// Enable streaming mode for large files
    #[serde(default = "default_streaming")]
    pub streaming: bool,

    /// Pattern configuration
    #[serde(default)]
    pub patterns: PatternConfig,
}

/// Proxy application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ProxyConfig {
    /// Listen configuration
    #[serde(default)]
    pub listen: ListenConfig,

    /// Upstream backend configuration
    #[serde(default)]
    pub upstream: UpstreamConfig,

    /// Per-path rules for selective redaction
    #[serde(default)]
    pub rules: Vec<PathRule>,
}

/// MITM proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MitmConfig {
    /// Listen configuration
    #[serde(default)]
    pub listen: ListenConfig,

    /// Upstream proxy for corporate environments
    #[serde(default, rename = "upstream-proxy")]
    pub upstream_proxy: Option<UpstreamProxyConfig>,

    /// CA certificate configuration
    #[serde(default, rename = "ca-cert")]
    pub ca_cert: CaCertConfig,

    /// Traffic filtering policy (default-deny with allowed domains)
    #[serde(default)]
    pub traffic: TrafficPolicyConfig,
}

/// Listen address and port configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListenConfig {
    /// Port to listen on (default: 9999 for proxy, 8080 for MITM)
    #[serde(default)]
    pub port: Option<u16>,

    /// Address to bind to (default: 0.0.0.0)
    #[serde(default)]
    pub address: Option<String>,
}

impl Default for ListenConfig {
    fn default() -> Self {
        Self {
            port: None,
            address: Some("0.0.0.0".to_string()),
        }
    }
}

/// Upstream backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UpstreamConfig {
    /// Backend URL (required for proxy)
    #[serde(default)]
    pub url: Option<String>,

    /// Connection timeout in seconds
    #[serde(default)]
    pub timeout_secs: Option<u64>,

    /// Enable keep-alive connections
    #[serde(default = "default_true")]
    pub keep_alive: bool,
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            url: None,
            timeout_secs: Some(30),
            keep_alive: true,
        }
    }
}

/// Corporate upstream proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UpstreamProxyConfig {
    /// Enable upstream proxy (default: false)
    #[serde(default)]
    pub enabled: bool,

    /// Upstream proxy URL (e.g., http://proxy.corp.com:8080)
    #[serde(default)]
    pub url: Option<String>,

    /// Domains that bypass upstream proxy
    #[serde(default)]
    pub no_proxy: Vec<String>,

    /// Connection pool configuration
    #[serde(default)]
    pub pool: ConnectionPoolConfig,
}

/// Connection pool configuration for upstream proxy
/// Based on industry best practices (nginx, Envoy, Squid)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConnectionPoolConfig {
    /// Maximum connections in pool (default: 100)
    /// Recommended: 2 × CPU cores, or 10-100 depending on throughput
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Idle timeout in seconds before closing unused connections (default: 60)
    /// Recommended: 30-90 seconds for NAT/firewall cleanup
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,

    /// Maximum requests per connection before recycling (default: 1000)
    /// Prevents long-lived pathological connections
    #[serde(default = "default_max_requests")]
    pub max_requests_per_connection: usize,

    /// Wait timeout in seconds when pool is exhausted (default: 30)
    /// Set to 0 for fail-fast behavior
    #[serde(default = "default_wait_timeout")]
    pub wait_timeout_secs: u64,

    /// Enable HTTP/2 multiplexing when upstream supports it (default: true)
    /// When enabled, uses fewer connections (1-4) with multiple streams
    #[serde(default = "default_true")]
    pub enable_h2_multiplexing: bool,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            idle_timeout_secs: default_idle_timeout(),
            max_requests_per_connection: default_max_requests(),
            wait_timeout_secs: default_wait_timeout(),
            enable_h2_multiplexing: true,
        }
    }
}

fn default_max_connections() -> usize { 100 }
fn default_idle_timeout() -> u64 { 60 }
fn default_max_requests() -> usize { 1000 }
fn default_wait_timeout() -> u64 { 30 }

/// Pattern detection and redaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PatternConfig {
    /// Patterns to detect (CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS)
    #[serde(default = "default_detect_patterns")]
    pub detect: Vec<String>,

    /// Patterns to redact (default: CRITICAL, API_KEYS)
    #[serde(default = "default_redact_patterns")]
    pub redact: Vec<String>,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            detect: default_detect_patterns(),
            redact: default_redact_patterns(),
        }
    }
}

/// Per-path redaction rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PathRule {
    /// URL path pattern (supports * wildcard, e.g., /api/internal/*)
    pub path: String,

    /// Whether to redact this path (true/false)
    #[serde(default = "default_true")]
    pub redact: bool,

    /// Optional custom patterns for this path
    #[serde(default)]
    pub patterns: Option<PatternConfig>,

    /// Optional reason/comment for this rule
    #[serde(default)]
    pub reason: Option<String>,
}

/// CA certificate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CaCertConfig {
    /// Generate CA certificate if not found
    #[serde(default = "default_true")]
    pub generate: bool,

    /// Path to CA certificate file
    #[serde(default)]
    pub path: Option<String>,

    /// Certificate cache directory
    #[serde(default = "default_cert_cache_dir")]
    pub cache_dir: Option<String>,

    /// Certificate organization name
    #[serde(default = "default_cert_org")]
    pub organization: String,

    /// Certificate validity in days
    #[serde(default = "default_cert_validity_days")]
    pub validity_days: u32,

    /// Path to CA private key file
    #[serde(default)]
    pub key_path: Option<String>,
}

impl Default for CaCertConfig {
    fn default() -> Self {
        Self {
            generate: true,
            path: default_ca_cert_path(),
            cache_dir: default_cert_cache_dir(),
            organization: default_cert_org(),
            validity_days: default_cert_validity_days(),
            key_path: Some("/tmp/scred-ca-key.pem".to_string()),
        }
    }
}

// Default value functions for serde
fn default_cli_mode() -> String { "auto".to_string() }
fn default_streaming() -> bool { false }
fn default_true() -> bool { true }
fn default_detect_patterns() -> Vec<String> {
    vec![
        "CRITICAL".to_string(),
        "API_KEYS".to_string(),
        "INFRASTRUCTURE".to_string(),
    ]
}
fn default_redact_patterns() -> Vec<String> {
    vec!["CRITICAL".to_string(), "API_KEYS".to_string()]
}

/// Configuration loader with file precedence and environment overrides
pub struct ConfigLoader;

fn default_allowed_domains() -> Vec<String> {
    vec!["*".to_string()]
}
fn default_block_message() -> String {
    "Domain not allowed".to_string()
}


fn default_ca_cert_path() -> Option<String> {
    Some("/tmp/scred-ca.pem".to_string())
}
fn default_cert_cache_dir() -> Option<String> {
    Some("/tmp/scred-certs".to_string())
}
fn default_cert_org() -> String {
    "SCRED MITM Proxy".to_string()
}
fn default_cert_validity_days() -> u32 {
    365
}
