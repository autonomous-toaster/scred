//! URI Pattern Detection & Credential Extraction
//!
//! Handles database connection strings and webhook URLs where we need to:
//! 1. Detect the full URI using Aho-Corasick (scheme matching)
//! 2. Extract credentials (username:password or token)
//! 3. Redact ONLY the credentials, preserve the URI structure
//!
//! Uses Aho-Corasick for O(n) scheme detection (faster than regex)
//!
//! Examples:
//!   mongodb://user:password@host:27017/db → mongodb://user:[REDACTED]@host:27017/db
//!   https://hooks.slack.com/services/T00/B00/KEY → https://hooks.slack.com/services/T00/B00/[REDACTED]
//!   redis://user:password@host:6379 → redis://user:[REDACTED]@host:6379

use crate::match_result::Match;
use aho_corasick::AhoCorasick;
use std::sync::OnceLock;

/// URI pattern for credential extraction
#[derive(Debug, Clone)]
pub struct UriPattern {
    pub name: &'static str,
    pub scheme: &'static str,            // e.g., "mongodb://", "https://"
    pub credential_type: CredentialType, // How credentials are structured
    pub min_length: usize,               // Minimum credential length
    pub pattern_id: u16,                 // ID for matches (400-420 reserved)
}

#[derive(Debug, Clone, PartialEq)]
pub enum CredentialType {
    /// username:password format (extract password after ':')
    UserPassword,
    /// Just a token/API key (extract everything)
    TokenOnly,
    /// user:password@host format (extract password between ':' and '@')
    UserPasswordAtHost,
}

/// Database connection URI patterns (11 patterns)
pub const DATABASE_URI_PATTERNS: &[UriPattern] = &[
    UriPattern {
        name: "mongodb-uri",
        scheme: "mongodb://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 20,
        pattern_id: 400,
    },
    UriPattern {
        name: "mongodb-srv-uri",
        scheme: "mongodb+srv://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 20,
        pattern_id: 401,
    },
    UriPattern {
        name: "redis-uri",
        scheme: "redis://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 15,
        pattern_id: 402,
    },
    UriPattern {
        name: "postgres-uri",
        scheme: "postgres://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 20,
        pattern_id: 403,
    },
    UriPattern {
        name: "cassandra-uri",
        scheme: "cassandra://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 20,
        pattern_id: 404,
    },
    UriPattern {
        name: "elasticsearch-uri",
        scheme: "elasticsearch://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 20,
        pattern_id: 405,
    },
    UriPattern {
        name: "amqp-uri",
        scheme: "amqp://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 15,
        pattern_id: 406,
    },
    UriPattern {
        name: "dynamodb-uri",
        scheme: "dynamodb://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 15,
        pattern_id: 407,
    },
    UriPattern {
        name: "couchdb-uri",
        scheme: "couchdb://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 20,
        pattern_id: 408,
    },
    UriPattern {
        name: "influxdb-uri",
        scheme: "influxdb://",
        credential_type: CredentialType::UserPasswordAtHost,
        min_length: 20,
        pattern_id: 409,
    },
    UriPattern {
        name: "memcached-uri",
        scheme: "memcached://",
        credential_type: CredentialType::TokenOnly,
        min_length: 10,
        pattern_id: 410,
    },
];

/// Webhook URL patterns (3 patterns)
pub const WEBHOOK_URI_PATTERNS: &[UriPattern] = &[
    UriPattern {
        name: "slack-webhook-url",
        scheme: "https://hooks.slack.com/services/",
        credential_type: CredentialType::TokenOnly,
        min_length: 50,
        pattern_id: 411,
    },
    UriPattern {
        name: "discord-webhook-url",
        scheme: "https://discord.com/api/webhooks/",
        credential_type: CredentialType::TokenOnly,
        min_length: 50,
        pattern_id: 412,
    },
];

/// Aho-Corasick automaton for database URI schemes
static DATABASE_AC: OnceLock<AhoCorasick> = OnceLock::new();
/// Aho-Corasick automaton for webhook URL schemes
static WEBHOOK_AC: OnceLock<AhoCorasick> = OnceLock::new();

/// Get or initialize database URI automaton
fn get_database_ac() -> &'static AhoCorasick {
    DATABASE_AC.get_or_init(|| {
        let schemes: Vec<&str> = DATABASE_URI_PATTERNS.iter().map(|p| p.scheme).collect();
        AhoCorasick::new(schemes).unwrap()
    })
}

