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

impl Default for PatternSelector {
    fn default() -> Self {
        PatternSelector::default_detect()
    }
}

// ============================================================================
// CompositePatternSelector: Handle mixed filters (tiers + globs + exclusions)
// ============================================================================

