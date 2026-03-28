use tracing::info;
use scred_mitm::mitm::proxy::ProxyServer;
use scred_mitm::mitm::config::Config;
use scred_http::logging;
use scred_redactor::get_all_patterns;
use scred_config::ConfigLoader;
use std::env;

/// Load MITM configuration from scred-config file, with fallback to existing Config::load()
fn load_mitm_config_from_file() -> anyhow::Result<Option<Config>> {
    match ConfigLoader::load() {
        Ok(file_config) => {
            if let Some(mitm_cfg) = file_config.scred_mitm {
                info!("[mitm-config] Loaded configuration from file");
                
                // Convert scred-config MitmConfig to scred-mitm Config
                let mut config = Config::default();
                
                // Set listen configuration
                if let Some(port) = mitm_cfg.listen.port {
                    config.proxy.listen = format!("0.0.0.0:{}", port);
                }
                
                // Set CA certificate configuration if provided
                if let Some(ca_path) = &mitm_cfg.ca_cert.path {
                    config.tls.ca_cert = ca_path.clone().into();
                }
                if let Some(key_path) = &mitm_cfg.ca_cert.key_path {
                    config.tls.ca_key = key_path.clone().into();
                }
                
                // Set redaction mode based on mode string
                let mode_str = if mitm_cfg.redaction.mode.is_empty() {
                    "redact"
                } else {
                    mitm_cfg.redaction.mode.as_str()
                };
                
                config.proxy.redaction_mode = match mode_str {
                    "passive" => scred_mitm::mitm::config::RedactionMode::Passthrough,
                    "selective" => scred_mitm::mitm::config::RedactionMode::DetectOnly,
                    "strict" => scred_mitm::mitm::config::RedactionMode::Redact,
                    _ => scred_mitm::mitm::config::RedactionMode::Redact,
                };
                
                return Ok(Some(config));
            }
        }
        Err(e) => {
            info!("[mitm-config] Config file not found ({}). Using environment variables or defaults.", e);
        }
    }
    Ok(None)
}