/// Get or initialize webhook URI automaton
fn get_webhook_ac() -> &'static AhoCorasick {
    WEBHOOK_AC.get_or_init(|| {
        let schemes: Vec<&str> = WEBHOOK_URI_PATTERNS.iter().map(|p| p.scheme).collect();
        AhoCorasick::new(schemes).unwrap()
    })
}

/// Detect and extract credentials from database URIs
/// Returns vector of matches with redacted credentials
/// Uses Aho-Corasick for O(n) scheme detection
pub fn detect_database_uris(text: &[u8]) -> Vec<Match> {
    let mut matches = Vec::new();
    let ac = get_database_ac();
    let text_str = match std::str::from_utf8(text) {
        Ok(s) => s,
        Err(_) => return matches,
    };

    // Find all scheme matches using Aho-Corasick
    for mat in ac.find_iter(text_str) {
        let scheme_idx = mat.pattern();
        let pattern = &DATABASE_URI_PATTERNS[scheme_idx];

        // Extract URI from this position to end of line or next whitespace
        let uri_start = mat.start();
        let mut uri_end = uri_start + pattern.scheme.len();

        // Extend URI boundary until whitespace or newline
        while uri_end < text_str.len() {
            let ch = text_str.as_bytes()[uri_end] as char;
            if ch.is_whitespace() {
                break;
            }
            uri_end += 1;
        }

        let uri = &text_str[uri_start..uri_end];
        if uri.len() >= pattern.min_length {
            matches.push(Match::new(uri_start, uri_end, pattern.pattern_id));
        }
    }

    matches
}

/// Detect and extract credentials from webhook URLs
/// Uses Aho-Corasick for O(n) scheme detection
pub fn detect_webhook_uris(text: &[u8]) -> Vec<Match> {
    let mut matches = Vec::new();
    let ac = get_webhook_ac();
    let text_str = match std::str::from_utf8(text) {
        Ok(s) => s,
        Err(_) => return matches,
    };

    // Find all scheme matches using Aho-Corasick
    for mat in ac.find_iter(text_str) {
        let scheme_idx = mat.pattern();
        let pattern = &WEBHOOK_URI_PATTERNS[scheme_idx];

        // Extract URL from this position to end of line or next whitespace
        let url_start = mat.start();
        let mut url_end = url_start + pattern.scheme.len();

        // Extend URL boundary until whitespace or newline
        while url_end < text_str.len() {
            let ch = text_str.as_bytes()[url_end] as char;
            if ch.is_whitespace() {
                break;
            }
            url_end += 1;
        }

        let url = &text_str[url_start..url_end];
        if url.len() >= pattern.min_length {
            matches.push(Match::new(url_start, url_end, pattern.pattern_id));
        }
    }

    matches
}

/// Redact credentials from a URI while preserving structure
///
/// Examples:
///   mongodb://user:password@host:27017/db → mongodb://user:[REDACTED]@host:27017/db
///   https://hooks.slack.com/services/T00/B00/KEY → https://hooks.slack.com/services/T00/B00/[REDACTED]
pub fn redact_uri_credentials(uri: &str, credential_type: &CredentialType) -> String {
    match credential_type {
        CredentialType::UserPasswordAtHost => {
            // mongodb://user:password@host → mongodb://user:[REDACTED]@host
            if let Some(at_pos) = uri.rfind('@') {
                if let Some(colon_pos) = uri[..at_pos].rfind(':') {
                    // Found user:password pattern
                    let before = &uri[..colon_pos + 1];
                    let after = &uri[at_pos..];
                    return format!("{}[REDACTED]{}", before, after);
                }
            }
            // No credentials found, return as-is
            uri.to_string()
        }
        CredentialType::TokenOnly => {
            // Extract last path segment or token
            if let Some(last_slash) = uri.rfind('/') {
                let before = &uri[..last_slash + 1];
                return format!("{}[REDACTED]", before);
            }
            // Fallback: redact everything after scheme
            if let Some(scheme_end) = uri.find("://") {
                let scheme = &uri[..scheme_end + 3];
                return format!("{}[REDACTED]", scheme);
            }
            "[REDACTED]".to_string()
        }
        CredentialType::UserPassword => {
            // Extract password after ':'
            if let Some(colon_pos) = uri.find(':') {
                let before = &uri[..colon_pos + 1];
                if let Some(space_pos) = uri[colon_pos..].find(' ') {
                    let after = &uri[colon_pos + space_pos..];
                    return format!("{}[REDACTED]{}", before, after);
                }
            }
            uri.to_string()
        }
    }
}
