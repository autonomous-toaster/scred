//! File-based configuration system for SCRED applications
//! 
//! Supports YAML and TOML configuration files with:
//! - Multiple file locations with precedence
//! - Environment variable overrides
//! - Schema validation
//! - Hot-reload support

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn, debug};

pub mod hot_reload;
pub use hot_reload::{HotReloadHandler, setup_sighup_handler};

/// Configuration file with environment variable interpolation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FileConfig {
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

    /// Redaction configuration
    #[serde(default)]
    pub redaction: RedactionConfig,

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

    /// Redaction configuration
    #[serde(default)]
    pub redaction: RedactionConfig,

    /// CA certificate configuration
    #[serde(default, rename = "ca-cert")]
    pub ca_cert: CaCertConfig,
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
}

/// Redaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RedactionConfig {
    /// Redaction mode: selective | strict | passive
    #[serde(default)]
    pub mode: String,

    /// Pattern configuration
    #[serde(default)]
    pub patterns: PatternConfig,

    /// Redact request bodies (default: true)
    #[serde(default = "default_true")]
    pub redact_request_body: bool,

    /// Redact request headers (default: true)
    #[serde(default = "default_true")]
    pub redact_request_headers: bool,

    /// Redact response bodies (default: true)
    #[serde(default = "default_true")]
    pub redact_response_body: bool,

    /// Redact response headers (default: true)
    #[serde(default = "default_true")]
    pub redact_response_headers: bool,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            mode: "selective".to_string(),
            patterns: PatternConfig::default(),
            redact_request_body: true,
            redact_request_headers: true,
            redact_response_body: true,
            redact_response_headers: true,
        }
    }
}

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

    /// Path to CA private key file
    #[serde(default)]
    pub key_path: Option<String>,
}

impl Default for CaCertConfig {
    fn default() -> Self {
        Self {
            generate: true,
            path: Some("/tmp/scred-ca.pem".to_string()),
            key_path: Some("/tmp/scred-ca-key.pem".to_string()),
        }
    }
}

// Default value functions for serde
fn default_cli_mode() -> String {
    "auto".to_string()
}

fn default_streaming() -> bool {
    false
}

fn default_true() -> bool {
    true
}

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

impl ConfigLoader {
    /// Load configuration from files with precedence
    /// 
    /// Precedence (highest to lowest):
    /// 1. CLI flags (passed separately)
    /// 2. Environment variables (SCRED_CONFIG_*)
    /// 3. ./scred.yaml (current directory)
    /// 4. ~/.scred/config.yaml (user home)
    /// 5. /etc/scred/config.yaml (system-wide)
    /// 6. Environment-specific files (based on SCRED_ENV)
    pub fn load() -> Result<FileConfig> {
        let env_mode = env::var("SCRED_ENV").unwrap_or_else(|_| "dev".to_string());
        
        // Build search paths
        let mut search_paths = Vec::new();

        // 1. System-wide config
        search_paths.push(PathBuf::from("/etc/scred/config.yaml"));

        // 2. User home config
        if let Ok(home) = env::var("HOME") {
            search_paths.push(PathBuf::from(format!("{}/.scred/config.yaml", home)));
        }

        // 3. Environment-specific config (e.g., config-prod.yaml for production)
        search_paths.push(PathBuf::from(format!("scred-{}.yaml", env_mode)));

        // 4. Current directory config
        search_paths.push(PathBuf::from("scred.yaml"));
        search_paths.push(PathBuf::from("./scred.yaml"));

        // 5. SCRED_CONFIG_FILE environment variable
        if let Ok(config_file) = env::var("SCRED_CONFIG_FILE") {
            search_paths.push(PathBuf::from(config_file));
        }

        // Find first existing config file
        let config_path = search_paths
            .iter()
            .find(|p| p.exists())
            .cloned();

        let config = if let Some(path) = config_path {
            debug!("Loading config from: {}", path.display());
            Self::load_from_file(&path)?
        } else {
            info!("No config file found in standard locations, using defaults");
            FileConfig::default()
        };

        // Apply environment variable overrides
        let config = Self::apply_env_overrides(config)?;

        Ok(config)
    }

    /// Load configuration from a specific file
    pub fn load_from_file(path: &Path) -> Result<FileConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read config file {}: {}", path.display(), e))?;

