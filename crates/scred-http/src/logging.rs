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
            // JSON format
            let fmt_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true);

            match log_output.as_str() {
                "stdout" => {
                    let registry = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stdout));
                    registry.init();
                }
                "stderr" => {
                    let registry = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stderr));
                    registry.init();
                }
                path => {
                    // Try to use as file path
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)?;
                    let registry = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(file));
                    registry.init();
                }
            }
        }
        "text" | "compact" => {
            // Text/Compact format (single line, human-readable)
            let fmt_layer = fmt::layer()
                .compact()
                .with_target(false)
                .with_thread_ids(false);

            match log_output.as_str() {
                "stdout" => {
                    let registry = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stdout));
                    registry.init();
                }
                "stderr" => {
                    let registry = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stderr));
                    registry.init();
                }
                path => {
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)?;
                    let registry = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(file));
                    registry.init();
                }
            }
        }
        "pretty" => {
            // Pretty format (human-readable, multi-line)
            let fmt_layer = fmt::layer()
                .pretty()
                .with_target(true)
                .with_thread_ids(true);

            match log_output.as_str() {
                "stdout" => {
                    let registry = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stdout));
                    registry.init();
                }
                "stderr" => {
                    let registry = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(std::io::stderr));
                    registry.init();
                }
                path => {
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)?;
                    let registry = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer.with_writer(file));
                    registry.init();
                }
            }
        }
        _ => {
            // Default to text (compact)
            let fmt_layer = fmt::layer()
                .compact()
                .with_target(false)
                .with_thread_ids(false);

            let registry = tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer.with_writer(std::io::stderr));
            registry.init();
        }
    }

    Ok(())
}
