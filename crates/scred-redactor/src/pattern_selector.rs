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
    
    /// No patterns (don't detect/redact anything)
    None,
    
    /// Specific tiers (e.g., [Critical, ApiKeys])
    Tiers(Vec<RiskTier>),
    
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

impl PatternSelector {
    /// Check if a pattern matches this selector
    pub fn matches(&self, metadata: &PatternMetadata) -> bool {
        match self {
            PatternSelector::All => true,
            PatternSelector::None => false,
            
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
            
            PatternSelector::Regex(_regex_patterns) => {
                // Regex matching: for now, simplified
                false
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
                for name in cache.all_pattern_names() {
                    matching.push(name.clone());
                }
            },
            
            PatternSelector::None => {
                // No patterns match
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
                for (name, _) in cache.all_patterns() {
                    if self.wildcard_match_name(pattern, name) {
                        matching.push(name.clone());
                    }
                }
            },
            
            PatternSelector::Regex(_regex_patterns) => {
                // Regex patterns - simplified for now
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
    pub fn matches_pattern(&self, _pattern: &str, _tier: RiskTier) -> bool {
        // Simplified: always true for now
        // Real implementation would check actual pattern matching
        true
    }
    
    /// Get description of selector
    pub fn description(&self) -> String {
        match self {
            PatternSelector::All => "All patterns".to_string(),
            PatternSelector::None => "No patterns".to_string(),
            PatternSelector::Tiers(tiers) => {
                let tier_names: Vec<String> = tiers.iter().map(|t| format!("{:?}", t)).collect();
                format!("Tiers: {}", tier_names.join(", "))
            },
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
            let patterns = rest
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            return Ok(PatternSelector::Patterns(patterns));
        }
        
        // Handle tags:X,Y
        if let Some(rest) = spec_lower.strip_prefix("tags:") {
            let tags = rest
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
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
            - 'CRITICAL' or 'CRITICAL,API_KEYS' (comma-separated tier names)\n  \
            - 'tier:critical,api_keys'\n  \
            - 'patterns:aws-*,github-*'\n  \
            - 'tags:aws,github'\n  \
            - 'wildcard:aws-*'\n  \
            - 'regex:^(aws|github)'\n\n\
            Valid tier names: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS\n\
            Valid patterns: aws-*, github-*, sk-*, etc.\n\
            Valid regex: regex:^sk-",
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
