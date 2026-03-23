/// Pattern selection system for flexible pattern tier-based detection and redaction
///
/// Supports multiple selection modes:
/// - Tier-based: Select by risk category (CRITICAL, API_KEYS, INFRASTRUCTURE, etc.)
/// - Wildcards: Pattern glob matching (aws-*, github-*)
/// - Regex: Complex regex matching (regex:^(aws|github))
/// - Whitelist/Blacklist: Include/exclude specific patterns

use std::collections::HashSet;

/// Five risk-based pattern tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternTier {
    Critical,       // AWS, GitHub, Stripe, Database (24 patterns)
    ApiKeys,        // Third-party API keys (60+ patterns)
    Infrastructure, // K8s, Docker, Vault, Grafana (40+ patterns)
    Services,       // Specialty services (100+ patterns)
    Patterns,       // Generic regex (50+ patterns)
}

impl PatternTier {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::ApiKeys => "API_KEYS",
            Self::Infrastructure => "INFRASTRUCTURE",
            Self::Services => "SERVICES",
            Self::Patterns => "PATTERNS",
        }
    }

    pub fn risk_score(&self) -> u8 {
        match self {
            Self::Critical => 95,
            Self::ApiKeys => 80,
            Self::Infrastructure => 60,
            Self::Services => 40,
            Self::Patterns => 30,
        }
    }

    pub fn should_redact_by_default(&self) -> bool {
        matches!(self, Self::Critical | Self::ApiKeys)
    }

    /// Parse comma-separated tier names
    /// Example: "CRITICAL,API_KEYS,INFRASTRUCTURE" -> [Critical, ApiKeys, Infrastructure]
    pub fn parse_list(input: &str) -> Result<Vec<Self>, String> {
        input
            .split(',')
            .map(|s| Self::from_str(s.trim()))
            .collect()
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "CRITICAL" => Ok(Self::Critical),
            "API_KEYS" | "API-KEYS" => Ok(Self::ApiKeys),
            "INFRASTRUCTURE" | "INFRA" => Ok(Self::Infrastructure),
            "SERVICES" => Ok(Self::Services),
            "PATTERNS" | "GENERIC" => Ok(Self::Patterns),
            _ => Err(format!("Unknown tier: {}", s)),
        }
    }
}

/// Flexible pattern selection modes
#[derive(Debug, Clone)]
pub enum PatternSelector {
    /// Select specific tiers (e.g., CRITICAL, API_KEYS, INFRASTRUCTURE)
    Tier(Vec<PatternTier>),

    /// Wildcard patterns (e.g., "aws-*", "github-*")
    Wildcard(Vec<String>),

    /// Regex patterns (e.g., "regex:^(aws|github)")
    Regex(Vec<String>),

    /// Whitelist: only these specific patterns
    Whitelist(HashSet<String>),

    /// Blacklist: all patterns except these
    Blacklist(HashSet<String>),

    /// All 274 patterns
    All,

    /// No patterns
    None,
}

impl Default for PatternSelector {
    fn default() -> Self {
        Self::default_detect()
    }
}

impl PatternSelector {
    /// Create default selector: CRITICAL + API_KEYS + INFRASTRUCTURE
    pub fn default_detect() -> Self {
        Self::Tier(vec![
            PatternTier::Critical,
            PatternTier::ApiKeys,
            PatternTier::Infrastructure,
        ])
    }

    /// Create default redact selector: CRITICAL + API_KEYS
    pub fn default_redact() -> Self {
        // Redact CRITICAL, API_KEYS, and generic PATTERNS by default
        // Only exclude Infrastructure and Services tiers (lower priority)
        Self::Tier(vec![PatternTier::Critical, PatternTier::ApiKeys, PatternTier::Patterns])
    }

    /// Parse selector from string
    /// Format: "CRITICAL", "CRITICAL,API_KEYS", "aws-*", "regex:^aws", "all", "none"
    pub fn from_str(input: &str) -> Result<Self, String> {
        let input = input.trim();

        match input.to_lowercase().as_str() {
            "all" => Ok(Self::All),
            "none" => Ok(Self::None),
            _ if input.starts_with("regex:") => {
                let regex_str = input[6..].to_string();
                Ok(Self::Regex(vec![regex_str]))
            }
            _ if input.contains(',') => {
                // Could be tiers or patterns
                // Try parsing as tiers first
                if let Ok(tiers) = PatternTier::parse_list(input) {
                    return Ok(Self::Tier(tiers));
                }

                // Fall back to wildcard patterns
                let patterns: Vec<String> =
                    input.split(',').map(|s| s.trim().to_string()).collect();
                Ok(Self::Wildcard(patterns))
            }
            _ if input.contains('-') && !input.contains('*') => {
                // Single pattern
                Ok(Self::Wildcard(vec![input.to_string()]))
            }
            _ if input.contains('*') => {
                // Wildcard pattern
                Ok(Self::Wildcard(vec![input.to_string()]))
            }
            _ => {
                // Try as single tier
                match PatternTier::from_str(input) {
                    Ok(tier) => Ok(Self::Tier(vec![tier])),
                    Err(_) => {
                        // Fall back to pattern name
                        Ok(Self::Whitelist({
                            let mut set = HashSet::new();
                            set.insert(input.to_string());
                            set
                        }))
                    }
                }
            }
        }
    }

