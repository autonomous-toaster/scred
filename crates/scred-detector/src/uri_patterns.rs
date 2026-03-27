//! URI Pattern Detection & Credential Extraction
//!
//! Handles database connection strings and webhook URLs where we need to:
//! 1. Detect the full URI
//! 2. Extract credentials (username:password or token)
//! 3. Redact ONLY the credentials, preserve the URI structure
//!
//! Examples:
//!   mongodb://user:password@host:27017/db → mongodb://user:[REDACTED]@host:27017/db
//!   https://hooks.slack.com/services/T00/B00/KEY → https://hooks.slack.com/services/T00/B00/[REDACTED]
//!   redis://user:password@host:6379 → redis://user:[REDACTED]@host:6379

use crate::match_result::Match;
use regex::Regex;
use std::sync::OnceLock;

/// URI pattern for credential extraction
#[derive(Debug, Clone)]
pub struct UriPattern {
    pub name: &'static str,
    pub scheme: &'static str,           // e.g., "mongodb://", "https://"
    pub credential_type: CredentialType, // How credentials are structured
    pub min_length: usize,               // Minimum credential length
    pub pattern_id: u16,                // ID for matches (400-420 reserved)
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

/// Detect and extract credentials from database URIs
/// Returns vector of matches with redacted credentials
pub fn detect_database_uris(text: &[u8]) -> Vec<Match> {
    let mut matches = Vec::new();

    for pattern in DATABASE_URI_PATTERNS {
        if let Some(regex) = get_database_uri_regex(pattern) {
            let text_str = match std::str::from_utf8(text) {
                Ok(s) => s,
                Err(_) => continue,
            };

            for m in regex.find_iter(text_str) {
                let uri = m.as_str();
                if uri.len() >= pattern.min_length {
                    // Extract start position in original bytes
                    let start = m.start();
                    let end = m.end();
                    matches.push(Match::new(start, end, pattern.pattern_id));
                }
            }
        }
    }

    matches
}

/// Detect and extract credentials from webhook URLs
pub fn detect_webhook_uris(text: &[u8]) -> Vec<Match> {
    let mut matches = Vec::new();

    for pattern in WEBHOOK_URI_PATTERNS {
        if let Some(regex) = get_webhook_uri_regex(pattern) {
            let text_str = match std::str::from_utf8(text) {
                Ok(s) => s,
                Err(_) => continue,
            };

            for m in regex.find_iter(text_str) {
                let url = m.as_str();
                if url.len() >= pattern.min_length {
                    let start = m.start();
                    let end = m.end();
                    matches.push(Match::new(start, end, pattern.pattern_id));
                }
            }
        }
    }

    matches
}

/// Build regex for database URI pattern
/// Matches: scheme://[user[:password]@]host[:port][/path][?query]
fn get_database_uri_regex(pattern: &UriPattern) -> Option<&'static Regex> {
    match pattern.name {
        "mongodb-uri" => Some(get_or_init_regex("mongodb", r#"mongodb://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{20,500}"#)),
        "mongodb-srv-uri" => Some(get_or_init_regex("mongodb+srv", r#"mongodb\+srv://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{20,500}"#)),
        "redis-uri" => Some(get_or_init_regex("redis", r#"redis://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{15,500}"#)),
        "postgres-uri" => Some(get_or_init_regex("postgres", r#"postgres://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{20,500}"#)),
        "cassandra-uri" => Some(get_or_init_regex("cassandra", r#"cassandra://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{20,500}"#)),
        "elasticsearch-uri" => Some(get_or_init_regex("elasticsearch", r#"elasticsearch://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{20,500}"#)),
        "amqp-uri" => Some(get_or_init_regex("amqp", r#"amqp://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{15,500}"#)),
        "dynamodb-uri" => Some(get_or_init_regex("dynamodb", r#"dynamodb://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{15,500}"#)),
        "couchdb-uri" => Some(get_or_init_regex("couchdb", r#"couchdb://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{20,500}"#)),
        "influxdb-uri" => Some(get_or_init_regex("influxdb", r#"influxdb://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{20,500}"#)),
        "memcached-uri" => Some(get_or_init_regex("memcached", r#"memcached://[a-zA-Z0-9\-._~%!$&'()*+,;=:@/?\#\[\]]{10,500}"#)),
        _ => None,
    }
}

/// Build regex for webhook URI pattern
fn get_webhook_uri_regex(pattern: &UriPattern) -> Option<&'static Regex> {
    match pattern.name {
        "slack-webhook-url" => Some(get_or_init_regex("slack", r#"https://hooks\.slack\.com/services/[A-Z0-9/]{50,200}"#)),
        "discord-webhook-url" => Some(get_or_init_regex("discord", r#"https://discord\.com/api/webhooks/[0-9]+/[a-zA-Z0-9\-_]{50,200}"#)),
        _ => None,
    }
}

/// Cached regex compilations (use OnceLock for thread safety)
static MONGODB_REGEX: OnceLock<Regex> = OnceLock::new();
static MONGODB_SRV_REGEX: OnceLock<Regex> = OnceLock::new();
static REDIS_REGEX: OnceLock<Regex> = OnceLock::new();
static POSTGRES_REGEX: OnceLock<Regex> = OnceLock::new();
static CASSANDRA_REGEX: OnceLock<Regex> = OnceLock::new();
static ELASTICSEARCH_REGEX: OnceLock<Regex> = OnceLock::new();
static AMQP_REGEX: OnceLock<Regex> = OnceLock::new();
static DYNAMODB_REGEX: OnceLock<Regex> = OnceLock::new();
static COUCHDB_REGEX: OnceLock<Regex> = OnceLock::new();
static INFLUXDB_REGEX: OnceLock<Regex> = OnceLock::new();
static MEMCACHED_REGEX: OnceLock<Regex> = OnceLock::new();
static SLACK_REGEX: OnceLock<Regex> = OnceLock::new();
static DISCORD_REGEX: OnceLock<Regex> = OnceLock::new();

/// Helper to get or initialize regex with caching
fn get_or_init_regex(name: &str, pattern: &str) -> &'static Regex {
    match name {
        "mongodb" => MONGODB_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "mongodb+srv" => MONGODB_SRV_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "redis" => REDIS_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "postgres" => POSTGRES_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "cassandra" => CASSANDRA_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "elasticsearch" => ELASTICSEARCH_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "amqp" => AMQP_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "dynamodb" => DYNAMODB_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "couchdb" => COUCHDB_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "influxdb" => INFLUXDB_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "memcached" => MEMCACHED_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "slack" => SLACK_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        "discord" => DISCORD_REGEX.get_or_init(|| Regex::new(pattern).unwrap()),
        _ => panic!("Unknown regex name: {}", name),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_mongodb_uri() {
        let uri = "mongodb://admin:p@ssw0rd@localhost:27017/mydb";
        let redacted = redact_uri_credentials(uri, &CredentialType::UserPasswordAtHost);
        assert_eq!(redacted, "mongodb://admin:[REDACTED]@localhost:27017/mydb");
    }

    #[test]
    fn test_redact_redis_uri() {
        let uri = "redis://user:secretpass@redis.example.com:6379/0";
        let redacted = redact_uri_credentials(uri, &CredentialType::UserPasswordAtHost);
        assert_eq!(redacted, "redis://user:[REDACTED]@redis.example.com:6379/0");
    }

    #[test]
    fn test_redact_slack_webhook() {
        let uri = "https://hooks.slack.com/services/T12345678/B87654321/abcdef1234567890";
        let redacted = redact_uri_credentials(uri, &CredentialType::TokenOnly);
        assert_eq!(
            redacted,
            "https://hooks.slack.com/services/T12345678/B87654321/[REDACTED]"
        );
    }

    #[test]
    fn test_redact_discord_webhook() {
        let uri = "https://discord.com/api/webhooks/123456789/abcdefghijklmnop";
        let redacted = redact_uri_credentials(uri, &CredentialType::TokenOnly);
        assert_eq!(
            redacted,
            "https://discord.com/api/webhooks/123456789/[REDACTED]"
        );
    }

    #[test]
    fn test_postgres_uri_with_complex_password() {
        let uri = "postgres://user:p@ss%40w0rd@db.example.com:5432/database";
        let redacted = redact_uri_credentials(uri, &CredentialType::UserPasswordAtHost);
        assert_eq!(
            redacted,
            "postgres://user:[REDACTED]@db.example.com:5432/database"
        );
    }

    #[test]
    fn test_uri_without_credentials() {
        let uri = "mongodb://localhost:27017/mydb";
        let redacted = redact_uri_credentials(uri, &CredentialType::UserPasswordAtHost);
        // Should return as-is since there are no credentials
        assert_eq!(redacted, "mongodb://localhost:27017/mydb");
    }
}
