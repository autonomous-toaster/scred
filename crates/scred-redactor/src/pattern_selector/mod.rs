/// Phase 4: Pattern Selector
/// Flexible pattern selection with 6 modes: All, Tiers, Patterns, Tags, Wildcard, Regex
use crate::metadata_cache::{MetadataCache, PatternMetadata, RiskTier};
use std::collections::HashSet;

pub mod composite;
pub mod filter;
pub mod selector;
pub mod tests;

// ============================================================================
// GlobMatcher: Simple, fast glob pattern matching (no regex)
// ============================================================================

/// Fast glob pattern matcher supporting * and ? wildcards
/// - '*' matches 0+ characters
/// - '?' matches exactly 1 character
/// - Everything else matches literally
pub struct GlobMatcher {
    pattern: String,
}

impl GlobMatcher {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }

    /// Match a name against this glob pattern
    /// Performance: O(name_len * pattern_len) worst case, typically O(name_len)
    pub fn matches(&self, name: &str) -> bool {
        self.matches_impl(name.as_bytes(), self.pattern.as_bytes(), 0, 0)
    }

    /// Recursive glob matching implementation
    fn matches_impl(&self, name: &[u8], pattern: &[u8], n_idx: usize, p_idx: usize) -> bool {
        // Base case: both exhausted
        if n_idx == name.len() && p_idx == pattern.len() {
            return true;
        }

        // Pattern exhausted but name still has chars
        if p_idx == pattern.len() {
            return n_idx == name.len();
        }

        let p_char = pattern[p_idx];
        match p_char {
            b'*' => {
                // Try matching 0 chars (skip *)
                if self.matches_impl(name, pattern, n_idx, p_idx + 1) {
                    return true;
                }
                // Try matching 1+ chars (advance name)
                if n_idx < name.len() {
                    return self.matches_impl(name, pattern, n_idx + 1, p_idx);
                }
                false
            }
            b'?' => {
                // Match exactly 1 char
                if n_idx >= name.len() {
                    return false;
                }
                self.matches_impl(name, pattern, n_idx + 1, p_idx + 1)
            }
            _ => {
                // Literal match
                if n_idx >= name.len() || name[n_idx] != p_char {
                    return false;
                }
                self.matches_impl(name, pattern, n_idx + 1, p_idx + 1)
            }
        }
    }
}

// ============================================================================
// NEW ARCHITECTURE: Separate Classification Dimensions (Phase 2)
// ============================================================================

/// DIMENSION 1: Severity - Actual risk if the secret is leaked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    Critical = 95, // Signing keys, database credentials, payment keys (high impact if leaked)
    High = 85,     // AWS/GitHub tokens, cloud credentials (high value targets)
    Medium = 65,   // Generic API keys, OAuth tokens (medium impact)
    Low = 40,      // Specialty/niche services (low impact, easy to rotate)
    Generic = 30,  // Regex patterns, generic formats (lowest confidence)
}

impl Severity {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
            Self::Generic => "GENERIC",
        }
    }

    pub fn risk_score(&self) -> u8 {
        *self as u8
    }

    pub fn should_redact_by_default(&self) -> bool {
        matches!(self, Self::Critical | Self::High)
    }

    pub fn parse_list(input: &str) -> Result<Vec<Self>, String> {
        input.split(',').map(|s| Self::from_str(s.trim())).collect()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "CRITICAL" | "95" => Ok(Self::Critical),
            "HIGH" | "85" => Ok(Self::High),
            "MEDIUM" | "65" => Ok(Self::Medium),
            "LOW" | "40" => Ok(Self::Low),
            "GENERIC" | "30" => Ok(Self::Generic),
            _ => Err(format!("Unknown severity: {}", s)),
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}%)", self.name(), self.risk_score())
    }
}

/// DIMENSION 2: Service Category - What type of service/system?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceCategory {
    CloudProvider,    // AWS, Azure, GCP
    PaymentProcessor, // Stripe, Square, PayPal, etc.
    CodeHost,         // GitHub, GitLab, Bitbucket, etc.
    Database,         // PostgreSQL, MongoDB, MySQL, etc.
    Messaging,        // Slack, Discord, Telegram, etc.
    Infrastructure,   // Docker, K8s, Vault, etcd, etc.
    Authentication,   // Auth0, Okta, KeyCloak, etc.
    Monitoring,       // Datadog, New Relic, Grafana, etc.
    Development,      // npm, PyPI, RubyGems, etc.
    AI,               // OpenAI, Anthropic, Huggingface, etc.
    Other,            // Everything else
}

impl ServiceCategory {
    pub fn name(&self) -> &'static str {
        match self {
            Self::CloudProvider => "CloudProvider",
            Self::PaymentProcessor => "PaymentProcessor",
            Self::CodeHost => "CodeHost",
            Self::Database => "Database",
            Self::Messaging => "Messaging",
            Self::Infrastructure => "Infrastructure",
            Self::Authentication => "Authentication",
            Self::Monitoring => "Monitoring",
            Self::Development => "Development",
            Self::AI => "AI",
            Self::Other => "Other",
        }
    }

    pub fn parse_list(input: &str) -> Result<Vec<Self>, String> {
        input.split(',').map(|s| Self::from_str(s.trim())).collect()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().replace("-", "").as_str() {
            "cloudprovider" => Ok(Self::CloudProvider),
            "paymentprocessor" => Ok(Self::PaymentProcessor),
            "codehost" => Ok(Self::CodeHost),
            "database" => Ok(Self::Database),
            "messaging" => Ok(Self::Messaging),
            "infrastructure" => Ok(Self::Infrastructure),
            "authentication" => Ok(Self::Authentication),
            "monitoring" => Ok(Self::Monitoring),
            "development" => Ok(Self::Development),
            "ai" => Ok(Self::AI),
            "other" => Ok(Self::Other),
            _ => Err(format!("Unknown service category: {}", s)),
        }
    }
}

impl std::fmt::Display for ServiceCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// DIMENSION 3: Pattern Kind - How is the pattern detected?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternKind {
    FixedPrefix,      // Starts with known prefix (e.g., AKIA for AWS)
    StructuredFormat, // JWT, PEM, Base64-encoded format
    RegexBased,       // Generic regex pattern
}

impl PatternKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::FixedPrefix => "FixedPrefix",
            Self::StructuredFormat => "StructuredFormat",
            Self::RegexBased => "RegexBased",
        }
    }

    pub fn parse_list(input: &str) -> Result<Vec<Self>, String> {
        input.split(',').map(|s| Self::from_str(s.trim())).collect()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().replace("-", "").as_str() {
            "fixedprefix" => Ok(Self::FixedPrefix),
            "structuredformat" => Ok(Self::StructuredFormat),
            "regexbased" => Ok(Self::RegexBased),
            _ => Err(format!("Unknown pattern kind: {}", s)),
        }
    }
}

impl std::fmt::Display for PatternKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// DIMENSION 4: Origin - Internal or external service?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    FirstParty, // Internal company services
    ThirdParty, // External vendor services
}

impl Origin {
    pub fn name(&self) -> &'static str {
        match self {
            Self::FirstParty => "FirstParty",
            Self::ThirdParty => "ThirdParty",
        }
    }
}

impl std::fmt::Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// PatternSelector Enum
// ============================================================================

// Re-exports
pub use composite::CompositePatternSelector;
pub use filter::PatternFilter;
pub use selector::PatternSelector;
