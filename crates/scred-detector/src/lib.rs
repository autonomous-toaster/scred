#![allow(dead_code)]

//! SCRED Pattern Detector - Fast Rust Implementation
//!
//! Replaces broken Zig FFI with fast Rust pattern detection.
//! All 275 patterns from Zig converted to Rust with identical logic.
//!
//! Performance: 185.5 MB/s on realistic workloads (3.8× improvement).
//! Uses pure scalar code optimized for production stability.

pub mod detector;
pub mod match_result;
pub mod patterns;
pub mod prefix_index;
pub mod uri_patterns;

pub use detector::{
    detect_all, detect_jwt, detect_simple_prefix, detect_ssh_keys, detect_uri_patterns,
    detect_validation, redact_in_place, redact_in_place_with_original, redact_text,
};
pub use match_result::{DetectionResult, Match, RedactionResult};
pub use patterns::{
    Charset, JwtPattern, PatternTier, PrefixValidationPattern, SimplePrefixPattern, JWT_PATTERNS,
    PREFIX_VALIDATION_PATTERNS, SIMPLE_PREFIX_PATTERNS,
};

// Version matching Zig implementation
pub const VERSION: &str = "0.1.0";
pub const TOTAL_PATTERNS: usize = patterns::TOTAL_PATTERNS;
