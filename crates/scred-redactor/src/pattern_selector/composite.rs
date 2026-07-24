use super::*;

pub struct CompositePatternSelector {
    inclusions: Vec<PatternFilter>,
    exclusions: Vec<PatternFilter>,
}

impl CompositePatternSelector {
    /// Create from comma-separated filters
    /// Examples:
    ///   "CRITICAL"                              // Single tier
    ///   "CRITICAL,API_KEYS"                     // Multiple tiers
    ///   "mysql*,postgresql*"                    // Glob patterns only
    ///   "CRITICAL,mysql*"                       // Tier + glob
    ///   "CRITICAL,mysql*,!test-*"               // Tier + glob + exclusion
    ///   "CRITICAL,API_KEYS,aws-*,!dummy-*"     // Complex
    pub fn from_string(spec: &str) -> Result<Self, String> {
        let mut inclusions = Vec::new();
        let mut exclusions = Vec::new();

        for filter_str in spec.split(',') {
            let filter = PatternFilter::from_str(filter_str)?;

            match &filter {
                PatternFilter::Exclude(_) => exclusions.push(filter),
                _ => inclusions.push(filter),
            }
        }

        if inclusions.is_empty() {
            return Err("No inclusion filters specified".to_string());
        }

        Ok(Self {
            inclusions,
            exclusions,
        })
    }

    /// Check if a pattern should be selected
    /// Returns true if:
    /// 1. Pattern matches at least one inclusion filter, AND
    /// 2. Pattern does NOT match any exclusion filter
    pub fn matches(&self, pattern_name: &str, pattern_tier: RiskTier) -> bool {
        // Check exclusions first (fail fast)
        for exclusion in &self.exclusions {
            if exclusion.should_exclude(pattern_name) {
                return false;
            }
        }

        // Check inclusions (at least one must match)
        for inclusion in &self.inclusions {
            if inclusion.matches(pattern_name, pattern_tier) {
                return true;
            }
        }

        false
    }

    /// Get description of this selector
    pub fn description(&self) -> String {
        let inclusion_strs: Vec<String> = self
            .inclusions
            .iter()
            .map(|f| match f {
                PatternFilter::Tier(t) => format!("tier:{:?}", t),
                PatternFilter::GlobName(g) => format!("glob:{}", g),
                PatternFilter::Exclude(_) => unreachable!(),
            })
            .collect();

        let mut desc = inclusion_strs.join(", ");

        if !self.exclusions.is_empty() {
            let exclude_strs: Vec<String> = self
                .exclusions
                .iter()
                .map(|f| {
                    if let PatternFilter::Exclude(g) = f {
                        format!("!{}", g)
                    } else {
                        unreachable!()
                    }
                })
                .collect();
            desc.push_str(&format!(", excluding: {}", exclude_strs.join(", ")));
        }

        desc
    }
}

// ============================================================================
// PatternSelector - Original Implementation (Updated)
// ============================================================================

impl PatternSelector {
    /// Check if a pattern matches this selector
    pub fn matches(&self, metadata: &PatternMetadata) -> bool {
        match self {
            PatternSelector::All => true,
            PatternSelector::None => false,

            PatternSelector::Tiers(tiers) => tiers.iter().any(|t| t == &metadata.tier),

            PatternSelector::Type(_types) => {
                // TODO: Add pattern_type to PatternMetadata once integrated
                // For now, Type filtering happens at detector level
                true
            }

            PatternSelector::Patterns(names) => names.iter().any(|n| n == &metadata.name),

            PatternSelector::Tags(tags) => {
                tags.iter().any(|tag| {
                    if tag.ends_with('*') {
                        // Prefix match
                        let prefix = &tag[..tag.len() - 1];
                        metadata.tags.iter().any(|t| t.starts_with(prefix))
                    } else {
                        // Exact match
                        metadata.tags.contains(tag)
                    }
                })
            }

            PatternSelector::Wildcard(pattern) => self.wildcard_match_name(pattern, &metadata.name),

            PatternSelector::Regex(_regex_patterns) => {
                // Regex matching: for now, simplified
                false
            }
        }
    }

    /// Wildcard matching: "aws-*" matches "aws-access-key", etc.
    /// Uses efficient glob matching with * and ? support
    fn wildcard_match_name(&self, pattern: &str, name: &str) -> bool {
        let matcher = GlobMatcher::new(pattern);
        matcher.matches(name)
    }

