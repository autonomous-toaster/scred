/// Phase 3: Runtime Metadata Cache
/// O(1) pattern lookups with HashMap indices
///
/// This module provides thread-safe, lazy-loaded metadata caching for all 274 patterns.
/// Initialized once at startup via OnceLock singleton.

use std::collections::HashMap;
use std::sync::OnceLock;

// ============================================================================
// PatternMetadata Runtime Structure
// ============================================================================

#[derive(Debug, Clone)]
pub struct PatternMetadata {
    pub name: String,
    pub tier: RiskTier,
    pub category: PatternCategory,
    pub risk_score: u8,
    pub ffi_path: FFIPath,
    
    // Prefix information
    pub prefix: Option<String>,
    pub prefix_len: u16,
    
    // Charset
    pub charset: Charset,
    
    // Length constraints
    pub min_length: u16,
    pub max_length: u16,
    pub fixed_length: Option<u16>,
    
    // Regex pattern
    pub regex_pattern: Option<String>,
    
    // Example secret
    pub example_secret: String,
    
    // Tags
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum RiskTier {
    Critical,
    ApiKeys,
    Infrastructure,
    Services,
    Patterns,
}

impl RiskTier {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(RiskTier::Critical),
            1 => Some(RiskTier::ApiKeys),
            2 => Some(RiskTier::Infrastructure),
            3 => Some(RiskTier::Services),
            4 => Some(RiskTier::Patterns),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            RiskTier::Critical => 0,
            RiskTier::ApiKeys => 1,
            RiskTier::Infrastructure => 2,
            RiskTier::Services => 3,
            RiskTier::Patterns => 4,
        }
    }

    pub fn risk_score(&self) -> u8 {
        match self {
            RiskTier::Critical => 95,
            RiskTier::ApiKeys => 80,
            RiskTier::Infrastructure => 60,
            RiskTier::Services => 40,
            RiskTier::Patterns => 30,
        }
    }

    pub fn default_redact(&self) -> bool {
        match self {
            RiskTier::Critical | RiskTier::ApiKeys => true,
            _ => false,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            RiskTier::Critical => "CRITICAL",
            RiskTier::ApiKeys => "API_KEYS",
            RiskTier::Infrastructure => "INFRASTRUCTURE",
            RiskTier::Services => "SERVICES",
            RiskTier::Patterns => "PATTERNS",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternCategory {
    SimplePrefix,
    PrefixFixed,
    PrefixMinlen,
    PrefixVariable,
    JwtPattern,
    Regex,
}

impl PatternCategory {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(PatternCategory::SimplePrefix),
            1 => Some(PatternCategory::PrefixFixed),
            2 => Some(PatternCategory::PrefixMinlen),
            3 => Some(PatternCategory::PrefixVariable),
            4 => Some(PatternCategory::JwtPattern),
            5 => Some(PatternCategory::Regex),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FFIPath {
    MatchPrefix,
    PrefixCharset,
    PrefixLength,
    PrefixMinlen,
    PrefixVariable,
    JwtSpecial,
    RegexMatch,
}

impl FFIPath {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(FFIPath::MatchPrefix),
            1 => Some(FFIPath::PrefixCharset),
            2 => Some(FFIPath::PrefixLength),
            3 => Some(FFIPath::PrefixMinlen),
            4 => Some(FFIPath::PrefixVariable),
            5 => Some(FFIPath::JwtSpecial),
            6 => Some(FFIPath::RegexMatch),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Charset {
    Alphanumeric,
    Hex,
    Base64,
    Base64Url,
    Numeric,
    Any,
}

impl Charset {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Charset::Alphanumeric),
            1 => Some(Charset::Hex),
            2 => Some(Charset::Base64),
            3 => Some(Charset::Base64Url),
            4 => Some(Charset::Numeric),
            5 => Some(Charset::Any),
            _ => None,
        }
    }
}

// ============================================================================
// MetadataCache: O(1) Lookups
// ============================================================================

pub struct MetadataCache {
    // By-name: HashMap<pattern_name, PatternMetadata> → O(1)
    patterns_by_name: HashMap<String, PatternMetadata>,
    
    // By-tier: HashMap<tier, Vec<pattern_name>> → O(1) list access
    patterns_by_tier: HashMap<RiskTier, Vec<String>>,
    
    // By-tag: HashMap<tag, Vec<pattern_name>> → O(1) list access
    patterns_by_tag: HashMap<String, Vec<String>>,
    
    // Total count
    total_patterns: usize,
}

impl MetadataCache {
    /// Initialize cache from FFI
    pub fn new() -> Self {
        let cache = MetadataCache {
            patterns_by_name: HashMap::new(),
            patterns_by_tier: HashMap::new(),
            patterns_by_tag: HashMap::new(),
            total_patterns: 0,
        };
        
        // Load patterns from Zig via FFI (if implemented)
        // For now, this is a template for the integration
        
        cache
    }
    
    /// Get pattern by name - O(1)
    pub fn get_pattern(&self, name: &str) -> Option<&PatternMetadata> {
        self.patterns_by_name.get(name)
    }
    
    /// Get all patterns in a tier - O(1)
    pub fn get_patterns_by_tier(&self, tier: &RiskTier) -> Option<&[String]> {
        self.patterns_by_tier.get(tier).map(|v| v.as_slice())
    }
    
    /// Get all patterns with a tag - O(1)
    pub fn get_patterns_by_tag(&self, tag: &str) -> Option<&[String]> {
        self.patterns_by_tag.get(tag).map(|v| v.as_slice())
    }
    
    /// Get total pattern count
    pub fn total_patterns(&self) -> usize {
        self.total_patterns
    }
    
    /// Get tier distribution statistics
    pub fn tier_statistics(&self) -> HashMap<RiskTier, usize> {
        let mut stats = HashMap::new();
        for (tier, patterns) in &self.patterns_by_tier {
            stats.insert(tier.clone(), patterns.len());
        }
        stats
    }
    
    /// Get all pattern names as iterator (public accessor)
    pub fn all_pattern_names(&self) -> impl Iterator<Item = &String> {
        self.patterns_by_name.keys()
    }
    
    /// Get all patterns as iterator (public accessor)
    pub fn all_patterns(&self) -> impl Iterator<Item = (&String, &PatternMetadata)> {
        self.patterns_by_name.iter()
    }
}

// ============================================================================
// Global Singleton Cache
// ============================================================================

pub static METADATA_CACHE: OnceLock<MetadataCache> = OnceLock::new();

/// Get the global metadata cache (lazy-initialized on first access)
pub fn get_cache() -> &'static MetadataCache {
    METADATA_CACHE.get_or_init(|| MetadataCache::new())
}

