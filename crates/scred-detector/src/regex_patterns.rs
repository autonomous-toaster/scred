// REGEX PATTERNS - Last Resort Detection
// For patterns that cannot be detected with simple prefix+length+charset validation
// Examples: Private keys (multiline), database URIs, SAML assertions, etc.
//
// These patterns use the full regex engine and are checked AFTER prefix patterns fail.
// Performance: ~10-20 µs per pattern (acceptable since used as fallback)

use crate::patterns::PatternTier;

/// Regex pattern definition
#[derive(Debug, Clone)]
pub struct RegexPattern {
    pub name: &'static str,
    pub pattern: &'static str,
    pub tier: crate::patterns::PatternTier,
    pub min_match_len: usize,
}

/// All critical REGEX patterns (23 total)
/// These are patterns that CANNOT be expressed as simple prefix+length+charset
pub const REGEX_PATTERNS: &[RegexPattern] = &[
    // ============================================================================
    // PRIVATE KEYS (4 patterns) - Multiline PEM format
    // ============================================================================
    RegexPattern {
        name: "private-key",
        pattern: r"(?i)-----BEGIN[ A-Z0-9_-]{0,100}PRIVATE KEY(?: BLOCK)?-----[\s\S-]{64,}?KEY(?: BLOCK)?-----",
        tier: PatternTier::Critical,
        min_match_len: 100,
    },
    RegexPattern {
        name: "privatekey",
        pattern: r"(?i)-----\s*?BEGIN[ A-Z0-9_-]*?PRIVATE KEY\s*?-----[\s\S]*?----\s*?END[ A-Z0-9_-]*? PRIVATE KEY\s*?-----",
        tier: PatternTier::Critical,
        min_match_len: 100,
    },
    RegexPattern {
        name: "base64-encoded-keys",
        pattern: r"-----BEGIN [A-Z]+ PRIVATE KEY-----[\s\S]{50,}-----END [A-Z]+ PRIVATE KEY-----",
        tier: PatternTier::Critical,
        min_match_len: 100,
    },
    RegexPattern {
        name: "openssh-private-key",
        pattern: r"-----BEGIN OPENSSH PRIVATE KEY-----[\s\S]+?-----END OPENSSH PRIVATE KEY-----",
        tier: PatternTier::Critical,
        min_match_len: 100,
    },

    // ============================================================================
    // CERTIFICATES (3 patterns) - X.509, public keys, kubeconfig
    // ============================================================================
    RegexPattern {
        name: "x509-certificate",
        pattern: r"-----BEGIN CERTIFICATE-----[\s\S]+?-----END CERTIFICATE-----",
        tier: PatternTier::Infrastructure,
        min_match_len: 50,
    },
    RegexPattern {
        name: "public-key-pem",
        pattern: r"-----BEGIN [A-Z]+ PUBLIC KEY-----[\s\S]+?-----END [A-Z]+ PUBLIC KEY-----",
        tier: PatternTier::Infrastructure,
        min_match_len: 50,
    },
    RegexPattern {
        name: "kubeconfig-credentials",
        pattern: r"certificate-authority-data:\s*([A-Za-z0-9+/]{50,}={0,2})",
        tier: PatternTier::Critical,
        min_match_len: 50,
    },

    // ============================================================================
    // DATABASE URIS (3 patterns) - Connection strings with credentials
    // Note: Prefix patterns in patterns.rs now have max_len bounds for streaming support
    // These regex patterns kept for reference but not used in detect_all()
    // ============================================================================
    RegexPattern {
        name: "mongodb-uri",
        pattern: r"mongodb(?:\+srv)?://(?:[a-zA-Z0-9_\-\.]+:[a-zA-Z0-9_\-\.]+@)?[a-zA-Z0-9\.\-:/?]+",
        tier: PatternTier::Critical,
        min_match_len: 30,
    },
    RegexPattern {
        name: "postgres-uri",
        pattern: r"postgres(?:ql)?://(?:[a-zA-Z0-9_\-\.]+:[a-zA-Z0-9_\-\.]+@)?[a-zA-Z0-9\.\-:/?]+",
        tier: PatternTier::Critical,
        min_match_len: 30,
    },
    RegexPattern {
        name: "mysql-uri",
        pattern: r"mysql://(?:[a-zA-Z0-9_\-\.]+:[a-zA-Z0-9_\-\.]+@)?[a-zA-Z0-9\.\-:/?]+",
        tier: PatternTier::Critical,
        min_match_len: 30,
    },

    // ============================================================================
    // COMPLEX CREDENTIALS (5 patterns) - Kubernetes, SAML, AWS, Azure, Google
    // ============================================================================
    RegexPattern {
        name: "kubernetes-service-account",
        pattern: r"eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9[\s\S]{100,}",  // JWT with K8s header
        tier: PatternTier::Critical,
        min_match_len: 100,
    },
    RegexPattern {
        name: "saml-assertion",
        pattern: r"<saml:Assertion[\s\S]{50,}</saml:Assertion>",
        tier: PatternTier::Critical,
        min_match_len: 50,
    },
    RegexPattern {
        name: "aws-session-token-json",
        pattern: r#"(?i)"SessionToken"\s*:\s*"[A-Za-z0-9/+=]{100,}""#,
        tier: PatternTier::Critical,
        min_match_len: 100,
    },
    RegexPattern {
        name: "azure-service-principal",
        pattern: r#"(?i)"client_secret"\s*:\s*"[A-Za-z0-9\-\.~]+"#,
        tier: PatternTier::Critical,
        min_match_len: 20,
    },
    RegexPattern {
        name: "google-service-account",
        pattern: r#"(?i)"private_key"\s*:\s*"-----BEGIN[\s\S]{500,}-----END[^"]+""#,
        tier: PatternTier::Critical,
        min_match_len: 500,
    },

    // ============================================================================
    // MISCELLANEOUS MULTILINE (3 patterns) - Special formats
    // ============================================================================
    RegexPattern {
        name: "ssh-rsa-key",
        pattern: r"ssh-rsa\s+[A-Za-z0-9+/=]{300,}",
        tier: PatternTier::Infrastructure,
        min_match_len: 300,
    },
    RegexPattern {
        name: "ssh-ed25519-key",
        pattern: r"ssh-ed25519\s+[A-Za-z0-9+/=]{68}",
        tier: PatternTier::Infrastructure,
        min_match_len: 68,
    },
    RegexPattern {
        name: "pgp-private-key",
        pattern: r"-----BEGIN PGP PRIVATE KEY BLOCK-----[\s\S]+?-----END PGP PRIVATE KEY BLOCK-----",
        tier: PatternTier::Critical,
        min_match_len: 100,
    },
];

pub const REGEX_PATTERN_COUNT: usize = 18; // Critical multiline patterns

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_pattern_count() {
        assert_eq!(REGEX_PATTERNS.len(), REGEX_PATTERN_COUNT);
    }

    #[test]
    fn test_regex_patterns_have_names() {
        for pattern in REGEX_PATTERNS {
            assert!(!pattern.name.is_empty(), "All patterns must have a name");
            assert!(!pattern.pattern.is_empty(), "All patterns must have a regex");
            assert!(pattern.min_match_len > 0, "All patterns must have min_match_len");
        }
    }

    #[test]
    fn test_critical_patterns() {
        let critical: Vec<_> = REGEX_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Critical)
            .collect();
        assert!(
            critical.len() >= 10,
            "Should have at least 10 critical regex patterns"
        );
    }
}
