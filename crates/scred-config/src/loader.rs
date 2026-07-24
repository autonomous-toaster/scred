use crate::*;
use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

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
        let config_path = search_paths.iter().find(|p| p.exists()).cloned();

        let config = if let Some(path) = config_path {
            debug!("Loading config from: {}", path.display());
            let config = Self::load_from_file(&path)?;
            info!("Configuration loaded from: {}", path.display());
            config
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
    pub fn validate(config: &mut FileConfig) -> Result<()> {
        // Validate and normalize traffic mode
        if let Some(mitm_cfg) = &mut config.scred_mitm {
            // If mode is "allow-list", enable traffic filtering
            if let Some(mode) = &mitm_cfg.traffic.mode {
                if mode == "allow-list" {
                    mitm_cfg.traffic.enabled = true;
                }
            }
        }
        
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

        Ok(())
    }

    /// Validate pattern tier names
    fn validate_patterns(patterns: &PatternConfig) -> Result<()> {
        let valid_tiers = [
            "CRITICAL",
            "API_KEYS",
            "INFRASTRUCTURE",
            "SERVICES",
            "PATTERNS",
        ];

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

        let mut config = Self::load_from_file(&path)?;
        Self::validate(&mut config)?;

        println!("✓ Config file is valid: {}", path.display());

        let sections: Vec<&str> = [
            config.scred_cli.is_some().then_some("scred-cli"),
            config.scred_proxy.is_some().then_some("scred-proxy"),
            config.scred_mitm.is_some().then_some("scred-mitm"),
        ]
        .iter()
        .filter_map(|x| *x)
        .collect();

        println!("  Sections: {:?}", sections);
        Ok(())
    }

    /// Find the first existing config file in standard locations
    pub fn find_config_file() -> Result<PathBuf> {
        let env_mode = env::var("SCRED_ENV").unwrap_or_else(|_| "dev".to_string());

        let candidates = vec![
            PathBuf::from("./scred.yaml"),
            PathBuf::from(format!("scred-{}.yaml", env_mode)),
            PathBuf::from(format!(
                "{}/.scred/config.yaml",
                env::var("HOME").unwrap_or_default()
            )),
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
            policy: PolicyConfig::default(),
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


/// Default-deny: block all traffic unless explicitly allowed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TrafficPolicyConfig {
    /// Traffic filtering mode: "allow-list" enables filtering, "disabled" disables it
    /// Setting mode to "allow-list" automatically enables filtering
    #[serde(default)]
    pub mode: Option<String>,
    /// Enable traffic filtering (default: false)
    /// Automatically set to true when mode is "allow-list"
    #[serde(default)]
    pub enabled: bool,

    /// Allowed domains (glob patterns, e.g., ["*.openai.com", "api.github.com"])
    /// Use ["*"] to allow all domains
    #[serde(default = "default_allowed_domains")]
    pub allowed_domains: Vec<String>,

    /// Block message returned for denied requests
    #[serde(default = "default_block_message")]
    pub block_message: String,
}

impl Default for TrafficPolicyConfig {
    fn default() -> Self {
        Self {
            mode: None,
            enabled: false,
            allowed_domains: default_allowed_domains(),
            block_message: default_block_message(),
        }
    }
}