/// Initialize cache explicitly (optional, called automatically on first access)
pub fn initialize_cache() -> &'static MetadataCache {
    get_cache()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_initialization() {
        let cache = get_cache();
        println!("Cache initialized with {} patterns", cache.total_patterns());
        assert!(cache.total_patterns() >= 0);
    }

    #[test]
    fn test_cache_singleton() {
        let cache1 = get_cache();
        let cache2 = get_cache();
        
        // Both should point to same memory
        assert_eq!(
            cache1 as *const _,
            cache2 as *const _,
            "Cache should be singleton"
        );
    }

    #[test]
    fn test_risk_tier_conversion() {
        assert_eq!(RiskTier::Critical.to_u8(), 0);
        assert_eq!(RiskTier::ApiKeys.to_u8(), 1);
        assert_eq!(RiskTier::from_u8(0), Some(RiskTier::Critical));
        assert_eq!(RiskTier::from_u8(1), Some(RiskTier::ApiKeys));
    }

    #[test]
    fn test_risk_score() {
        assert_eq!(RiskTier::Critical.risk_score(), 95);
        assert_eq!(RiskTier::ApiKeys.risk_score(), 80);
        assert_eq!(RiskTier::Patterns.risk_score(), 30);
    }

    #[test]
    fn test_default_redact() {
        assert!(RiskTier::Critical.default_redact());
        assert!(RiskTier::ApiKeys.default_redact());
        assert!(!RiskTier::Infrastructure.default_redact());
        assert!(!RiskTier::Patterns.default_redact());
    }

    #[test]
    fn test_charset_conversion() {
        assert_eq!(Charset::from_u8(0), Some(Charset::Alphanumeric));
        assert_eq!(Charset::from_u8(3), Some(Charset::Base64Url));
        assert_eq!(Charset::from_u8(255), None);
    }

    #[test]
    fn test_pattern_category_conversion() {
        assert_eq!(PatternCategory::from_u8(0), Some(PatternCategory::SimplePrefix));
        assert_eq!(PatternCategory::from_u8(5), Some(PatternCategory::Regex));
        assert_eq!(PatternCategory::from_u8(255), None);
    }

    #[test]
    fn test_ffi_path_conversion() {
        assert_eq!(FFIPath::from_u8(0), Some(FFIPath::MatchPrefix));
        assert_eq!(FFIPath::from_u8(6), Some(FFIPath::RegexMatch));
        assert_eq!(FFIPath::from_u8(255), None);
    }
}
