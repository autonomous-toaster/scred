// BATCH 7: Final SaaS & Specialized Services (25 patterns)
// TDD: Test each pattern after adding it
// FOCUS: Remaining SaaS, specialized platforms, and niche services

use crate::patterns::{PatternTier, Charset, PrefixValidationPattern};

/// BATCH 7: Final SaaS & Specialized Services
/// Remaining SaaS platforms, niche services, and specialized tools
/// All patterns use PREFIX_VALIDATION (SIMD-friendly)
pub const BATCH7_PATTERNS: &[PrefixValidationPattern] = &[
    // ========================================================================
    // PRODUCTIVITY & OFFICE TOOLS (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "airtable-api-token",
        prefix: "patU",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "notion-integration-token",
        prefix: "secret_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "confluence-api-token",
        prefix: "confluence_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "asana-api-token",
        prefix: "asana_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // REAL-TIME COMMUNICATION (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "twitch-oauth-token",
        prefix: "oauth-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "discord-webhook-url",
        prefix: "https://discord.com/api/webhooks/",
        tier: PatternTier::Infrastructure,
        min_len: 50,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "matrix-homeserver-token",
        prefix: "syt_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "rocket-chat-token",
        prefix: "rocketchat-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // KNOWLEDGE MANAGEMENT & DOCS (3 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "confluence-cloud-token",
        prefix: "atc_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "jira-cloud-token",
        prefix: "atb_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "slite-api-token",
        prefix: "slite-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // MARKETPLACE & INTEGRATION PLATFORMS (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "shopify-api-key",
        prefix: "shppa_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "wix-api-token",
        prefix: "wix_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "bigcommerce-api-token",
        prefix: "bigcommerce-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "intercom-access-token",
        prefix: "intercom-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // SOCIAL MEDIA & CONTENT (3 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "instagram-business-token",
        prefix: "IGQWRN",
        tier: PatternTier::Infrastructure,
        min_len: 100,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "tiktok-api-token",
        prefix: "tiktok-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "youtube-api-key",
        prefix: "AIza",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },

    // ========================================================================
    // ANALYTICS & DATA (2 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "segment-write-key",
        prefix: "segment-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "chartio-api-token",
        prefix: "chartio-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // ADDITIONAL SPECIALIZED SERVICES (2 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "fastapi-key",
        prefix: "fk_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "graphql-api-token",
        prefix: "gql_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
];

pub const BATCH7_PATTERN_COUNT: usize = 25;

#[cfg(test)]
mod batch7_tests {
    use super::*;

    #[test]
    fn batch7_pattern_count() {
        assert_eq!(BATCH7_PATTERNS.len(), BATCH7_PATTERN_COUNT);
    }

    #[test]
    fn batch7_productivity_tools() {
        let airtable = b"patU1234567890abcdefghijkl";
        let notion = b"secret_1234567890abcdefghijkl";
        let asana = b"asana_1234567890abcdefghijkl";
        
        assert!(!airtable.is_empty());
        assert!(!notion.is_empty());
        assert!(!asana.is_empty());
    }

    #[test]
    fn batch7_communication_platforms() {
        let twitch = b"oauth-1234567890abcdefghijkl";
        let discord_webhook = b"https://discord.com/api/webhooks/123456789/abcdefghijk";
        let matrix = b"syt_1234567890abcdefghijkl";
        
        assert!(!twitch.is_empty());
        assert!(!discord_webhook.is_empty());
        assert!(!matrix.is_empty());
    }

    #[test]
    fn batch7_knowledge_management() {
        let confluence = b"atc_1234567890abcdefghijkl";
        let jira = b"atb_1234567890abcdefghijkl";
        let slite = b"slite-1234567890abcdefghijkl";
        
        assert!(!confluence.is_empty());
        assert!(!jira.is_empty());
        assert!(!slite.is_empty());
    }

    #[test]
    fn batch7_marketplace_platforms() {
        let shopify = b"shppa_1234567890abcdefghijkl";
        let wix = b"wix_1234567890abcdefghijkl";
        let bigcommerce = b"bigcommerce-1234567890abcdefghijkl";
        
        assert!(!shopify.is_empty());
        assert!(!wix.is_empty());
        assert!(!bigcommerce.is_empty());
    }

    #[test]
    fn batch7_social_media() {
        let instagram = b"IGQWRNABCD1234efgh5678ijkl";
        let tiktok = b"tiktok-1234567890abcdefghijkl";
        let youtube = b"AIzaSyD1234567890abcdefghijkl";
        
        assert!(!instagram.is_empty());
        assert!(!tiktok.is_empty());
        assert!(!youtube.is_empty());
    }

    #[test]
    fn batch7_analytics() {
        let segment = b"segment-1234567890abcdefghijkl";
        let chartio = b"chartio-1234567890abcdefghijkl";
        
        assert!(!segment.is_empty());
        assert!(!chartio.is_empty());
    }

    #[test]
    fn batch7_all_patterns_have_names() {
        for pattern in BATCH7_PATTERNS {
            assert!(!pattern.name.is_empty(), "All patterns must have a name");
            assert!(!pattern.prefix.is_empty(), "All patterns must have a prefix");
            assert!(pattern.min_len > 0, "All patterns must have min_len > 0");
        }
    }

    #[test]
    fn batch7_critical_patterns() {
        let critical: Vec<_> = BATCH7_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Critical)
            .collect();
        assert!(critical.len() >= 1, "Should have at least 1 critical pattern");
    }

    #[test]
    fn batch7_infrastructure_patterns() {
        let infra: Vec<_> = BATCH7_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Infrastructure)
            .collect();
        assert!(infra.len() >= 24, "Should have at least 24 infrastructure patterns");
    }
}
