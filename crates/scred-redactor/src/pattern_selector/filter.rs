use super::*;

pub enum PatternFilter {
    /// Match by tier: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS
    Tier(RiskTier),

    /// Match by glob pattern name: mysql*, aws-*, github-*
    GlobName(String),

    /// Exclude pattern by glob: !test-*, !mock-*
    Exclude(String),
}

impl PatternFilter {
    /// Check if this filter matches a pattern name and tier
    pub fn matches(&self, pattern_name: &str, pattern_tier: RiskTier) -> bool {
        match self {
            PatternFilter::Tier(tier) => *tier == pattern_tier,
            PatternFilter::GlobName(glob) => {
                let matcher = GlobMatcher::new(glob);
                matcher.matches(pattern_name)
            }
            PatternFilter::Exclude(_) => {
                // Exclusions are handled separately
                false
            }
        }
    }

    /// Check if this exclusion filter should block a pattern
    pub fn should_exclude(&self, pattern_name: &str) -> bool {
        if let PatternFilter::Exclude(glob) = self {
            let matcher = GlobMatcher::new(glob);
            matcher.matches(pattern_name)
        } else {
            false
        }
    }

    /// Parse a single filter from string
    /// Examples: "CRITICAL", "mysql*", "!test-*", "exclude:dummy-*"
    pub fn from_str(s: &str) -> Result<Self, String> {
        let s = s.trim();

        // Handle exclusion patterns
        if s.starts_with('!') {
            return Ok(PatternFilter::Exclude(s[1..].to_string()));
        }

        if s.starts_with("exclude:") {
            return Ok(PatternFilter::Exclude(s[8..].to_string()));
        }

        // Handle tier names (case-insensitive)
        match s.to_uppercase().as_str() {
            "CRITICAL" => return Ok(PatternFilter::Tier(RiskTier::Critical)),
            "API_KEYS" => return Ok(PatternFilter::Tier(RiskTier::ApiKeys)),
            "INFRASTRUCTURE" => return Ok(PatternFilter::Tier(RiskTier::Infrastructure)),
            "SERVICES" => return Ok(PatternFilter::Tier(RiskTier::Services)),
            "PATTERNS" => return Ok(PatternFilter::Tier(RiskTier::Patterns)),
            "ALL" => return Ok(PatternFilter::Tier(RiskTier::Critical)), // Special case: ALL means all tiers
            _ => {}
        }

        // Otherwise treat as glob pattern name
        Ok(PatternFilter::GlobName(s.to_string()))
    }
}