fn print_help() {
    println!("scred-mitm - MITM proxy with secret redaction and TLS interception");
    println!();
    println!("USAGE:");
    println!("  scred-mitm [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help              Print this help message");
    println!("  --detect TIERS          Pattern tiers to detect (comma-separated)");
    println!("  --redact TIERS          Pattern tiers to redact (comma-separated)");
    println!("  --list-tiers            Show available pattern tiers");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("  SCRED_MITM_LISTEN               Listen address (default: 0.0.0.0:8080)");
    println!("  SCRED_DETECT_PATTERNS           Patterns to detect (comma-separated tiers)");
    println!("  SCRED_REDACT_PATTERNS           Patterns to redact (comma-separated tiers)");
    println!("  SCRED_LOG_LEVEL                 Log level: trace, debug, info, warn, error (default: info)");
    println!("  SCRED_LOG_FORMAT                Log format: text, json, pretty (default: text)");
    println!("  SCRED_LOG_OUTPUT                Log output: stdout, stderr, or file path (default: stderr)");
    println!("  SCRED_MITM_CA_CERT              Path to CA certificate");
    println!("  SCRED_MITM_CA_KEY               Path to CA private key");
    println!();
    println!("EXAMPLES:");
    println!("  scred-mitm");
    println!("  scred-mitm --detect CRITICAL,API_KEYS");
    println!("  scred-mitm --redact CRITICAL --detect CRITICAL,API_KEYS");
    println!("  SCRED_LOG_FORMAT=json scred-mitm");
    println!();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Check for help flag before logging init
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        return Ok(());
    }

    // Initialize logging
    logging::init()?;
    
    // Check for CLI mode arguments
    let detect_mode = args.contains(&"--detect".to_string());
    let redact_mode = args.contains(&"--redact".to_string());
    let list_tiers = args.contains(&"--list-tiers".to_string());
    
    // Show available tiers if requested
    if list_tiers {
        println!("\nAvailable Pattern Tiers:\n");
        println!("CRITICAL       - AWS, GitHub, Stripe, Database credentials (risk: 95)");
        println!("API_KEYS       - OpenAI, Twilio, SendGrid, monitoring services (risk: 80)");
        println!("INFRASTRUCTURE - K8s, Docker, Vault, Grafana tokens (risk: 60)");
        println!("SERVICES       - Specialty services, payment processors (risk: 40)");
        println!("PATTERNS       - JWT, Bearer, BasicAuth generic patterns (risk: 30)");
        println!("\nUsage:");
        println!("  scred-mitm --detect CRITICAL,API_KEYS");
        println!("  scred-mitm --redact CRITICAL");
        println!("  SCRED_DETECT_PATTERNS=CRITICAL,API_KEYS,INFRASTRUCTURE scred-mitm\n");
        return Ok(());
    }
    
    if detect_mode {
        info!("DETECT MODE: Logging all detected secrets (no redaction)");
    }
    if redact_mode {
        info!("REDACT MODE: Actively redacting detected secrets");
    }
    if !detect_mode && !redact_mode {
        info!("DEFAULT MODE: Detect CRITICAL + API_KEYS + INFRASTRUCTURE, redact CRITICAL + API_KEYS");
    }
    
    // Load configuration from scred-config file first, fall back to existing Config::load()
    let mut config = if let Some(file_config) = load_mitm_config_from_file()? {
        file_config
    } else {
        Config::load()?
    };
    
    // Initialize pattern selectors with defaults
    config.proxy.init_patterns();
    
    // Parse CLI flags for pattern selection
    // Format: --detect CRITICAL,API_KEYS --redact CRITICAL
    for i in 0..args.len() {
        if args[i] == "--detect" && i + 1 < args.len() {
            match config.proxy.set_detect_patterns(&args[i + 1]) {
                Ok(_) => info!("OK: Pattern detect selector: {}", args[i + 1]),
                Err(e) => {
                    info!("WARN:  Invalid detect patterns: {}", e);
                    return Err(anyhow::anyhow!("Invalid --detect argument: {}", e));
                }
            }
        }
        if args[i] == "--redact" && i + 1 < args.len() {
            match config.proxy.set_redact_patterns(&args[i + 1]) {
                Ok(_) => info!("OK: Pattern redact selector: {}", args[i + 1]),
                Err(e) => {
                    info!("WARN:  Invalid redact patterns: {}", e);
                    return Err(anyhow::anyhow!("Invalid --redact argument: {}", e));
                }
            }
        }
    }
    
    // Parse environment variables for pattern selection (lower precedence than CLI)
    if env::var("SCRED_DETECT_PATTERNS").is_ok() && !args.iter().any(|a| a == "--detect") {
        let env_detect = env::var("SCRED_DETECT_PATTERNS")?;
        match config.proxy.set_detect_patterns(&env_detect) {
            Ok(_) => info!("OK: ENV: Pattern detect selector from SCRED_DETECT_PATTERNS"),
            Err(e) => info!("WARN:  Invalid SCRED_DETECT_PATTERNS: {}", e),
        }
    }
    
    // Always check SCRED_REDACT_PATTERNS env var (unless CLI --redact-patterns overrides it)
    if env::var("SCRED_REDACT_PATTERNS").is_ok() && !args.iter().any(|a| a == "--redact-patterns") {
        let env_redact = env::var("SCRED_REDACT_PATTERNS")?;
        match config.proxy.set_redact_patterns(&env_redact) {
            Ok(_) => info!("OK: ENV: Pattern redact selector from SCRED_REDACT_PATTERNS"),
            Err(e) => info!("WARN:  Invalid SCRED_REDACT_PATTERNS: {}", e),
        }
    }
    
    // Override redaction mode based on CLI flags (for backward compatibility)
    if detect_mode {
        config.proxy.redaction_mode = scred_mitm::mitm::config::RedactionMode::DetectOnly;
        info!("OK: CLI override: DetectOnly mode");
    } else if redact_mode {
        config.proxy.redaction_mode = scred_mitm::mitm::config::RedactionMode::Redact;
        info!("OK: CLI override: Redact mode");
    }
    
    info!("Loaded config: {:?}", config);
    
    // Debug: Show active SCRED_ environment variables
    let env_vars = Config::debug_env_vars();
    if !env_vars.is_empty() {
        info!("Active SCRED_ environment variables:");
        for (key, value) in env_vars {
            // Mask sensitive values
            let display_value = if key.contains("KEY") || key.contains("CERT") {
                "***REDACTED***".to_string()
            } else {
                value.clone()
            };
            info!("  {} = {}", key, display_value);
        }
    }
    
    // Verify all patterns are available from redactor (for info)
    let all_patterns = get_all_patterns();
    info!("All patterns available: {} patterns loaded from redactor", all_patterns.len());
    
    // Log pattern selector info
    info!("Pattern detection selector: {}", config.proxy.detect_patterns.description());
    info!("Pattern redaction selector: {}", config.proxy.redact_patterns.description());
    
    // Generate CA if missing
    scred_mitm::mitm::tls::CertificateGenerator::generate_ca_if_missing(
        &config.tls.ca_key,
        &config.tls.ca_cert,
    )?;
    
    // Create proxy server (will use RedactionEngine with all patterns)
    let proxy = ProxyServer::new(&config)?;
    
    // Start listening
    info!("Starting MITM proxy...");
    info!("  Listen: {}", config.proxy.listen);
    info!("  Redaction mode: {:?}", config.proxy.redaction_mode);
    info!("  H2 redact headers: {}", config.proxy.h2_redact_headers);
    proxy.run().await?;
    
    Ok(())
}
