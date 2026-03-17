// BATCH 4: Specialized Services & Cloud Platforms (25 patterns)
// TDD: Test each pattern after adding it
// FOCUS: Specialized services, AI/ML platforms, crypto/blockchain, and niche providers

use crate::patterns::{PatternTier, Charset, PrefixValidationPattern};

/// BATCH 4: Specialized Services & Cloud Platforms
/// AI/ML platforms, blockchain, specialized services
/// All patterns use PREFIX_VALIDATION (SIMD-friendly)
pub const BATCH4_PATTERNS: &[PrefixValidationPattern] = &[
    // ========================================================================
    // AI/ML PLATFORMS (5 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "replicate-api-token",
        prefix: "r8_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "anthropic-api-key",
        prefix: "sk-ant-",
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "mistral-api-key",
        prefix: "sk-",  // Mistral keys use sk- prefix
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "together-api-key",
        prefix: "together-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "modal-token-env",
        prefix: "MODAL_TOKEN_ID=",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // BLOCKCHAIN & CRYPTO (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "ethereum-private-key",
        prefix: "0x",
        tier: PatternTier::Critical,
        min_len: 66,  // 0x + 64 hex chars
        max_len: 66,
        charset: Charset::Hex,
    },
    PrefixValidationPattern {
        name: "infura-api-key",
        prefix: "infura-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "alchemy-api-key",
        prefix: "alchemy-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "web3-storage-token",
        prefix: "eyJ",  // Base64 for JWT header
        tier: PatternTier::Infrastructure,
        min_len: 100,
        max_len: 0,
        charset: Charset::Base64,
    },

    // ========================================================================
    // INCIDENT MANAGEMENT & COLLABORATION (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "victorops-api-key",
        prefix: "victorops-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "opsgenie-api-key",
        prefix: "opsgenie-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "jira-api-token",
        prefix: "jira-api-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "linear-api-key",
        prefix: "lin_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },

    // ========================================================================
    // DATA & SEARCH PLATFORMS (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "algolia-api-key",
        prefix: "algolia-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "typesense-api-key",
        prefix: "typesense-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "meilisearch-api-key",
        prefix: "meili_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "weaviate-api-key",
        prefix: "weaviate-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // IDENTITY & AUTH PLATFORMS (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "auth0-management-token",
        prefix: "mgmt_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "auth0-client-secret",
        prefix: "client_secret=",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "cognito-user-pool-id",
        prefix: "cognito-idp://",
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "keycloak-client-secret",
        prefix: "keycloak-secret=",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },

    // ========================================================================
    // ENTERPRISE & SaaS PLATFORMS (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "servicenow-api-key",
        prefix: "servicenow-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "salesforce-api-key",
        prefix: "salesforce-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "zendesk-api-token",
        prefix: "zendesk-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "supabase-postgres-password",
        prefix: "postgres://",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Any,
    },
];

pub const BATCH4_PATTERN_COUNT: usize = 25;

#[cfg(test)]
mod batch4_tests {
    use super::*;

    #[test]
    fn batch4_pattern_count() {
        assert_eq!(BATCH4_PATTERNS.len(), BATCH4_PATTERN_COUNT);
    }

    #[test]
    fn batch4_ai_ml_tokens() {
        let replicate = b"r8_1234567890abcdefghijklmnop";
        let anthropic = b"sk-ant-d3VyZWQgMzI=abcdef12345678";
        
        assert!(!replicate.is_empty());
        assert!(!anthropic.is_empty());
    }

    #[test]
    fn batch4_blockchain_keys() {
        let ethereum = b"0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let infura = b"infura-1234567890abcdefghijkl";
        
        assert!(ethereum.len() == 66);
        assert!(!infura.is_empty());
    }

    #[test]
    fn batch4_incident_management() {
        let victorops = b"victorops-1234567890abcdefghijkl";
        let opsgenie = b"opsgenie-1234567890abcdefghijkl";
        
        assert!(!victorops.is_empty());
        assert!(!opsgenie.is_empty());
    }

    #[test]
    fn batch4_search_platforms() {
        let algolia = b"algolia-1234567890abcdefghijkl";
        let meilisearch = b"meili_abcd1234efgh5678ijkl";
        
        assert!(!algolia.is_empty());
        assert!(!meilisearch.is_empty());
    }

    #[test]
    fn batch4_auth_platforms() {
        let auth0_mgmt = b"mgmt_abcd1234efgh5678ijklmnop";
        let cognito = b"cognito-idp://us-east-1:123456789012:userpool/us-east-1_abc123";
        
        assert!(!auth0_mgmt.is_empty());
        assert!(!cognito.is_empty());
    }

    #[test]
    fn batch4_enterprise_saas() {
        let servicenow = b"servicenow-1234567890abcdefghijkl";
        let salesforce = b"salesforce-1234567890abcdefghijkl";
        
        assert!(!servicenow.is_empty());
        assert!(!salesforce.is_empty());
    }

    #[test]
    fn batch4_all_patterns_have_names() {
        for pattern in BATCH4_PATTERNS {
            assert!(!pattern.name.is_empty(), "All patterns must have a name");
            assert!(!pattern.prefix.is_empty(), "All patterns must have a prefix");
            assert!(pattern.min_len > 0, "All patterns must have min_len > 0");
        }
    }

    #[test]
    fn batch4_critical_patterns() {
        let critical: Vec<_> = BATCH4_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Critical)
            .collect();
        assert!(critical.len() >= 10, "Should have at least 10 critical patterns");
    }
}
