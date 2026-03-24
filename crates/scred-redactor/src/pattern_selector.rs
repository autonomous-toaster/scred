/// Phase 4: Pattern Selector
/// Flexible pattern selection with 6 modes: All, Tiers, Patterns, Tags, Wildcard, Regex

use crate::metadata_cache::{MetadataCache, PatternMetadata, RiskTier};
use std::collections::HashSet;

// ============================================================================
// PatternSelector Enum
// ============================================================================

#[derive(Debug, Clone)]
pub enum PatternSelector {
    /// All 274 patterns
    All,
    
    /// Specific tiers (e.g., [Critical, ApiKeys])
    Tiers(Vec<RiskTier>),
    
    /// Exact pattern names
    Patterns(Vec<String>),
    
    /// By tags (exact match)
    Tags(Vec<String>),
    
    /// Wildcard matching (e.g., "aws-*", "github-*")
    Wildcard(String),
    
    /// Regex pattern matching
    Regex(String),
}

impl PatternSelector {
    /// Check if a pattern matches this selector
    pub fn matches(&self, metadata: &PatternMetadata) -> bool {
        match self {
            PatternSelector::All => true,
            
            PatternSelector::Tiers(tiers) => {
                tiers.iter().any(|t| t == &metadata.tier)
            },
            
            PatternSelector::Patterns(names) => {
                names.iter().any(|n| n == &metadata.name)
            },
            
            PatternSelector::Tags(tags) => {
                tags.iter().any(|tag| {
                    if tag.ends_with('*') {
                        // Prefix match
                        let prefix = &tag[..tag.len()-1];
                        metadata.tags.iter().any(|t| t.starts_with(prefix))
                    } else {
                        // Exact match
                        metadata.tags.contains(tag)
                    }
                })
            },
            
            PatternSelector::Wildcard(pattern) => {
                self.wildcard_match_name(pattern, &metadata.name)
            },
            
            PatternSelector::Regex(regex_pattern) => {
                if let Ok(re) = regex::Regex::new(regex_pattern) {
                    re.is_match(&metadata.name)
                } else {
                    false
                }
            },
        }
    }
    
    /// Wildcard matching: "aws-*" matches "aws-access-key", etc.
    fn wildcard_match_name(&self, pattern: &str, name: &str) -> bool {
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len()-1];
            name.starts_with(prefix)
        } else if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            name.ends_with(suffix)
        } else if pattern.contains('*') {
            // Simple * replacement pattern
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                name.starts_with(parts[0]) && name.ends_with(parts[1])
            } else {
                false
            }
        } else {
            name == pattern
        }
    }
    
    /// Get all matching pattern names from cache
    pub fn get_matching_patterns(
        &self,
        cache: &MetadataCache,
    ) -> Vec<String> {
        let mut matching = Vec::new();
        
        match self {
            PatternSelector::All => {
                // Get all pattern names
                for name in cache.patterns_by_name.keys() {
                    matching.push(name.clone());
                }
            },
            
            PatternSelector::Tiers(tiers) => {
                // Get patterns for each tier
                for tier in tiers {
                    if let Some(patterns) = cache.get_patterns_by_tier(tier) {
                        matching.extend_from_slice(patterns);
                    }
                }
            },
            
            PatternSelector::Patterns(names) => {
                // Filter to only specified names
                for name in names {
                    if cache.get_pattern(name).is_some() {
                        matching.push(name.clone());
                    }
                }
            },
            
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
            },
            
            PatternSelector::Wildcard(pattern) => {
                // Find all patterns matching wildcard
                for (name, _) in &cache.patterns_by_name {
                    if self.wildcard_match_name(pattern, name) {
                        matching.push(name.clone());
                    }
                }
            },
            
            PatternSelector::Regex(regex_pattern) => {
                // Find all patterns matching regex
                if let Ok(re) = regex::Regex::new(regex_pattern) {
                    for (name, _) in &cache.patterns_by_name {
                        if re.is_match(name) {
                            matching.push(name.clone());
                        }
                    }
                }
            },
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
// Configuration Parser
// ============================================================================

impl PatternSelector {
    /// Parse selector from string format
    /// Examples:
    ///   "all"
    ///   "tier:critical,api_keys"
    ///   "patterns:aws-access-key,github-token"
    ///   "tags:aws,github"
    ///   "wildcard:aws-*"
    ///   "regex:^(aws|github)"
    pub fn from_string(spec: &str) -> Result<Self, String> {
        if spec == "all" {
            return Ok(PatternSelector::All);
        }
        
        if let Some(rest) = spec.strip_prefix("tier:") {
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
                .collect();
            return Ok(PatternSelector::Tiers(tiers));
        }
        
        if let Some(rest) = spec.strip_prefix("patterns:") {
            let patterns = rest
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            return Ok(PatternSelector::Patterns(patterns));
        }
        
        if let Some(rest) = spec.strip_prefix("tags:") {
            let tags = rest
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            return Ok(PatternSelector::Tags(tags));
        }
        
        if let Some(rest) = spec.strip_prefix("wildcard:") {
            return Ok(PatternSelector::Wildcard(rest.to_string()));
        }
        
        if let Some(rest) = spec.strip_prefix("regex:") {
            return Ok(PatternSelector::Regex(rest.to_string()));
        }
        
        Err(format!(
            "Invalid selector spec: {}. Expected format: 'all', 'tier:X,Y', 'patterns:X,Y', 'tags:X,Y', 'wildcard:X-*', or 'regex:pattern'",
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
    fn test_wildcard_matching() {
        let selector = PatternSelector::Wildcard("aws-*".to_string());
        
        assert!(selector.wildcard_match_name("aws-*", "aws-access-key"));
        assert!(selector.wildcard_match_name("aws-*", "aws-secret-key"));
        assert!(!selector.wildcard_match_name("aws-*", "github-token"));
    }

    #[test]
    fn test_suffix_wildcard() {
        let selector = PatternSelector::Wildcard("*-token".to_string());
        
        assert!(selector.wildcard_match_name("*-token", "github-token"));
        assert!(selector.wildcard_match_name("*-token", "aws-token"));
        assert!(!selector.wildcard_match_name("*-token", "github-secret-key"));
    }

    #[test]
    fn test_selector_parsing() {
        assert!(PatternSelector::from_string("all").is_ok());
        assert!(PatternSelector::from_string("tier:critical,api_keys").is_ok());
        assert!(PatternSelector::from_string("patterns:aws-access-key").is_ok());
        assert!(PatternSelector::from_string("tags:aws").is_ok());
        assert!(PatternSelector::from_string("wildcard:aws-*").is_ok());
        assert!(PatternSelector::from_string("regex:^(aws|github)").is_ok());
        assert!(PatternSelector::from_string("invalid").is_err());
    }

    #[test]
    fn test_tier_selector_parsing() {
        match PatternSelector::from_string("tier:critical,api_keys") {
            Ok(PatternSelector::Tiers(tiers)) => {
                assert_eq!(tiers.len(), 2);
                assert!(tiers.contains(&RiskTier::Critical));
                assert!(tiers.contains(&RiskTier::ApiKeys));
            },
            _ => panic!("Expected Tiers selector"),
        }
    }
}
