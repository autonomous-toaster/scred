/// CLI-specific environment variable redaction
/// 
/// Intelligently detects KEY=VALUE format and redacts:
/// - Values for any KEY (secret or not)
/// - Patterns in values are detected by the underlying redactor
///
/// This module acts as a thin wrapper over the core RedactionEngine,
/// ensuring consistent behavior everywhere (env mode = text mode).
///
/// This handles raw environment variable output like:
///   env | scred --env-mode
///   aws_access_key_id=AKIA...
///   SECRET_TOKEN=sk-...

use scred_http::ConfigurableEngine;

const SECRET_KEYWORDS: &[&str] = &[
    "KEY",
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "CREDENTIAL",
    "API",
    "ACCESS",
    "PRIVATE",
    "PASSPHRASE",
    "AWS",
    "AZURE",
    "GCP",
];

/// Check if a variable name contains secret keywords
/// (Now mostly informational - all values are redacted by the engine)
pub fn is_secret_variable(name: &str) -> bool {
    let name_upper = name.to_uppercase();
    SECRET_KEYWORDS.iter().any(|keyword| name_upper.contains(keyword))
}

/// Generic environment line parser
/// 
/// Parses KEY=VALUE format and delegates redaction to provided function.
/// This shared implementation eliminates code duplication while supporting
/// both trait-based and concrete redactors.
fn redact_env_line_generic<F: Fn(&str) -> String>(line: &str, redact_fn: F) -> String {
    if line.is_empty() {
        return String::new();
    }

    // Try to find separator
    let (sep_pos, sep_char) = if let Some(pos) = line.find('=') {
        (Some(pos), '=')
    } else if let Some(pos) = line.find(':') {
        // Check if it's not a URL-like colon (://)
        if pos == 0 || pos == line.len() - 1 || 
           (line.chars().nth(pos - 1) == Some('/') && line.chars().nth(pos + 1) == Some('/')) {
            (None, ':')
        } else {
            (Some(pos), ':')
        }
    } else {
        (None, '=')
    };

    match sep_pos {
        None => {
            // No separator - just scan for patterns
            redact_fn(line)
        }
        Some(sep) => {
            let key = line[..sep].trim();
            let value = line[sep + 1..].trim();


            // Build result
            let mut result = String::new();
            result.push_str(key);
            result.push(sep_char);

            // Always use the redactor
            // The redactor handles:
            // - Prefix preservation (AKIA → AKIAxxx...)
            // - Pattern detection (finds actual secrets)
            // - Consistent behavior with --text-mode
            let redacted_value = redact_fn(value);
            result.push_str(&redacted_value);

            result
        }
    }
}

/// Redact a single environment variable line with generic redaction function
/// 
/// Handles formats like:
///   KEY=VALUE
///   KEY = VALUE (AWS config style)
///   key: value (YAML style)
///
/// # Example
/// ```ignore
/// let result = redact_env_line("API_KEY=sk-abc123...", |v| redactor.redact(v));
/// ```
pub fn redact_env_line(line: &str, redact_fn: impl Fn(&str) -> String) -> String {
    redact_env_line_generic(line, redact_fn)
}

/// Redact an environment variable line using ConfigurableEngine
/// 
/// This is the main entry point for CLI env-mode processing.
/// Ensures all redaction goes through the same engine for consistency.
///
/// # Example
/// ```ignore
/// let result = redact_env_line_configurable("API_KEY=sk-abc123...", &engine);
/// ```
pub fn redact_env_line_configurable(line: &str, config_engine: &ConfigurableEngine) -> String {
    redact_env_line_generic(line, |v| config_engine.redact_only(v))
}

