use super::PatternTier;

// MULTILINE MARKER PATTERNS (4 total)
// Detect multiline secrets like SSH keys using bounded lookahead
// Pattern: -----BEGIN <TYPE> PRIVATE KEY-----...(multiline)...-----END <TYPE> PRIVATE KEY-----
// ============================================================================

#[derive(Debug, Clone)]
pub struct MultilineMarkerPattern {
    pub name: &'static str,
    pub start_marker: &'static str,
    pub end_marker: &'static str,
    pub tier: PatternTier,
    pub max_lookahead: usize, // Max bytes to look ahead (SSH keys ~4KB max)
}

/// Generalized marker-based pattern for multiline secrets
/// Supports start_marker -> content -> end_marker format
/// Examples: SSH keys, certificates, PGP keys, kubeconfig
#[derive(Debug, Clone, Copy)]
pub struct GeneralizedMarkerPattern {
    pub name: &'static str,
    pub start_marker: &'static str,
    pub end_marker: &'static str,
    pub tier: PatternTier,
    pub max_lookahead: usize,

    // Optional validation keywords
    pub contains_keyword: Option<&'static str>, // e.g., Some("PRIVATE KEY")
    pub exclude_keyword: Option<&'static str>,  // e.g., Some("PUBLIC") to skip public keys

    // Content characteristics (optimization hints)
    pub min_body_len: usize, // Minimum content size between markers
    pub pattern_type: u16,   // Type ID for pattern classification (300+ for multiline)
}

impl From<MultilineMarkerPattern> for GeneralizedMarkerPattern {
    fn from(p: MultilineMarkerPattern) -> Self {
        GeneralizedMarkerPattern {
            name: p.name,
            start_marker: p.start_marker,
            end_marker: p.end_marker,
            tier: p.tier,
            max_lookahead: p.max_lookahead,
            contains_keyword: None,
            exclude_keyword: None,
            min_body_len: 0,
            pattern_type: 300, // Default pattern type for multiline markers
        }
    }
}

pub const MULTILINE_MARKER_PATTERNS: &[MultilineMarkerPattern] = &[
    MultilineMarkerPattern {
        name: "ssh-rsa-private-key",
        start_marker: "-----BEGIN RSA PRIVATE KEY-----",
        end_marker: "-----END RSA PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
    },
    MultilineMarkerPattern {
        name: "ssh-openssh-private-key",
        start_marker: "-----BEGIN OPENSSH PRIVATE KEY-----",
        end_marker: "-----END OPENSSH PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
    },
    MultilineMarkerPattern {
        name: "ssh-private-key",
        start_marker: "-----BEGIN PRIVATE KEY-----",
        end_marker: "-----END PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
    },
    MultilineMarkerPattern {
        name: "ssh-ec-private-key",
        start_marker: "-----BEGIN EC PRIVATE KEY-----",
        end_marker: "-----END EC PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
    },
    // X.509 Certificate patterns (Phase 4b)
    MultilineMarkerPattern {
        name: "x509-certificate",
        start_marker: "-----BEGIN CERTIFICATE-----",
        end_marker: "-----END CERTIFICATE-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 10240, // Certificates can be 5-10KB
    },
    MultilineMarkerPattern {
        name: "x509-certificate-request",
        start_marker: "-----BEGIN CERTIFICATE REQUEST-----",
        end_marker: "-----END CERTIFICATE REQUEST-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 5120, // CSRs typically 1-3KB
    },
    MultilineMarkerPattern {
        name: "encrypted-private-key",
        start_marker: "-----BEGIN ENCRYPTED PRIVATE KEY-----",
        end_marker: "-----END ENCRYPTED PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 5120, // Encrypted keys typically 2-4KB
    },
    MultilineMarkerPattern {
        name: "public-key",
        start_marker: "-----BEGIN PUBLIC KEY-----",
        end_marker: "-----END PUBLIC KEY-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 3072, // Public keys typically 1-2KB
    },
    // PGP key patterns (Phase 4c)
    MultilineMarkerPattern {
        name: "pgp-private-key-block",
        start_marker: "-----BEGIN PGP PRIVATE KEY BLOCK-----",
        end_marker: "-----END PGP PRIVATE KEY BLOCK-----",
        tier: PatternTier::Critical,
        max_lookahead: 20480, // PGP keys can be 2-20KB
    },
    MultilineMarkerPattern {
        name: "pgp-public-key-block",
        start_marker: "-----BEGIN PGP PUBLIC KEY BLOCK-----",
        end_marker: "-----END PGP PUBLIC KEY BLOCK-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 6144, // PGP public keys typically 1-5KB
    },
    MultilineMarkerPattern {
        name: "pgp-message",
        start_marker: "-----BEGIN PGP MESSAGE-----",
        end_marker: "-----END PGP MESSAGE-----",
        tier: PatternTier::Critical,
        max_lookahead: 15360, // PGP messages are variable (up to 15KB)
    },
];

