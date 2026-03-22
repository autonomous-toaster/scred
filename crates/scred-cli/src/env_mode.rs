/// CLI-specific environment variable redaction
/// 
/// Intelligently detects KEY=VALUE format and redacts:
/// - Values for any KEY containing secret keywords (API_KEY, SECRET, TOKEN, etc.)
/// - Patterns in values (even for non-secret keys)
///
/// This handles raw environment variable output like:
///   env | scred --env-mode
///   aws_access_key_id=AKIA...
///   SECRET_TOKEN=sk-...

use std::collections::HashMap;

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
pub fn is_secret_variable(name: &str) -> bool {
    let name_upper = name.to_uppercase();
    SECRET_KEYWORDS.iter().any(|keyword| name_upper.contains(keyword))
}

/// Redact a single environment variable line
/// Handles formats like:
///   KEY=VALUE
///   KEY = VALUE (AWS config style)
///   key: value (YAML style)
pub fn redact_env_line(line: &str, redact_fn: impl Fn(&str) -> String) -> String {
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

            if is_secret_variable(key) {
                // Redact entire value
                result.push_str(&"x".repeat(value.len()));
            } else {
                // Scan value for patterns
                result.push_str(&redact_fn(value));
            }

            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_variable_detection() {
        assert!(is_secret_variable("API_KEY"));
        assert!(is_secret_variable("AWS_SECRET_ACCESS_KEY"));
        assert!(is_secret_variable("TOKEN"));
        assert!(is_secret_variable("password"));
        assert!(!is_secret_variable("HOSTNAME"));
        assert!(!is_secret_variable("PATH"));
    }
}
