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
        match self {
            PatternSelector::All => Self::collect_all_patterns(cache),
            PatternSelector::None => Vec::new(),
            PatternSelector::Tiers(tiers) => Self::collect_tier_patterns(cache, tiers),
            PatternSelector::Patterns(names) => Self::collect_named_patterns(cache, names),
            PatternSelector::Tags(tags) => Self::collect_tagged_patterns(cache, tags),
            PatternSelector::Wildcard(pattern) => self.collect_wildcard_patterns(cache, pattern),
            PatternSelector::Regex(_) => Vec::new(),
            PatternSelector::Type(_) => Self::collect_all_patterns(cache),
        }
    }

    /// Collect all pattern names from cache
    fn collect_all_patterns(cache: &MetadataCache) -> Vec<String> {
        cache.all_pattern_names().cloned().collect()
    }

    /// Collect patterns matching specified tiers
    fn collect_tier_patterns(cache: &MetadataCache, tiers: &[RiskTier]) -> Vec<String> {
        let mut matching = Vec::new();
        for tier in tiers {
            if let Some(patterns) = cache.get_patterns_by_tier(tier) {
                matching.extend_from_slice(patterns);
            }
        }
        matching
    }

    /// Collect patterns matching specified names
    fn collect_named_patterns(cache: &MetadataCache, names: &[String]) -> Vec<String> {
        let mut matching = Vec::new();
        for name in names {
            if cache.get_pattern(name).is_some() {
                matching.push(name.to_string());
            }
        }
        matching
    }

    /// Collect patterns matching specified tags (deduplicated)
    fn collect_tagged_patterns(cache: &MetadataCache, tags: &[String]) -> Vec<String> {
        let mut matching = Vec::new();
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
        matching
    }

    /// Collect patterns matching a wildcard pattern
    fn collect_wildcard_patterns(&self, cache: &MetadataCache, pattern: &str) -> Vec<String> {
        let mut matching = Vec::new();
        for (name, _) in cache.all_patterns() {
            if self.wildcard_match_name(pattern, name) {
                matching.push(name.clone());
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
    #[allow(clippy::should_implement_trait)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_selector_all() {
        let selector = PatternSelector::All;
        assert!(selector.matches_pattern("any-pattern", RiskTier::Critical));
        assert_eq!(selector.description(), "All patterns");
    }

    #[test]
    fn test_pattern_selector_none() {
        let selector = PatternSelector::None;
        assert!(!selector.matches_pattern("any-pattern", RiskTier::Critical));
        assert_eq!(selector.description(), "No patterns");
    }

    #[test]
    fn test_pattern_selector_tiers() {
        let selector = PatternSelector::Tiers(vec![RiskTier::Critical]);
        assert!(selector.matches_pattern("any", RiskTier::Critical));
        assert!(!selector.matches_pattern("any", RiskTier::ApiKeys));
    }

    #[test]
    fn test_pattern_selector_patterns() {
        let selector = PatternSelector::Patterns(vec!["jwt".to_string()]);
        // matches_pattern returns true for non-Tier selectors (conservative)
        assert!(selector.matches_pattern("any", RiskTier::Critical));
        assert_eq!(selector.description(), "Patterns: 1");
    }

    #[test]
    fn test_pattern_selector_tags() {
        let selector = PatternSelector::Tags(vec!["auth".to_string()]);
        assert!(selector.matches_pattern("any", RiskTier::Critical));
        assert_eq!(selector.description(), "Tags: auth");
    }

    #[test]
    fn test_pattern_selector_wildcard() {
        let selector = PatternSelector::Wildcard("*.example.com".to_string());
        assert!(selector.matches_pattern("any", RiskTier::Critical));
        assert_eq!(selector.description(), "Wildcard: *.example.com");
    }

    #[test]
    fn test_pattern_selector_regex() {
        let selector = PatternSelector::Regex(vec!["^jwt_.*".to_string()]);
        assert!(selector.matches_pattern("any", RiskTier::Critical));
        assert_eq!(selector.description(), "Regex: ^jwt_.*");
    }

    #[test]
    fn test_pattern_selector_type() {
        let selector = PatternSelector::Type(vec!["credential".to_string()]);
        assert!(selector.matches_pattern("any", RiskTier::Critical));
        assert_eq!(selector.description(), "Pattern Types: credential");
    }

    #[test]
    fn test_pattern_selector_from_string_all() {
        let selector = PatternSelector::from_string("all").unwrap();
        assert!(matches!(selector, PatternSelector::All));
    }

    #[test]
    fn test_pattern_selector_from_string_none() {
        let selector = PatternSelector::from_string("none").unwrap();
        assert!(matches!(selector, PatternSelector::None));
    }

    #[test]
    fn test_pattern_selector_from_string_tiers() {
        let selector = PatternSelector::from_string("tier:critical").unwrap();
        assert!(matches!(selector, PatternSelector::Tiers(_)));
    }

    #[test]
    fn test_pattern_selector_from_string_patterns() {
        let selector = PatternSelector::from_string("patterns:jwt,api-key").unwrap();
        assert!(matches!(selector, PatternSelector::Patterns(_)));
    }

    #[test]
    fn test_pattern_selector_from_string_tags() {
        let selector = PatternSelector::from_string("tags:auth").unwrap();
        assert!(matches!(selector, PatternSelector::Tags(_)));
    }

    #[test]
    fn test_pattern_selector_from_string_wildcard() {
        let selector = PatternSelector::from_string("wildcard:*.example.com").unwrap();
        assert!(matches!(selector, PatternSelector::Wildcard(_)));
    }

    #[test]
    fn test_pattern_selector_from_string_regex() {
        let selector = PatternSelector::from_string("regex:^jwt").unwrap();
        assert!(matches!(selector, PatternSelector::Regex(_)));
    }

    #[test]
    fn test_pattern_selector_from_string_type() {
        let selector = PatternSelector::from_string("type:credential").unwrap();
        assert!(matches!(selector, PatternSelector::Type(_)));
    }

    #[test]
    fn test_pattern_selector_from_string_invalid() {
        assert!(PatternSelector::from_string("").is_err());
    }

    #[test]
    fn test_pattern_selector_clone() {
        let selector = PatternSelector::All;
        let cloned = selector.clone();
        assert!(matches!(cloned, PatternSelector::All));
    }

    #[test]
    fn test_pattern_selector_debug() {
        let selector = PatternSelector::All;
        let debug = format!("{:?}", selector);
        assert!(!debug.is_empty());
    }

    #[test]
    fn test_count_matches() {
        let selector = PatternSelector::All;
        let cache = MetadataCache::new();
        assert_eq!(selector.count_matches(&cache), 0);
    }

    #[test]
    fn test_get_tier_distribution() {
        let selector = PatternSelector::All;
        let cache = MetadataCache::new();
        let dist = selector.get_tier_distribution(&cache);
        assert!(dist.is_empty());
    }
}
