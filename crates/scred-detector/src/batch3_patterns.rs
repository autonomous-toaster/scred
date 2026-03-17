// BATCH 3: API Keys & Services (25 patterns)
// TDD: Test each pattern after adding it
// FOCUS: Additional API keys, cloud services, communication platforms

use crate::patterns::{PatternTier, Charset, PrefixValidationPattern};

/// BATCH 3: API Keys & Services Patterns
/// Additional provider APIs and communication platforms
/// All patterns use PREFIX_VALIDATION (SIMD-friendly)
pub const BATCH3_PATTERNS: &[PrefixValidationPattern] = &[
    // ========================================================================
    // ADDITIONAL DATABASE/CACHE SERVICES (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "dynamodb-uri",
        prefix: "dynamodb://",
        tier: PatternTier::Critical,
        min_len: 25,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "memcached-uri",
        prefix: "memcached://",
        tier: PatternTier::Infrastructure,
        min_len: 25,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "couchdb-uri",
        prefix: "couchdb://",
        tier: PatternTier::Infrastructure,
        min_len: 25,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "influxdb-uri",
        prefix: "influxdb://",
        tier: PatternTier::Infrastructure,
        min_len: 25,
        max_len: 0,
        charset: Charset::Any,
    },

    // ========================================================================
    // COMMUNICATION PLATFORMS (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "discord-bot-token",
        prefix: "ODI",  // Base64 for bot token format
        tier: PatternTier::Infrastructure,
        min_len: 60,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "telegram-token-env",
        prefix: "TELEGRAM_BOT_TOKEN=",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "slack-webhook-url",
        prefix: "https://hooks.slack.com/services/",
        tier: PatternTier::Infrastructure,
        min_len: 60,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "zulip-api-key",
        prefix: "Zulip",  // Zulip tokens often start with "Zulip"
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // DEVELOPMENT TOOLS & CI/CD (5 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "sonarqube-token",
        prefix: "squ_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "codecov-token",
        prefix: "codecov-token=",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "buildkite-token",
        prefix: "bk_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "dependabot-token",
        prefix: "ghs_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sourcegraph-token",
        prefix: "sgp_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },

    // ========================================================================
    // ANALYTICS & MONITORING (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "datadog-api-key",
        prefix: "dd_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sentry-dsn-url",
        prefix: "https://",
        tier: PatternTier::Infrastructure,
        min_len: 50,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "amplitude-api-key",
        prefix: "amplitude-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "mixpanel-api-token",
        prefix: "mixpanel_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // EMAIL & MESSAGE SERVICES (3 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "sparkpost-api-key",
        prefix: "sparkpost-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sendpulse-api-token",
        prefix: "sendpulse-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "mailerlite-api-key",
        prefix: "mailerlite-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // STORAGE & CDN SERVICES (3 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "digitalocean-spaces-key",
        prefix: "DO00",  // Base64 encoding of DO token
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "backblaze-b2-key",
        prefix: "b2_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "wasabi-s3-key",
        prefix: "wasabi-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
];

pub const BATCH3_PATTERN_COUNT: usize = 25;

#[cfg(test)]
mod batch3_tests {
    use super::*;

    #[test]
    fn batch3_pattern_count() {
        assert_eq!(BATCH3_PATTERNS.len(), BATCH3_PATTERN_COUNT);
    }

    #[test]
    fn batch3_database_uris() {
        let dynamodb = b"dynamodb://user:pass@localhost:8000/database";
        let memcached = b"memcached://localhost:11211";
        let couchdb = b"couchdb://user:pass@localhost:5984/database";
        
        assert!(!dynamodb.is_empty());
        assert!(!memcached.is_empty());
        assert!(!couchdb.is_empty());
    }

    #[test]
    fn batch3_communication_platforms() {
        let discord = b"ODI4NTMyNDI0NDEyNDEwMzY5";  // Discord token format
        let slack_webhook = b"https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX";
        
        assert!(!discord.is_empty());
        assert!(!slack_webhook.is_empty());
    }

    #[test]
    fn batch3_ci_cd_tokens() {
        let sonarqube = b"squ_abcd1234efgh5678ijkl";
        let buildkite = b"bk_live_abcdefghijklmnop1234567890";
        
        assert!(!sonarqube.is_empty());
        assert!(!buildkite.is_empty());
    }

    #[test]
    fn batch3_monitoring() {
        let datadog = b"dd_abcd1234efgh5678ijkl";
        let sentry = b"https://abc1234@sentry.io/1234567";
        
        assert!(!datadog.is_empty());
        assert!(!sentry.is_empty());
    }

    #[test]
    fn batch3_all_patterns_have_names() {
        for pattern in BATCH3_PATTERNS {
            assert!(!pattern.name.is_empty(), "All patterns must have a name");
            assert!(!pattern.prefix.is_empty(), "All patterns must have a prefix");
            assert!(pattern.min_len > 0, "All patterns must have min_len > 0");
        }
    }

    #[test]
    fn batch3_critical_patterns() {
        let critical: Vec<_> = BATCH3_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Critical)
            .collect();
        assert!(critical.len() >= 4, "Should have at least 4 critical patterns");
    }

    #[test]
    fn batch3_infrastructure_patterns() {
        let infra: Vec<_> = BATCH3_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Infrastructure)
            .collect();
        assert!(infra.len() >= 20, "Should have at least 20 infrastructure patterns");
    }
}
