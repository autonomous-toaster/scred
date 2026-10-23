use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum PatternSelector {
    /// All 274 patterns
    All,

    /// No patterns (don't detect/redact anything)
    None,

    /// Specific tiers (e.g., [Critical, ApiKeys])
    Tiers(Vec<RiskTier>),

    /// Select by pattern type (performance-based: FastPrefix, StructuredFormat, RegexBased)
    Type(Vec<String>), // "fast", "structured", "regex"

    /// Exact pattern names
    Patterns(Vec<String>),

    /// By tags (exact match)
    Tags(Vec<String>),

    /// Wildcard matching (e.g., "aws-*", "github-*")
    Wildcard(String),

    /// Regex pattern matching
    Regex(Vec<String>),
}

impl PatternSelector {
    /// Decide a selector against a pattern *tier*.
    ///
    /// Callers that redact at runtime only hold a numeric pattern type, not a
    /// name, so name/glob/tag selectors cannot be resolved here: those return
    /// `None` and the caller decides (the engine falls back to "allowed", so a
    /// name-based selection never silently disables redaction).
    pub fn allows_tier(&self, tier: RiskTier) -> Option<bool> {
        match self {
            PatternSelector::All => Some(true),
            PatternSelector::None => Some(false),
            PatternSelector::Tiers(tiers) => Some(tiers.contains(&tier)),
            _ => None,
        }
    }
}

impl Default for PatternSelector {
    fn default() -> Self {
        PatternSelector::default_detect()
    }
}

// ============================================================================
// CompositePatternSelector: Handle mixed filters (tiers + globs + exclusions)
// ============================================================================
