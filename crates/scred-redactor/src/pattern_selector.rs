/// Phase 4: Pattern Selector
/// Flexible pattern selection with 6 modes: All, Tiers, Patterns, Tags, Wildcard, Regex

use crate::metadata_cache::{MetadataCache, PatternMetadata, RiskTier};
use std::collections::HashSet;

// ============================================================================
// NEW ARCHITECTURE: Separate Classification Dimensions (Phase 2)
// ============================================================================

/// DIMENSION 1: Severity - Actual risk if the secret is leaked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    Critical = 95,  // Signing keys, database credentials, payment keys (high impact if leaked)
    High = 85,      // AWS/GitHub tokens, cloud credentials (high value targets)
    Medium = 65,    // Generic API keys, OAuth tokens (medium impact)
    Low = 40,       // Specialty/niche services (low impact, easy to rotate)
    Generic = 30,   // Regex patterns, generic formats (lowest confidence)
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
        input
            .split(',')
            .map(|s| Self::from_str(s.trim()))
            .collect()
    }

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
    CloudProvider,      // AWS, Azure, GCP
    PaymentProcessor,   // Stripe, Square, PayPal, etc.
    CodeHost,          // GitHub, GitLab, Bitbucket, etc.
    Database,          // PostgreSQL, MongoDB, MySQL, etc.
    Messaging,         // Slack, Discord, Telegram, etc.
    Infrastructure,    // Docker, K8s, Vault, etcd, etc.
    Authentication,    // Auth0, Okta, KeyCloak, etc.
    Monitoring,        // Datadog, New Relic, Grafana, etc.
    Development,       // npm, PyPI, RubyGems, etc.
    AI,                // OpenAI, Anthropic, Huggingface, etc.
    Other,             // Everything else
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
        input
            .split(',')
            .map(|s| Self::from_str(s.trim()))
            .collect()
    }

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
    FixedPrefix,       // Starts with known prefix (e.g., AKIA for AWS)
    StructuredFormat,  // JWT, PEM, Base64-encoded format
    RegexBased,        // Generic regex pattern
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
        input
            .split(',')
            .map(|s| Self::from_str(s.trim()))
            .collect()
    }

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
    FirstParty,  // Internal company services
    ThirdParty,  // External vendor services
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

impl PatternSelector {
    /// Check if a pattern matches this selector
    pub fn matches(&self, metadata: &PatternMetadata) -> bool {
        match self {
            PatternSelector::All => true,
            PatternSelector::None => false,
            
            PatternSelector::Tiers(tiers) => {
                tiers.iter().any(|t| t == &metadata.tier)
            },
            
            PatternSelector::Type(_types) => {
                // TODO: Add pattern_type to PatternMetadata once integrated
                // For now, Type filtering happens at detector level
                true
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
        if let Some(prefix) = pattern.strip_suffix('*') {
            name.starts_with(prefix)
        } else if let Some(suffix) = pattern.strip_prefix('*') {
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
            
            PatternSelector::Type(_types) => {
                // Type filtering happens at detector level
                // For now, return all patterns
                for name in cache.all_pattern_names() {
                    matching.push(name.clone());
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
            PatternSelector::Type(types) => {
                format!("Pattern Types: {}", types.join(", "))
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
            let types: Vec<String> = rest
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect();
            return Ok(PatternSelector::Type(types));
        }
        
        // Handle comma-separated pattern types (e.g., "fast,structured")
        if !spec_lower.contains(':') && !spec_lower.contains('*') && !spec_lower.contains('^') {
            let parts: Vec<&str> = spec_lower.split(',').map(|s| s.trim()).collect();
            if parts.iter().all(|p| matches!(p, &"fast" | &"fastprefix" | &"structured" | &"structuredformat" | &"regex" | &"regexbased")) {
                let types: Vec<String> = parts.iter()
                    .map(|p| match *p {
                        "fastprefix" => "fast".to_string(),
                        "structuredformat" => "structured".to_string(),
                        "regexbased" => "regex".to_string(),
                        other => other.to_string(),
                    })
                    .collect();
                if types.iter().all(|t| matches!(t.as_str(), "fast" | "structured" | "regex")) {
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

