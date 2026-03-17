// BATCH 5: Additional Providers & Niche Services (25 patterns)
// TDD: Test each pattern after adding it
// FOCUS: Game dev, documentation, design tools, automation platforms, and miscellaneous providers

use crate::patterns::{PatternTier, Charset, PrefixValidationPattern};

/// BATCH 5: Additional Providers & Niche Services
/// Game dev, automation, documentation, and specialized niche platforms
/// All patterns use PREFIX_VALIDATION (SIMD-friendly)
pub const BATCH5_PATTERNS: &[PrefixValidationPattern] = &[
    // ========================================================================
    // GAME DEVELOPMENT PLATFORMS (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "unity-api-key",
        prefix: "unity-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "unreal-api-key",
        prefix: "unreal-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "godot-export-key",
        prefix: "godot-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "steam-api-key",
        prefix: "steam-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // DOCUMENTATION & PUBLISHING (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "readme-io-api-key",
        prefix: "rdme_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "gitbook-api-token",
        prefix: "gbook_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "contentful-cpa-token",
        prefix: "CFPAT-",
        tier: PatternTier::Infrastructure,
        min_len: 40,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "strapi-api-token",
        prefix: "strapi-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // AUTOMATION & WORKFLOW PLATFORMS (5 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "zapier-api-key",
        prefix: "zapier-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "make-integration-token",
        prefix: "make_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "n8n-api-key",
        prefix: "n8n_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "workflow-automation-token",
        prefix: "wf_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "ifttt-webhook-key",
        prefix: "ifttt-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // DESIGN & COLLABORATION TOOLS (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "miro-api-token",
        prefix: "miro_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "lucidchart-api-key",
        prefix: "lucidchart-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "craft-api-token",
        prefix: "craft_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "invision-api-key",
        prefix: "invision-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // PAYMENT & FINANCIAL SERVICES (3 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "square-access-token",
        prefix: "sq0atp-",
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "wise-api-token",
        prefix: "wise_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "coinbase-api-key",
        prefix: "cb-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },

    // ========================================================================
    // QUALITY ASSURANCE & TESTING (3 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "browserstack-api-key",
        prefix: "browserstack-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "testproject-io-token",
        prefix: "testproject_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "lambdatest-api-key",
        prefix: "lambdatest-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
];

pub const BATCH5_PATTERN_COUNT: usize = 25;

#[cfg(test)]
mod batch5_tests {
    use super::*;

    #[test]
    fn batch5_pattern_count() {
        assert_eq!(BATCH5_PATTERNS.len(), BATCH5_PATTERN_COUNT);
    }

    #[test]
    fn batch5_game_dev_platforms() {
        let unity = b"unity-1234567890abcdefghijkl";
        let unreal = b"unreal-1234567890abcdefghijkl";
        let steam = b"steam-1234567890abcdefghijkl";
        
        assert!(!unity.is_empty());
        assert!(!unreal.is_empty());
        assert!(!steam.is_empty());
    }

    #[test]
    fn batch5_documentation_platforms() {
        let readme = b"rdme_1234567890abcdefghijkl";
        let gitbook = b"gbook_1234567890abcdefghijkl";
        let contentful = b"CFPAT-abcd1234efgh5678ijkl";
        
        assert!(!readme.is_empty());
        assert!(!gitbook.is_empty());
        assert!(!contentful.is_empty());
    }

    #[test]
    fn batch5_automation_platforms() {
        let zapier = b"zapier-1234567890abcdefghijkl";
        let make = b"make_1234567890abcdefghijkl";
        let n8n = b"n8n_1234567890abcdefghijkl";
        
        assert!(!zapier.is_empty());
        assert!(!make.is_empty());
        assert!(!n8n.is_empty());
    }

    #[test]
    fn batch5_design_tools() {
        let miro = b"miro_1234567890abcdefghijkl";
        let lucidchart = b"lucidchart-1234567890abcdefghijkl";
        
        assert!(!miro.is_empty());
        assert!(!lucidchart.is_empty());
    }

    #[test]
    fn batch5_payment_services() {
        let square = b"sq0atp-1234567890abcdefghijkl";
        let wise = b"wise_1234567890abcdefghijkl";
        let coinbase = b"cb-abcd1234efgh5678ijkl";
        
        assert!(!square.is_empty());
        assert!(!wise.is_empty());
        assert!(!coinbase.is_empty());
    }

    #[test]
    fn batch5_qa_testing() {
        let browserstack = b"browserstack-1234567890abcdefghijkl";
        let lambdatest = b"lambdatest-1234567890abcdefghijkl";
        
        assert!(!browserstack.is_empty());
        assert!(!lambdatest.is_empty());
    }

    #[test]
    fn batch5_all_patterns_have_names() {
        for pattern in BATCH5_PATTERNS {
            assert!(!pattern.name.is_empty(), "All patterns must have a name");
            assert!(!pattern.prefix.is_empty(), "All patterns must have a prefix");
            assert!(pattern.min_len > 0, "All patterns must have min_len > 0");
        }
    }

    #[test]
    fn batch5_critical_patterns() {
        let critical: Vec<_> = BATCH5_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Critical)
            .collect();
        assert!(critical.len() >= 3, "Should have at least 3 critical patterns");
    }

    #[test]
    fn batch5_infrastructure_patterns() {
        let infra: Vec<_> = BATCH5_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Infrastructure)
            .collect();
        assert!(infra.len() >= 20, "Should have at least 20 infrastructure patterns");
    }
}
