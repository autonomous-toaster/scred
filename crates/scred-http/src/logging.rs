use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging with tracing subscriber
/// Uses SCRED_LOG_LEVEL, SCRED_LOG_FORMAT, and SCRED_LOG_OUTPUT env vars
pub fn init() -> anyhow::Result<()> {
    let log_level = std::env::var("SCRED_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_format = std::env::var("SCRED_LOG_FORMAT").unwrap_or_else(|_| "text".to_string());
    let log_output = std::env::var("SCRED_LOG_OUTPUT").unwrap_or_else(|_| "stderr".to_string());

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    match log_format.as_str() {
        "json" => {
            let fmt_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false);

            match log_output.as_str() {
                "stdout" => {
                    tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stdout))
                        .init();
                }
                "stderr" | _ => {
                    tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stderr))
                        .init();
                }
            }
        }
        "text" | "compact" | _ => {
            let fmt_layer = fmt::layer()
                .compact()
                .with_target(false)
                .with_thread_ids(false);

            match log_output.as_str() {
                "stdout" => {
                    tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stdout))
                        .init();
                }
                "stderr" | _ => {
                    tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stderr))
                        .init();
                }
            }
        }
    }

    Ok(())
}

/// Initialize logging from environment (convenience function)
pub fn init_from_env() {
    if let Err(e) = init() {
        eprintln!("Failed to initialize logging: {}", e);
    }
}
