use tracing::info;
use scred_mitm::mitm::proxy::ProxyServer;
use scred_mitm::mitm::config::Config;
use scred_http::logging;
use scred_redactor::get_all_patterns;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    logging::init()?;
    
    // Load configuration (merges config file + env vars)
    let config = Config::load()?;
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
    
    // Generate CA if missing
    scred_mitm::mitm::tls::CertificateGenerator::generate_ca_if_missing(
        &config.tls.ca_key,
        &config.tls.ca_cert,
    )?;
    
    // Create proxy server (will use RedactionEngine with all 242 patterns)
    let proxy = ProxyServer::new(&config)?;
    
    // Start listening
    info!("Starting MITM proxy...");
    proxy.run().await?;
    
    Ok(())
}