    /// Get all matching pattern names from cache
    pub fn get_matching_patterns(&self, cache: &MetadataCache) -> Vec<String> {
        let mut matching = Vec::new();

        match self {
            PatternSelector::All => {
                // Get all pattern names
                for name in cache.all_pattern_names() {
                    matching.push(name.clone());
                }
            }

            PatternSelector::None => {
                // No patterns match
            }

            PatternSelector::Tiers(tiers) => {
                // Get patterns for each tier
                for tier in tiers {
                    if let Some(patterns) = cache.get_patterns_by_tier(tier) {
                        matching.extend_from_slice(patterns);
                    }
                }
            }

            PatternSelector::Patterns(names) => {
                // Filter to only specified names
                for name in names {
                    if cache.get_pattern(name).is_some() {
                        matching.push(name.to_string());
                    }
                }
            }

            PatternSelector::Tags(tags) => {
                // Collect all patterns with matching tags
                let mut seen = HashSet::new();
                for tag in tags {
                    if let Some(patterns) = cache.get_patterns_by_tag(tag) {
                        for pattern_name in patterns {
                            if seen.insert(pattern_name.clone()) {
                                matching.push(pattern_name.clone());
                            }
                        }
                    }
                }
            }

            PatternSelector::Wildcard(pattern) => {
                // Find all patterns matching wildcard
                for (name, _) in cache.all_patterns() {
                    if self.wildcard_match_name(pattern, name) {
                        matching.push(name.clone());
                    }
                }
            }

            PatternSelector::Regex(_regex_patterns) => {
                // Regex patterns - simplified for now
            }

            PatternSelector::Type(_types) => {
                // Type filtering happens at detector level
                // For now, return all patterns
                for name in cache.all_pattern_names() {
                    matching.push(name.clone());
                }
            }
        }

        matching
    }

    /// Count matching patterns
    pub fn count_matches(&self, cache: &MetadataCache) -> usize {
        self.get_matching_patterns(cache).len()
    }

    /// Get statistics about matching patterns by tier
    pub fn get_tier_distribution(&self, cache: &MetadataCache) -> Vec<(RiskTier, usize)> {
        let matching = self.get_matching_patterns(cache);
        let matching_set: HashSet<&String> = matching.iter().collect();

        let mut distribution = Vec::new();

        let tiers = vec![
            RiskTier::Critical,
            RiskTier::ApiKeys,
            RiskTier::Infrastructure,
            RiskTier::Services,
            RiskTier::Patterns,
        ];

        for tier in tiers {
            let count = if let Some(patterns) = cache.get_patterns_by_tier(&tier) {
                patterns.iter().filter(|p| matching_set.contains(p)).count()
            } else {
                0
            };

            if count > 0 {
                distribution.push((tier, count));
            }
        }

        distribution
    }
}

// ============================================================================
// Configuration Parser & Defaults
// ============================================================================

impl PatternSelector {
    /// Default detection: all Critical and ApiKeys patterns
    pub fn default_detect() -> Self {
        PatternSelector::Tiers(vec![RiskTier::Critical, RiskTier::ApiKeys])
    }

    /// Default redaction: Critical and ApiKeys only (exclude Infrastructure, Services, Patterns)
    pub fn default_redact() -> Self {
        PatternSelector::Tiers(vec![RiskTier::Critical, RiskTier::ApiKeys])
    }

    /// Check if a pattern string matches this selector (for testing)
    pub fn matches_pattern(&self, _pattern: &str, tier: RiskTier) -> bool {
        // Check if this tier matches the selector
        match self {
            PatternSelector::All => true,
            PatternSelector::None => false,

            PatternSelector::Tiers(tiers) => tiers.contains(&tier),

            // For other selectors, we can't easily match by tier alone
            // Would need full PatternMetadata
            _ => true, // Conservative: match if we don't have enough info
        }
    }

    /// Get description of selector
    pub fn description(&self) -> String {
        match self {
            PatternSelector::All => "All patterns".to_string(),
            PatternSelector::None => "No patterns".to_string(),
            PatternSelector::Tiers(tiers) => {
                let tier_names: Vec<String> = tiers.iter().map(|t| format!("{:?}", t)).collect();
                format!("Tiers: {}", tier_names.join(", "))
            }
            PatternSelector::Type(types) => {
                format!("Pattern Types: {}", types.join(", "))
            }
            PatternSelector::Patterns(names) => format!("Patterns: {}", names.len()),
            PatternSelector::Tags(tags) => format!("Tags: {}", tags.join(", ")),
            PatternSelector::Wildcard(pattern) => format!("Wildcard: {}", pattern),
            PatternSelector::Regex(patterns) => format!("Regex: {}", patterns.join(", ")),
        }
    }

    /// Parse selector from string format
    /// Examples:
    ///   "all"
    ///   "tier:critical,api_keys"
    ///   "patterns:aws-access-key,github-token"
    ///   "tags:aws,github"
    ///   "wildcard:aws-*"
    ///   "regex:^(aws|github)"
    pub fn from_str(spec: &str) -> Result<Self, String> {
        Self::from_string(spec)
    }