    /// Check if a pattern matches this selector
    /// Requires pattern metadata (name and tier)
    pub fn matches_pattern(&self, pattern_name: &str, pattern_tier: PatternTier) -> bool {
        match self {
            Self::Tier(tiers) => tiers.contains(&pattern_tier),
            Self::All => true,
            Self::None => false,
            Self::Whitelist(patterns) => patterns.contains(pattern_name),
            Self::Blacklist(patterns) => !patterns.contains(pattern_name),
            Self::Wildcard(patterns) => {
                patterns.iter().any(|p| self.wildcard_match(pattern_name, p))
            }
            Self::Regex(patterns) => {
                // Implement actual regex matching
                use regex::Regex;
                
                patterns.iter().any(|p| {
                    match Regex::new(p) {
                        Ok(regex) => regex.is_match(pattern_name),
                        Err(e) => {
                            tracing::warn!("Invalid regex pattern '{}': {}", p, e);
                            false
                        }
                    }
                })
            }
        }
    }

    /// Wildcard pattern matching (e.g., "aws-*" matches "aws-akia")
    fn wildcard_match(&self, pattern_name: &str, wildcard: &str) -> bool {
        if wildcard == "*" {
            return true;
        }

        if let Some(star_pos) = wildcard.find('*') {
            let prefix = &wildcard[..star_pos];
            let suffix = &wildcard[star_pos + 1..];

            let lower_name = pattern_name.to_lowercase();
            let lower_prefix = prefix.to_lowercase();
            let lower_suffix = suffix.to_lowercase();

            lower_name.starts_with(&lower_prefix) && lower_name.ends_with(&lower_suffix)
        } else {
            pattern_name.to_lowercase() == wildcard.to_lowercase()
        }
    }

    /// Get description of what this selector matches
    pub fn description(&self) -> String {
        match self {
            Self::Tier(tiers) => {
                let tier_names: Vec<&str> = tiers.iter().map(|t| t.name()).collect();
                format!("Tiers: {}", tier_names.join(", "))
            }
            Self::All => "All patterns".to_string(),
            Self::None => "No patterns".to_string(),
            Self::Whitelist(patterns) => {
                format!("Whitelist: {}", patterns.len())
            }
            Self::Blacklist(patterns) => {
                format!("Blacklist: {} excluded", patterns.len())
            }
            Self::Wildcard(patterns) => {
                format!("Wildcards: {}", patterns.join(", "))
            }
            Self::Regex(patterns) => {
                format!("Regex: {}", patterns.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_names() {
        assert_eq!(PatternTier::Critical.name(), "CRITICAL");
        assert_eq!(PatternTier::ApiKeys.name(), "API_KEYS");
        assert_eq!(PatternTier::Infrastructure.name(), "INFRASTRUCTURE");
        assert_eq!(PatternTier::Services.name(), "SERVICES");
        assert_eq!(PatternTier::Patterns.name(), "PATTERNS");
    }

    #[test]
    fn test_tier_risk_scores() {
        assert_eq!(PatternTier::Critical.risk_score(), 95);
        assert_eq!(PatternTier::ApiKeys.risk_score(), 80);
        assert_eq!(PatternTier::Infrastructure.risk_score(), 60);
        assert_eq!(PatternTier::Services.risk_score(), 40);
        assert_eq!(PatternTier::Patterns.risk_score(), 30);
    }

    #[test]
    fn test_default_redact() {
        assert!(PatternTier::Critical.should_redact_by_default());
        assert!(PatternTier::ApiKeys.should_redact_by_default());
        assert!(!PatternTier::Infrastructure.should_redact_by_default());
        assert!(!PatternTier::Services.should_redact_by_default());
        assert!(!PatternTier::Patterns.should_redact_by_default());
    }

    #[test]
    fn test_parse_tier_list() {
        let tiers = PatternTier::parse_list("CRITICAL,API_KEYS,INFRASTRUCTURE").unwrap();
        assert_eq!(tiers.len(), 3);
        assert_eq!(tiers[0], PatternTier::Critical);
        assert_eq!(tiers[1], PatternTier::ApiKeys);
        assert_eq!(tiers[2], PatternTier::Infrastructure);
    }

    #[test]
    fn test_selector_from_str_all() {
        let sel = PatternSelector::from_str("all").unwrap();
        assert!(matches!(sel, PatternSelector::All));
    }

    #[test]
    fn test_selector_from_str_tier() {
        let sel = PatternSelector::from_str("CRITICAL").unwrap();
        assert!(matches!(sel, PatternSelector::Tier(_)));
    }

    #[test]
    fn test_selector_from_str_multiple_tiers() {
        let sel = PatternSelector::from_str("CRITICAL,API_KEYS").unwrap();
        assert!(matches!(sel, PatternSelector::Tier(ref tiers) if tiers.len() == 2));
    }

    #[test]
    fn test_selector_from_str_wildcard() {
        let sel = PatternSelector::from_str("aws-*").unwrap();
        assert!(matches!(sel, PatternSelector::Wildcard(_)));
    }

    #[test]
    fn test_wildcard_match() {
        let sel = PatternSelector::Wildcard(vec!["aws-*".to_string()]);
        assert!(sel.matches_pattern("aws-akia", PatternTier::Critical));
        assert!(sel.matches_pattern("aws-secret-access-key", PatternTier::Critical));
        assert!(!sel.matches_pattern("github-token", PatternTier::Critical));
    }

    #[test]
    fn test_tier_selector() {
        let sel = PatternSelector::Tier(vec![PatternTier::Critical, PatternTier::ApiKeys]);
        assert!(sel.matches_pattern("aws-akia", PatternTier::Critical));
        assert!(sel.matches_pattern("openai", PatternTier::ApiKeys));
        assert!(!sel.matches_pattern("k8s-token", PatternTier::Infrastructure));
    }
}