/// Generalized multiline pattern array - Phase A1 refactoring
/// Converts MultilineMarkerPattern to GeneralizedMarkerPattern for optimization
pub const GENERALIZED_MARKER_PATTERNS: &[GeneralizedMarkerPattern] = &[
    GeneralizedMarkerPattern {
        name: "ssh-rsa-private-key",
        start_marker: "-----BEGIN RSA PRIVATE KEY-----",
        end_marker: "-----END RSA PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
        contains_keyword: Some("PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 300,
    },
    GeneralizedMarkerPattern {
        name: "ssh-openssh-private-key",
        start_marker: "-----BEGIN OPENSSH PRIVATE KEY-----",
        end_marker: "-----END OPENSSH PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
        contains_keyword: Some("OPENSSH PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 301,
    },
    GeneralizedMarkerPattern {
        name: "ssh-private-key",
        start_marker: "-----BEGIN PRIVATE KEY-----",
        end_marker: "-----END PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
        contains_keyword: Some("PRIVATE KEY"),
        exclude_keyword: Some("RSA"), // Skip RSA (covered separately)
        min_body_len: 100,
        pattern_type: 302,
    },
    GeneralizedMarkerPattern {
        name: "ssh-ec-private-key",
        start_marker: "-----BEGIN EC PRIVATE KEY-----",
        end_marker: "-----END EC PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
        contains_keyword: Some("EC PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 303,
    },
    GeneralizedMarkerPattern {
        name: "x509-certificate",
        start_marker: "-----BEGIN CERTIFICATE-----",
        end_marker: "-----END CERTIFICATE-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 10240,
        contains_keyword: Some("CERTIFICATE"),
        exclude_keyword: Some("REQUEST"), // Skip CSRs (covered separately)
        min_body_len: 200,
        pattern_type: 304,
    },
    GeneralizedMarkerPattern {
        name: "x509-certificate-request",
        start_marker: "-----BEGIN CERTIFICATE REQUEST-----",
        end_marker: "-----END CERTIFICATE REQUEST-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 5120,
        contains_keyword: Some("CERTIFICATE REQUEST"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 305,
    },
    GeneralizedMarkerPattern {
        name: "encrypted-private-key",
        start_marker: "-----BEGIN ENCRYPTED PRIVATE KEY-----",
        end_marker: "-----END ENCRYPTED PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 5120,
        contains_keyword: Some("ENCRYPTED PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 306,
    },
    GeneralizedMarkerPattern {
        name: "public-key",
        start_marker: "-----BEGIN PUBLIC KEY-----",
        end_marker: "-----END PUBLIC KEY-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 3072,
        contains_keyword: Some("PUBLIC KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 307,
    },
    GeneralizedMarkerPattern {
        name: "pgp-private-key-block",
        start_marker: "-----BEGIN PGP PRIVATE KEY BLOCK-----",
        end_marker: "-----END PGP PRIVATE KEY BLOCK-----",
        tier: PatternTier::Critical,
        max_lookahead: 20480,
        contains_keyword: Some("PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 500,
        pattern_type: 308,
    },
    GeneralizedMarkerPattern {
        name: "pgp-public-key-block",
        start_marker: "-----BEGIN PGP PUBLIC KEY BLOCK-----",
        end_marker: "-----END PGP PUBLIC KEY BLOCK-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 6144,
        contains_keyword: Some("PUBLIC KEY"),
        exclude_keyword: None,
        min_body_len: 200,
        pattern_type: 309,
    },
    GeneralizedMarkerPattern {
        name: "pgp-message",
        start_marker: "-----BEGIN PGP MESSAGE-----",
        end_marker: "-----END PGP MESSAGE-----",
        tier: PatternTier::Critical,
        max_lookahead: 15360,
        contains_keyword: None,
        exclude_keyword: None,
        min_body_len: 50,
        pattern_type: 310,
    },
];

// ============================================================================
// REGEX PATTERNS (203 total) - NOT IMPLEMENTED YET
