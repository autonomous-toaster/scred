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
        if pos == 0
            || pos == line.len() - 1
            || (line.chars().nth(pos - 1) == Some('/') && line.chars().nth(pos + 1) == Some('/'))
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_env_line_generic_key_value() {
        let result = redact_env_line_generic("API_KEY=sk-abc123", |v| format!("[REDACTED:{}]", v));
        assert_eq!(result, "API_KEY=[REDACTED:sk-abc123]");
    }

    #[test]
    fn test_redact_env_line_generic_no_value() {
        let result = redact_env_line_generic("KEY=", |v| format!("[REDACTED:{}]", v));
        assert_eq!(result, "KEY=[REDACTED:]");
    }

    #[test]
    fn test_redact_env_line_generic_no_equals() {
        let result = redact_env_line_generic("just a comment", |v| format!("[REDACTED:{}]", v));
        assert_eq!(result, "[REDACTED:just a comment]");
    }

    #[test]
    fn test_redact_env_line_generic_export_format() {
        let result = redact_env_line_generic("export SECRET=abc123", |v| format!("[REDACTED:{}]", v));
        assert_eq!(result, "export SECRET=[REDACTED:abc123]");
    }

    #[test]
    fn test_redact_env_line_generic_quoted_value() {
        let result = redact_env_line_generic("KEY=\"quoted value\"", |v| format!("[REDACTED:{}]", v));
        assert_eq!(result, "KEY=[REDACTED:\"quoted value\"]");
    }

    #[test]
    fn test_redact_env_line_generic_empty_line() {
        let result = redact_env_line_generic("", |v| format!("[REDACTED:{}]", v));
        assert_eq!(result, "");
    }
}
