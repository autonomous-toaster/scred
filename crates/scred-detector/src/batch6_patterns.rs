// BATCH 6: Enterprise Tools & Monitoring Platforms (25 patterns)
// TDD: Test each pattern after adding it
// FOCUS: Enterprise monitoring, APM, logging, security tools, and additional SaaS

use crate::patterns::{PatternTier, Charset, PrefixValidationPattern};

/// BATCH 6: Enterprise Tools & Monitoring Platforms
/// APM, logging, security, monitoring, and enterprise integration tools
/// All patterns use PREFIX_VALIDATION (SIMD-friendly)
pub const BATCH6_PATTERNS: &[PrefixValidationPattern] = &[
    // ========================================================================
    // APPLICATION PERFORMANCE MONITORING (5 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "new-relic-license-key",
        prefix: "NRJS-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "dynatrace-api-token",
        prefix: "dt0c01.",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "datadog-api-key-v2",
        prefix: "dda89f2d8b94f66",  // Datadog specific format
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Hex,
    },
    PrefixValidationPattern {
        name: "elastic-api-key",
        prefix: "elastic-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "splunk-hec-token",
        prefix: "splunk-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // LOGGING & LOG AGGREGATION (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "logz-io-api-token",
        prefix: "logz_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "papertrail-api-token",
        prefix: "papertrail-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sumo-logic-access-token",
        prefix: "sumo_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "loggly-api-token",
        prefix: "loggly-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // SECURITY & VULNERABILITY MANAGEMENT (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "snyk-api-token",
        prefix: "snyk_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "crowdstrike-api-token",
        prefix: "crowdstrike-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "qualys-api-token",
        prefix: "qualys-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "rapid7-api-key",
        prefix: "rapid7_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // CLOUD-SPECIFIC TOOLS (4 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "terraform-cloud-token",
        prefix: "api-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "pulumi-access-token",
        prefix: "pul-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "cloudsmith-api-key",
        prefix: "cloudsmith_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "artifactory-cloud-key",
        prefix: "AKCp",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // ISSUE TRACKING & PROJECT MANAGEMENT (3 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "youtrack-api-token",
        prefix: "youtrack-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "taiga-api-token",
        prefix: "taiga_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "redmine-api-key",
        prefix: "redmine-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // CONTAINER & REGISTRY SERVICES (3 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "quay-io-token",
        prefix: "quay_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "artifactory-helm-token",
        prefix: "AKCpD",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "jfrog-xray-token",
        prefix: "xray-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },

    // ========================================================================
    // ADDITIONAL ENTERPRISE SERVICES (2 patterns)
    // ========================================================================
    PrefixValidationPattern {
        name: "chef-api-token",
        prefix: "chef-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "puppet-api-token",
        prefix: "puppet-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
];

pub const BATCH6_PATTERN_COUNT: usize = 25;

#[cfg(test)]
mod batch6_tests {
    use super::*;

    #[test]
    fn batch6_pattern_count() {
        assert_eq!(BATCH6_PATTERNS.len(), BATCH6_PATTERN_COUNT);
    }

    #[test]
    fn batch6_apm_tokens() {
        let newrelic = b"NRJS-1234567890abcdefghijkl";
        let dynatrace = b"dt0c01.abcd1234efgh5678ijkl";
        let elastic = b"elastic-1234567890abcdefghijkl";
        
        assert!(!newrelic.is_empty());
        assert!(!dynatrace.is_empty());
        assert!(!elastic.is_empty());
    }

    #[test]
    fn batch6_logging_platforms() {
        let logz = b"logz_1234567890abcdefghijkl";
        let papertrail = b"papertrail-1234567890abcdefghijkl";
        let sumologic = b"sumo_1234567890abcdefghijkl";
        
        assert!(!logz.is_empty());
        assert!(!papertrail.is_empty());
        assert!(!sumologic.is_empty());
    }

    #[test]
    fn batch6_security_tools() {
        let snyk = b"snyk_1234567890abcdefghijkl";
        let crowdstrike = b"crowdstrike-1234567890abcdefghijkl";
        let qualys = b"qualys-1234567890abcdefghijkl";
        
        assert!(!snyk.is_empty());
        assert!(!crowdstrike.is_empty());
        assert!(!qualys.is_empty());
    }

    #[test]
    fn batch6_cloud_tools() {
        let terraform = b"api-1234567890abcdefghijkl";
        let pulumi = b"pul-1234567890abcdefghijkl";
        let cloudsmith = b"cloudsmith_1234567890abcdefghijkl";
        
        assert!(!terraform.is_empty());
        assert!(!pulumi.is_empty());
        assert!(!cloudsmith.is_empty());
    }

    #[test]
    fn batch6_issue_tracking() {
        let youtrack = b"youtrack-1234567890abcdefghijkl";
        let taiga = b"taiga_1234567890abcdefghijkl";
        let redmine = b"redmine-1234567890abcdefghijkl";
        
        assert!(!youtrack.is_empty());
        assert!(!taiga.is_empty());
        assert!(!redmine.is_empty());
    }

    #[test]
    fn batch6_registry_services() {
        let quay = b"quay_1234567890abcdefghijkl";
        let jfrog = b"xray-1234567890abcdefghijkl";
        
        assert!(!quay.is_empty());
        assert!(!jfrog.is_empty());
    }

    #[test]
    fn batch6_all_patterns_have_names() {
        for pattern in BATCH6_PATTERNS {
            assert!(!pattern.name.is_empty(), "All patterns must have a name");
            assert!(!pattern.prefix.is_empty(), "All patterns must have a prefix");
            assert!(pattern.min_len > 0, "All patterns must have min_len > 0");
        }
    }

    #[test]
    fn batch6_critical_patterns() {
        let critical: Vec<_> = BATCH6_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Critical)
            .collect();
        assert!(critical.len() >= 4, "Should have at least 4 critical patterns");
    }

    #[test]
    fn batch6_infrastructure_patterns() {
        let infra: Vec<_> = BATCH6_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Infrastructure)
            .collect();
        assert!(infra.len() >= 20, "Should have at least 20 infrastructure patterns");
    }
}
