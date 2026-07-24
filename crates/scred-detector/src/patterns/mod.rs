//! Pattern definitions - extracted from Zig source of truth
//! All 275 patterns organized by detection type

pub mod marker;
pub mod prefix_validation;
pub mod simple_prefix;

pub use marker::{GeneralizedMarkerPattern, MultilineMarkerPattern, GENERALIZED_MARKER_PATTERNS, MULTILINE_MARKER_PATTERNS};
pub use prefix_validation::PREFIX_VALIDATION_PATTERNS;
pub use simple_prefix::SIMPLE_PREFIX_PATTERNS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternTier {
    Critical,
    Infrastructure,
    Services,
    ApiKeys,
    Patterns,
}

#[derive(Debug, Clone)]
pub struct SimplePrefixPattern {
    pub name: &'static str,
    pub prefix: &'static str,
    pub tier: PatternTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charset {
    Alphanumeric,
    Base64,
    Base64Url,
    Hex,
    Any,
}

#[derive(Debug, Clone)]
pub struct PrefixValidationPattern {
    pub name: &'static str,
    pub prefix: &'static str,
    pub tier: PatternTier,
    pub min_len: usize,
    pub max_len: usize,
    pub charset: Charset,
}

#[derive(Debug, Clone)]
pub struct JwtPattern {
    pub name: &'static str,
    pub tier: PatternTier,
}

pub const JWT_PATTERNS: &[JwtPattern] = &[JwtPattern {
    name: "jwt-generic",
    tier: PatternTier::Critical,
}];

// ============================================================================
// COUNTS
// ============================================================================

pub const REGEX_PATTERN_COUNT: usize = 18;
pub const URI_PATTERNS_COUNT: usize = 14;

pub const SIMPLE_PREFIX_COUNT: usize = 23;
pub const PREFIX_VALIDATION_COUNT: usize = 359;
pub const JWT_COUNT: usize = 1;
pub const MULTILINE_MARKER_COUNT: usize = 11;

pub const TOTAL_PATTERNS: usize = SIMPLE_PREFIX_COUNT
    + PREFIX_VALIDATION_COUNT
    + JWT_COUNT
    + MULTILINE_MARKER_COUNT
    + REGEX_PATTERN_COUNT
    + URI_PATTERNS_COUNT;
