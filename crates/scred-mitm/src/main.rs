use tracing::info;
use scred_mitm::mitm::proxy::ProxyServer;
use scred_mitm::mitm::config::Config;
use scred_http::logging;
use scred_redactor::get_all_patterns;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    logging::init()?;
    
    // Check for CLI mode arguments
    let args: Vec<String> = env::args().collect();
    let detect_mode = args.contains(&"--detect".to_string());
    let redact_mode = args.contains(&"--redact".to_string());
    
    if detect_mode {
        info!("🔍 DETECT MODE: Logging all detected secrets (no redaction)");
    }
    if redact_mode {
        info!("🔐 REDACT MODE: Actively redacting detected secrets");
    }
    if !detect_mode && !redact_mode {
        info!("📊 PASSTHROUGH MODE: Forwarding requests, logging only");
    }
    
    // Load configuration (merges config file + env vars)
    let mut config = Config::load()?;
    info!("Loaded config: {:?}", config);
    
    // Override redaction based on CLI flags
    if detect_mode {
        config.proxy.redact_responses = false;  // Log but don't redact
    } else if redact_mode {
        config.proxy.redact_responses = true;   // Actively redact
    }
    
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
    info!("  Redact responses: {}", config.proxy.redact_responses);
    info!("  H2 redact headers: {}", config.proxy.h2_redact_headers);
    proxy.run().await?;
    
    Ok(())
}
