#![allow(dead_code)]
#![cfg_attr(feature = "simd-accel", feature(portable_simd))]

//! SCRED Pattern Detector - Pure Rust SIMD Implementation
//! 
//! Replaces broken Zig FFI with fast Rust pattern detection.
//! All 275 patterns from Zig converted to Rust with identical logic.

pub mod patterns;
pub mod match_result;
pub mod simd_core;
pub mod simd_charset;
pub mod detector;
pub mod prefix_index;
pub mod uri_patterns;

pub use match_result::{Match, DetectionResult, RedactionResult};
pub use patterns::{
    SimplePrefixPattern, PrefixValidationPattern, JwtPattern,
    SIMPLE_PREFIX_PATTERNS, PREFIX_VALIDATION_PATTERNS, JWT_PATTERNS,
    PatternTier, Charset,
};
pub use detector::{detect_simple_prefix, detect_validation, detect_jwt, detect_all, detect_ssh_keys, detect_uri_patterns, redact_text, redact_in_place};

// Version matching Zig implementation
pub const VERSION: &str = "0.1.0";
pub const TOTAL_PATTERNS: usize = patterns::TOTAL_PATTERNS;


