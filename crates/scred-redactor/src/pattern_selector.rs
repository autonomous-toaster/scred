/// Phase 4: Pattern Selector
/// Flexible pattern selection with 6 modes: All, Tiers, Patterns, Tags, Wildcard, Regex

use crate::metadata_cache::{MetadataCache, PatternMetadata, RiskTier};
use std::collections::HashSet;

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

// ============================================================================
// CompositePatternSelector: Handle mixed filters (tiers + globs + exclusions)
// ============================================================================

/// Individual filter in a composite selector
#[derive(Debug, Clone, PartialEq)]
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

/// Composite pattern selector combining multiple filters
/// Supports: tiers, glob patterns, and exclusions
/// Example: "CRITICAL,mysql*,postgresql*,!test-*"
#[derive(Debug, Clone)]
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
        let inclusion_strs: Vec<String> = self.inclusions.iter().map(|f| {
            match f {
                PatternFilter::Tier(t) => format!("tier:{:?}", t),
                PatternFilter::GlobName(g) => format!("glob:{}", g),
                PatternFilter::Exclude(_) => unreachable!(),
            }
        }).collect();
        
        let mut desc = inclusion_strs.join(", ");
        
        if !self.exclusions.is_empty() {
            let exclude_strs: Vec<String> = self.exclusions.iter().map(|f| {
                if let PatternFilter::Exclude(g) = f {
                    format!("!{}", g)
                } else {
                    unreachable!()
                }
            }).collect();
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
    /// Uses efficient glob matching with * and ? support
    fn wildcard_match_name(&self, pattern: &str, name: &str) -> bool {
        let matcher = GlobMatcher::new(pattern);
        matcher.matches(name)
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
    pub fn matches_pattern(&self, _pattern: &str, tier: RiskTier) -> bool {
        // Check if this tier matches the selector
        match self {
            PatternSelector::All => true,
            PatternSelector::None => false,
            
            PatternSelector::Tiers(tiers) => {
                tiers.iter().any(|t| *t == tier)
            },
            
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


#[cfg(test)]
mod glob_tests {
    use super::*;

    #[test]
    fn test_glob_exact_match() {
        let matcher = GlobMatcher::new("mysql-password");
        assert!(matcher.matches("mysql-password"));
        assert!(!matcher.matches("mysql-user"));
    }

    #[test]
    fn test_glob_star_suffix() {
        let matcher = GlobMatcher::new("mysql*");
        assert!(matcher.matches("mysql-password"));
        assert!(matcher.matches("mysql-url"));
        assert!(matcher.matches("mysql-dsn"));
        assert!(matcher.matches("mysql")); // * matches 0 chars
        assert!(!matcher.matches("postgres-password"));
        assert!(!matcher.matches("mariadb-password"));
    }

    #[test]
    fn test_glob_star_prefix() {
        let matcher = GlobMatcher::new("*-password");
        assert!(matcher.matches("mysql-password"));
        assert!(matcher.matches("postgres-password"));
        assert!(matcher.matches("redis-password"));
        assert!(!matcher.matches("password"));
        assert!(!matcher.matches("mysql-user"));
    }

    #[test]
    fn test_glob_star_middle() {
        let matcher = GlobMatcher::new("aws*-key");
        assert!(matcher.matches("aws-key"));
        assert!(matcher.matches("aws-access-key"));
        assert!(matcher.matches("aws-secret-key"));
        assert!(!matcher.matches("aws-user"));
        assert!(!matcher.matches("aws"));
    }

    #[test]
    fn test_glob_question_single() {
        let matcher = GlobMatcher::new("aws-?");
        assert!(matcher.matches("aws-a"));
        assert!(matcher.matches("aws-k"));
        assert!(!matcher.matches("aws-ab"));
        assert!(!matcher.matches("aws-"));
        assert!(!matcher.matches("aws"));
    }

    #[test]
    fn test_glob_question_multiple() {
        let matcher = GlobMatcher::new("gh?-token");
        assert!(matcher.matches("ghp-token"));
        assert!(matcher.matches("ghu-token"));
        assert!(matcher.matches("ghs-token"));
        assert!(!matcher.matches("gh-token"));
        assert!(!matcher.matches("ghab-token"));
    }

    #[test]
    fn test_glob_combined_wildcards() {
        let matcher = GlobMatcher::new("*test*");
        assert!(matcher.matches("test"));
        assert!(matcher.matches("pre-test"));
        assert!(matcher.matches("test-post"));
        assert!(matcher.matches("pre-test-post"));
        assert!(!matcher.matches("tst"));
        assert!(!matcher.matches("tes"));
    }

    #[test]
    fn test_glob_aws_pattern() {
        let matcher = GlobMatcher::new("aws-*");
        assert!(matcher.matches("aws-akia"));
        assert!(matcher.matches("aws-access-key"));
        assert!(matcher.matches("aws-secret-key"));
        assert!(matcher.matches("aws-asia"));
        assert!(!matcher.matches("azure-key"));
        assert!(!matcher.matches("aws"));
    }

    #[test]
    fn test_glob_github_pattern() {
        let matcher = GlobMatcher::new("github-*");
        assert!(matcher.matches("github-ghp"));
        assert!(matcher.matches("github-token"));
        assert!(matcher.matches("github-pat"));
        assert!(!matcher.matches("gitlab-token"));
        assert!(!matcher.matches("github"));
    }

    #[test]
    fn test_glob_api_key_patterns() {
        let matchers = vec![
            ("mysql*", vec!["mysql-password", "mysql-url", "mysql-dsn"]),
            ("postgres*", vec!["postgresql-password", "postgresql-dsn"]),
            ("redis*", vec!["redis-password", "redis-url"]),
            ("mongodb*", vec!["mongodb-password", "mongodb-uri"]),
            ("openai*", vec!["openai-api-key", "openai-sk-proj"]),
            ("dependabot*", vec!["dependabot-token", "dependabot-secret"]),
        ];

        for (pattern, expected_matches) in matchers {
            let matcher = GlobMatcher::new(pattern);
            for name in expected_matches {
                assert!(matcher.matches(name), "Pattern {} should match {}", pattern, name);
            }
        }
    }

    #[test]
    fn test_glob_exclusion_pattern() {
        let matcher = GlobMatcher::new("test-*");
        // These should match the glob
        assert!(matcher.matches("test-secret"));
        assert!(matcher.matches("test-password"));
        assert!(matcher.matches("test-key"));
        // These should NOT match
        assert!(!matcher.matches("prod-secret"));
        assert!(!matcher.matches("staging-password"));
    }

    #[test]
    fn test_glob_performance_simple() {
        // Verify simple case is fast
        let start = std::time::Instant::now();
        let matcher = GlobMatcher::new("mysql*");
        for _ in 0..10000 {
            let _ = matcher.matches("mysql-password");
        }
        let elapsed = start.elapsed();
        // Should be very fast (<1ms for 10k matches)
        assert!(elapsed.as_millis() < 50, "Performance regression: {}ms for 10k matches", elapsed.as_millis());
    }

    #[test]
    fn test_glob_edge_cases() {
        // Empty pattern should only match empty string
        let matcher = GlobMatcher::new("");
        assert!(matcher.matches(""));
        assert!(!matcher.matches("anything"));

        // Single * should match anything
        let matcher = GlobMatcher::new("*");
        assert!(matcher.matches(""));
        assert!(matcher.matches("anything"));
        assert!(matcher.matches("mysql-password-12345"));

        // ? should match any single char
        let matcher = GlobMatcher::new("?");
        assert!(matcher.matches("a"));
        assert!(!matcher.matches(""));
        assert!(!matcher.matches("ab"));
    }
}

#[cfg(test)]
mod pattern_selector_glob_tests {
    use super::*;

    #[test]
    fn test_selector_wildcard_mode() {
        let selector = PatternSelector::Wildcard("mysql*".to_string());
        assert_eq!(selector.description(), "Wildcard: mysql*");
    }

    #[test]
    fn test_selector_from_string_wildcard() {
        let selector = PatternSelector::from_string("wildcard:mysql*").unwrap();
        assert!(matches!(selector, PatternSelector::Wildcard(_)));
    }

    #[test]
    fn test_selector_from_string_multiple_globs() {
        // Note: Currently supports "patterns:mysql*,postgres*" syntax
        let selector = PatternSelector::from_string("patterns:mysql-password,postgres-dsn").unwrap();
        assert!(matches!(selector, PatternSelector::Patterns(_)));
    }
}

#[cfg(test)]
mod composite_selector_tests {
    use super::*;

    #[test]
    fn test_single_tier_filter() {
        let selector = CompositePatternSelector::from_string("CRITICAL").unwrap();
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(!selector.matches("mysql-password", RiskTier::ApiKeys));
    }

    #[test]
    fn test_multiple_tiers() {
        let selector = CompositePatternSelector::from_string("CRITICAL,API_KEYS").unwrap();
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(selector.matches("mysql-password", RiskTier::ApiKeys));
        assert!(!selector.matches("ssh-key", RiskTier::Infrastructure));
    }

    #[test]
    fn test_glob_pattern_only() {
        let selector = CompositePatternSelector::from_string("mysql*").unwrap();
        assert!(selector.matches("mysql-password", RiskTier::Critical));
        assert!(selector.matches("mysql-url", RiskTier::Critical));
        assert!(!selector.matches("postgres-dsn", RiskTier::Critical));
    }

    #[test]
    fn test_multiple_glob_patterns() {
        let selector = CompositePatternSelector::from_string("mysql*,postgresql*,redis*").unwrap();
        assert!(selector.matches("mysql-password", RiskTier::Critical));
        assert!(selector.matches("postgresql-dsn", RiskTier::Critical));
        assert!(selector.matches("redis-password", RiskTier::Critical));
        assert!(!selector.matches("mongodb-uri", RiskTier::Critical));
    }

    #[test]
    fn test_tier_and_glob_combined() {
        let selector = CompositePatternSelector::from_string("CRITICAL,mysql*,postgres*").unwrap();
        // Matches CRITICAL tier
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        // Matches glob patterns
        assert!(selector.matches("mysql-password", RiskTier::ApiKeys));
        assert!(selector.matches("postgresql-dsn", RiskTier::ApiKeys));
        // Doesn't match anything
        assert!(!selector.matches("heroku-api-key", RiskTier::ApiKeys));
    }

    #[test]
    fn test_simple_exclusion() {
        let selector = CompositePatternSelector::from_string("CRITICAL,!test-*").unwrap();
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(!selector.matches("test-secret", RiskTier::Critical));
        assert!(!selector.matches("test-password", RiskTier::Critical));
    }

    #[test]
    fn test_exclude_syntax_variations() {
        let selector1 = CompositePatternSelector::from_string("CRITICAL,!test-*").unwrap();
        let selector2 = CompositePatternSelector::from_string("CRITICAL,exclude:test-*").unwrap();
        
        // Both should behave identically
        assert!(selector1.matches("aws-akia", RiskTier::Critical));
        assert!(selector2.matches("aws-akia", RiskTier::Critical));
        
        assert!(!selector1.matches("test-secret", RiskTier::Critical));
        assert!(!selector2.matches("test-secret", RiskTier::Critical));
    }

    #[test]
    fn test_multiple_exclusions() {
        let selector = CompositePatternSelector::from_string("CRITICAL,!test-*,!mock-*,!dummy-*").unwrap();
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(!selector.matches("test-secret", RiskTier::Critical));
        assert!(!selector.matches("mock-password", RiskTier::Critical));
        assert!(!selector.matches("dummy-key", RiskTier::Critical));
    }

    #[test]
    fn test_complex_real_world_scenario() {
        // Detect CRITICAL tier + AWS/GitHub/OpenAI patterns, excluding test patterns
        let selector = CompositePatternSelector::from_string(
            "CRITICAL,aws-*,github-*,openai-*,!test-*,!example-*"
        ).unwrap();
        
        // Should match
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(selector.matches("aws-access-key", RiskTier::Critical));
        assert!(selector.matches("github-ghp", RiskTier::Critical));
        assert!(selector.matches("openai-sk-proj", RiskTier::Critical));
        
        // Should NOT match (exclusions)
        assert!(!selector.matches("test-secret", RiskTier::Critical));
        assert!(!selector.matches("example-password", RiskTier::Critical));
        
        // Should NOT match (not included - different pattern type)
        assert!(!selector.matches("mysql-password", RiskTier::ApiKeys));
    }

    #[test]
    fn test_database_pattern_selection() {
        // Select only database patterns
        let selector = CompositePatternSelector::from_string(
            "mysql*,postgresql*,mongodb*,redis*"
        ).unwrap();
        
        assert!(selector.matches("mysql-password", RiskTier::Critical));
        assert!(selector.matches("postgresql-dsn", RiskTier::Critical));
        assert!(selector.matches("mongodb-uri", RiskTier::Critical));
        assert!(selector.matches("redis-password", RiskTier::Critical));
        
        assert!(!selector.matches("aws-akia", RiskTier::Critical));
        assert!(!selector.matches("github-ghp", RiskTier::Critical));
    }

    #[test]
    fn test_api_provider_selection() {
        // Select OpenAI, Anthropic, HuggingFace
        let selector = CompositePatternSelector::from_string(
            "openai*,anthropic*,huggingface*"
        ).unwrap();
        
        assert!(selector.matches("openai-api-key", RiskTier::Critical));
        assert!(selector.matches("openai-sk-proj", RiskTier::Critical));
        assert!(selector.matches("anthropic-api-key", RiskTier::Critical));
        assert!(selector.matches("huggingface-token", RiskTier::ApiKeys));
        
        assert!(!selector.matches("aws-akia", RiskTier::Critical));
    }

    #[test]
    fn test_tier_with_specific_glob_and_exclusion() {
        let selector = CompositePatternSelector::from_string(
            "CRITICAL,API_KEYS,mysql*,!test-*"
        ).unwrap();
        
        // CRITICAL tier
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        // API_KEYS tier
        assert!(selector.matches("heroku-api-key", RiskTier::ApiKeys));
        // Glob pattern
        assert!(selector.matches("mysql-password", RiskTier::Critical));
        // Excluded
        assert!(!selector.matches("test-password", RiskTier::Critical));
    }

    #[test]
    fn test_invalid_no_inclusions() {
        // Only exclusions should fail
        let result = CompositePatternSelector::from_string("!test-*,!mock-*");
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_filter_parsing() {
        let tier_filter = PatternFilter::from_str("CRITICAL").unwrap();
        assert!(matches!(tier_filter, PatternFilter::Tier(RiskTier::Critical)));
        
        let glob_filter = PatternFilter::from_str("mysql*").unwrap();
        assert!(matches!(glob_filter, PatternFilter::GlobName(_)));
        
        let exclude_filter1 = PatternFilter::from_str("!test-*").unwrap();
        assert!(matches!(exclude_filter1, PatternFilter::Exclude(_)));
        
        let exclude_filter2 = PatternFilter::from_str("exclude:dummy-*").unwrap();
        assert!(matches!(exclude_filter2, PatternFilter::Exclude(_)));
    }

    #[test]
    fn test_description() {
        let selector = CompositePatternSelector::from_string("CRITICAL,mysql*,!test-*").unwrap();
        let desc = selector.description();
        assert!(desc.contains("Critical")); // Debug format
        assert!(desc.contains("mysql"));
        assert!(desc.contains("test"));
    }

    #[test]
    fn test_performance_composite_matching() {
        let selector = CompositePatternSelector::from_string(
            "CRITICAL,API_KEYS,mysql*,postgresql*,redis*,mongodb*,!test-*,!mock-*"
        ).unwrap();
        
        // Verify it's fast (should be <1ms for 1000 matches)
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = selector.matches("mysql-password", RiskTier::Critical);
            let _ = selector.matches("aws-akia", RiskTier::Critical);
            let _ = selector.matches("test-secret", RiskTier::Critical);
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 10, "Performance regression: {}ms for 3000 matches", elapsed.as_millis());
    }
}