    pub fn from_string(spec: &str) -> Result<Self, String> {
        let spec_lower = spec.to_lowercase();

        // Handle "all" or "ALL"
        if spec_lower == "all" {
            return Ok(PatternSelector::All);
        }

        // Handle "none" or "NONE"
        if spec_lower == "none" {
            return Ok(PatternSelector::None);
        }

        // Handle pattern types: fast, structured, regex, or combinations
        match spec_lower.as_str() {
            "fast" | "fastprefix" => {
                return Ok(PatternSelector::Type(vec!["fast".to_string()]));
            }
            "structured" | "structuredformat" => {
                return Ok(PatternSelector::Type(vec!["structured".to_string()]));
            }
            "regex" | "regexbased" => {
                return Ok(PatternSelector::Type(vec!["regex".to_string()]));
            }
            _ => {}
        }

        // Handle type:X,Y
        if let Some(rest) = spec_lower.strip_prefix("type:") {
            let types: Vec<String> = rest.split(',').map(|s| s.trim().to_lowercase()).collect();
            return Ok(PatternSelector::Type(types));
        }

        // Handle comma-separated pattern types (e.g., "fast,structured")
        if !spec_lower.contains(':') && !spec_lower.contains('*') && !spec_lower.contains('^') {
            let parts: Vec<&str> = spec_lower.split(',').map(|s| s.trim()).collect();
            if parts.iter().all(|p| {
                matches!(
                    p,
                    &"fast"
                        | &"fastprefix"
                        | &"structured"
                        | &"structuredformat"
                        | &"regex"
                        | &"regexbased"
                )
            }) {
                let types: Vec<String> = parts
                    .iter()
                    .map(|p| match *p {
                        "fastprefix" => "fast".to_string(),
                        "structuredformat" => "structured".to_string(),
                        "regexbased" => "regex".to_string(),
                        other => other.to_string(),
                    })
                    .collect();
                if types
                    .iter()
                    .all(|t| matches!(t.as_str(), "fast" | "structured" | "regex"))
                {
                    return Ok(PatternSelector::Type(types));
                }
            }
        }

        // Handle tier:X,Y or TIER:X,Y
        if let Some(rest) = spec_lower.strip_prefix("tier:") {
            let tiers = rest
                .split(',')
                .map(|s| s.trim())
                .filter_map(|s| match s {
                    "critical" => Some(RiskTier::Critical),
                    "api_keys" => Some(RiskTier::ApiKeys),
                    "infrastructure" => Some(RiskTier::Infrastructure),
                    "services" => Some(RiskTier::Services),
                    "patterns" => Some(RiskTier::Patterns),
                    _ => None,
                })
                .collect::<Vec<_>>();

            if !tiers.is_empty() {
                return Ok(PatternSelector::Tiers(tiers));
            }
        }

        // Handle comma-separated tier names without prefix (e.g., "CRITICAL" or "CRITICAL,API_KEYS")
        if !spec_lower.contains(':') && !spec_lower.contains('*') && !spec_lower.contains('^') {
            let tiers = spec_lower
                .split(',')
                .map(|s| s.trim())
                .filter_map(|s| match s {
                    "critical" => Some(RiskTier::Critical),
                    "api_keys" => Some(RiskTier::ApiKeys),
                    "infrastructure" => Some(RiskTier::Infrastructure),
                    "services" => Some(RiskTier::Services),
                    "patterns" => Some(RiskTier::Patterns),
                    _ => None,
                })
                .collect::<Vec<_>>();

            if !tiers.is_empty() {
                return Ok(PatternSelector::Tiers(tiers));
            }
        }

        // Handle patterns:X,Y
        if let Some(rest) = spec_lower.strip_prefix("patterns:") {
            let patterns = rest.split(',').map(|s| s.trim().to_string()).collect();
            return Ok(PatternSelector::Patterns(patterns));
        }

        // Handle tags:X,Y
        if let Some(rest) = spec_lower.strip_prefix("tags:") {
            let tags = rest.split(',').map(|s| s.trim().to_string()).collect();
            return Ok(PatternSelector::Tags(tags));
        }

        // Handle wildcard:X-*
        if let Some(rest) = spec_lower.strip_prefix("wildcard:") {
            return Ok(PatternSelector::Wildcard(rest.to_string()));
        }

        // Handle regex:pattern
        if let Some(rest) = spec_lower.strip_prefix("regex:") {
            return Ok(PatternSelector::Regex(vec![rest.to_string()]));
        }

        Err(format!(
            "Invalid selector spec: {}. Expected format:\n  \
            - 'all' or 'ALL'\n  \
            - 'none' or 'NONE'\n  \
            - 'fast', 'structured', 'regex' (pattern type)\n  \
            - 'fast,structured' (multiple types)\n  \
            - 'CRITICAL' or 'CRITICAL,API_KEYS' (comma-separated tier names)\n  \
            - 'tier:critical,api_keys'\n  \
            - 'type:fast,regex' (pattern types)\n  \
            - 'patterns:aws-*,github-*'\n  \
            - 'tags:aws,github'\n  \
            - 'wildcard:aws-*'\n  \
            - 'regex:^(aws|github)'\n\n\
            Pattern types: fast, fastprefix, structured, structuredformat, regex, regexbased\n\
            Valid tier names: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS",
            spec
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

