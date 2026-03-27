#![allow(dead_code)]
#![cfg_attr(feature = "simd-accel", feature(portable_simd))]

//! SCRED Pattern Detector - Pure Rust SIMD Implementation
//! 
//! Replaces broken Zig FFI with fast Rust pattern detection.
//! All 275 patterns from Zig converted to Rust with identical logic.

pub mod patterns;
pub mod match_result;
pub mod simd_core;
pub mod simd_charset;
pub mod simd_pattern_matching;
pub mod vectorized_pattern_matching;
pub mod pattern_trie;
pub mod simd_memchr;
pub mod simd_validation;
pub mod simd_multi_search;
pub mod detector;
pub mod regex_patterns;
pub mod uri_patterns;

pub use match_result::{Match, DetectionResult, RedactionResult};
pub use patterns::{
    SimplePrefixPattern, PrefixValidationPattern, JwtPattern,
    SIMPLE_PREFIX_PATTERNS, PREFIX_VALIDATION_PATTERNS, JWT_PATTERNS,
    PatternTier, Charset,
};
pub use detector::{detect_simple_prefix, detect_validation, detect_jwt, detect_all, redact_text};

// Version matching Zig implementation
pub const VERSION: &str = "0.1.0";
pub const TOTAL_PATTERNS: usize = patterns::TOTAL_PATTERNS;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_loads() {
        // 23 SIMPLE_PREFIX + 349 PREFIX_VALIDATION + 1 JWT + 11 MULTILINE_MARKER + 18 REGEX + 14 URI = 416
        // Phase B: Added 129 API key patterns from batches 2-7 (220 → 349 PREFIX_VALIDATION)
        // Phase B: Added 14 URI patterns (11 database + 3 webhook) with credential extraction
        // Expected: 23 + 349 + 1 + 11 + 18 + 14 = 416 patterns
        assert_eq!(TOTAL_PATTERNS, 416);
    }

    #[test]
    fn test_aws_detection() {
        let result = detect_simple_prefix(b"Authorization: AKIAIOSFODNN7EXAMPLE");
        assert!(result.count() > 0, "Should detect AWS key");
    }
}

#[cfg(test)]
mod env_pattern_tests {
    use super::*;

    #[test]
    fn test_env_client_secret_suffix() {
        let text = b"SERVICE_CLIENT_SECRET=abcdef123456789012345678901234";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect _CLIENT_SECRET= suffix");
    }

    #[test]
    fn test_env_api_key_suffix() {
        let text = b"STRIPE_API_KEY=sk_test_abcd1234567890ef";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect _API_KEY= suffix");
    }

    #[test]
    fn test_env_token_suffix() {
        let text = b"AUTH_TOKEN=eyJhbGciOiJIUzI1NiJ9abcd1234567890";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect _TOKEN= suffix");
    }

    #[test]
    fn test_env_secret_suffix() {
        let text = b"STRIPE_SECRET=sh_test_abcd1234567890ef";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect _SECRET= suffix");
    }

    #[test]
    fn test_env_password_suffix() {
        let text = b"DB_PASSWORD=mySecurePassword123456";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect _PASSWORD= suffix");
    }

    #[test]
    #[test]
    fn test_passphrase_env() {
        let text = b"PASSPHRASE=secretPassphrase123456";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect PASSPHRASE= pattern");
    }

    #[test]
    fn test_mysql_pwd_env() {
        let text = b"MYSQL_PWD=rootPassword123456";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect MYSQL_PWD= pattern");
    }

    #[test]
    fn test_rabbitmq_default_pass_env() {
        let text = b"RABBITMQ_DEFAULT_PASS=guest_password_123";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect RABBITMQ_DEFAULT_PASS= pattern");
    }

    #[test]
    fn test_redis_password_env() {
        let text = b"REDIS_PASSWORD=foobared1234567890";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect REDIS_PASSWORD= pattern");
    }

    #[test]
    fn test_postgres_password_env() {
        let text = b"POSTGRES_PASSWORD=superSecurePassword123";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect POSTGRES_PASSWORD= pattern");
    }

    #[test]
    fn test_docker_registry_password_env() {
        let text = b"DOCKER_REGISTRY_PASSWORD=myDockerPassword123";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect DOCKER_REGISTRY_PASSWORD= pattern");
    }

    #[test]
    fn test_vault_token_env() {
        let text = b"VAULT_TOKEN=hvs.CAESIAbcDefGHijKlmnOpqrstuvwxyz123456";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect VAULT_TOKEN= pattern");
    }

    #[test]
    fn test_ldap_password_env() {
        let text = b"LDAP_PASSWORD=ldap_admin_password_2024";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect LDAP_PASSWORD= pattern");
    }

    #[test]
    fn test_ldap_bind_password_env() {
        let text = b"LDAP_BIND_PASSWORD=bind_account_secret_pass";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect LDAP_BIND_PASSWORD= pattern");
    }

    #[test]
    fn test_cassandra_password_env() {
        let text = b"CASSANDRA_PASSWORD=cassandra_node_password";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect CASSANDRA_PASSWORD= pattern");
    }

    #[test]
    fn test_elasticsearch_password_env() {
        let text = b"ELASTICSEARCH_PASSWORD=elastic_search_pwd_123";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect ELASTICSEARCH_PASSWORD= pattern");
    }

    #[test]
    fn test_couchdb_password_env() {
        let text = b"COUCHDB_PASSWORD=couchdb_admin_secret";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect COUCHDB_PASSWORD= pattern");
    }

    #[test]
    fn test_kafka_sasl_password_env() {
        let text = b"KAFKA_SASL_PASSWORD=kafka_broker_password_24";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect KAFKA_SASL_PASSWORD= pattern");
    }

    #[test]
    fn test_activemq_password_env() {
        let text = b"ACTIVEMQ_PASSWORD=activemq_broker_secret_pwd";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect ACTIVEMQ_PASSWORD= pattern");
    }

    #[test]
    fn test_bitbucket_password_env() {
        let text = b"BITBUCKET_PASSWORD=bitbucket_ci_password_123";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect BITBUCKET_PASSWORD= pattern");
    }

    #[test]
    fn test_smtp_password_env() {
        let text = b"SMTP_PASSWORD=smtp_mail_server_secret";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect SMTP_PASSWORD= pattern");
    }

    #[test]
    fn test_api_key_lowercase() {
        let text = b"api_key=xyz789abcdef1234567890123";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect lowercase api_key=");
    }

    #[test]
    fn test_client_secret_lowercase() {
        let text = b"client_secret=abcdef123456789012345678901234";
        let result = detect_all(text);
        assert!(!result.matches.is_empty(), "Should detect lowercase client_secret=");
    }
}