        let config = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse TOML config: {}", e))?
        } else {
            serde_yaml::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse YAML config: {}", e))?
        };

        Ok(config)
    }

    /// Apply environment variable overrides to configuration
    fn apply_env_overrides(mut config: FileConfig) -> Result<FileConfig> {
        // Example env var patterns:
        // SCRED_PROXY_LISTEN_PORT=9999
        // SCRED_PROXY_UPSTREAM_URL=https://backend.example.com
        // SCRED_CLI_STREAMING=true
        // SCRED_PROXY_REDACTION_MODE=strict

        // Proxy overrides
        if let Ok(port) = env::var("SCRED_PROXY_LISTEN_PORT") {
            if let Some(proxy_cfg) = &mut config.scred_proxy {
                proxy_cfg.listen.port = Some(port.parse()?);
            }
        }

        if let Ok(url) = env::var("SCRED_PROXY_UPSTREAM_URL") {
            if let Some(proxy_cfg) = &mut config.scred_proxy {
                proxy_cfg.upstream.url = Some(url);
            }
        }

        if let Ok(mode) = env::var("SCRED_PROXY_REDACTION_MODE") {
            if let Some(proxy_cfg) = &mut config.scred_proxy {
                proxy_cfg.redaction.mode = mode;
            }
        }

        // CLI overrides
        if let Ok(streaming) = env::var("SCRED_CLI_STREAMING") {
            if let Some(cli_cfg) = &mut config.scred_cli {
                cli_cfg.streaming = streaming.to_lowercase() == "true";
            }
        }

        // MITM overrides
        if let Ok(port) = env::var("SCRED_MITM_LISTEN_PORT") {
            if let Some(mitm_cfg) = &mut config.scred_mitm {
                mitm_cfg.listen.port = Some(port.parse()?);
            }
        }

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(config: &FileConfig) -> Result<()> {
        // Validate proxy config
        if let Some(proxy_cfg) = &config.scred_proxy {
            if proxy_cfg.upstream.url.is_none() {
                return Err(anyhow!(
                    "Proxy configuration missing required upstream URL. \
                     Set via scred_proxy.upstream.url in config file or \
                     SCRED_PROXY_UPSTREAM_URL environment variable"
                ));
            }

            // Validate upstream URL format
            if let Some(url) = &proxy_cfg.upstream.url {
                url.parse::<http::Uri>()
                    .map_err(|e| anyhow!("Invalid upstream URL '{}': {}", url, e))?;
            }

            // Validate path rules
            for rule in &proxy_cfg.rules {
                if rule.path.is_empty() {
                    return Err(anyhow!("Path rule has empty path"));
                }
            }
        }

        // Validate patterns
        if let Some(cli_cfg) = &config.scred_cli {
            Self::validate_patterns(&cli_cfg.patterns)?;
        }
        if let Some(proxy_cfg) = &config.scred_proxy {
            Self::validate_patterns(&proxy_cfg.redaction.patterns)?;
        }
        if let Some(mitm_cfg) = &config.scred_mitm {
            Self::validate_patterns(&mitm_cfg.redaction.patterns)?;
        }

        Ok(())
    }

    /// Validate pattern tier names
    fn validate_patterns(patterns: &PatternConfig) -> Result<()> {
        let valid_tiers = ["CRITICAL", "API_KEYS", "INFRASTRUCTURE", "SERVICES", "PATTERNS"];

        for tier in &patterns.detect {
            if !valid_tiers.contains(&tier.as_str()) {
                warn!("Unknown pattern tier in detect config: {}", tier);
            }
        }

        for tier in &patterns.redact {
            if !valid_tiers.contains(&tier.as_str()) {
                warn!("Unknown pattern tier in redact config: {}", tier);
            }
        }

        Ok(())
    }

    /// Check if configuration file exists and is valid
    pub fn check_config_file(path: Option<&str>) -> Result<()> {
        let path = if let Some(p) = path {
            PathBuf::from(p)
        } else {
            Self::find_config_file()?
        };

        if !path.exists() {
            return Err(anyhow!("Config file not found: {}", path.display()));
        }

        let config = Self::load_from_file(&path)?;
        Self::validate(&config)?;

        println!("✓ Config file is valid: {}", path.display());
        let sections: Vec<&str> = [
            config.scred_cli.is_some().then_some("scred-cli"),
            config.scred_proxy.is_some().then_some("scred-proxy"),
            config.scred_mitm.is_some().then_some("scred-mitm"),
        ].iter().filter_map(|x| *x).collect();
        println!("  Sections: {:?}", sections);

        Ok(())
    }

    /// Find the first existing config file in standard locations
    pub fn find_config_file() -> Result<PathBuf> {
        let env_mode = env::var("SCRED_ENV").unwrap_or_else(|_| "dev".to_string());
        
        let candidates = vec![
            PathBuf::from("./scred.yaml"),
            PathBuf::from(format!("scred-{}.yaml", env_mode)),
            PathBuf::from(format!("{}/.scred/config.yaml", env::var("HOME").unwrap_or_default())),
            PathBuf::from("/etc/scred/config.yaml"),
        ];

        candidates
            .into_iter()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow!("No config file found in standard locations"))
    }
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            scred_cli: Some(CliConfig {
                mode: default_cli_mode(),
                streaming: default_streaming(),
                patterns: PatternConfig::default(),
            }),
            scred_proxy: None,
            scred_mitm: None,
        }
    }
}


