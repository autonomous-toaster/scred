//! SCRED Redactor Library
//!
//! Core secret pattern redaction engine with 52 high-confidence patterns.
//!
//! # Features
//! - **52 high-confidence patterns**: AWS, GitHub, Stripe, OpenAI, etc.
//! - **Character-preserving**: Output length = input length
//! - **Streaming mode**: Bounded memory (64KB chunks), handles GB-scale files

pub mod detector;
pub mod redactor;
pub mod streaming;
pub mod pattern_selector;
pub mod metadata_cache;
pub mod frame_ring;
pub mod buffer_pool;

// ============================================================================
// PUBLIC API - PRIMARY EXPORTS
// ============================================================================

// Core detector API
pub use detector::{StreamingDetector, SecretDetectionEvent};

// Rust SIMD pattern detector (source of truth for all patterns)
pub use scred_detector;

// Legacy API (for backward compatibility with http/mitm/proxy crates)
pub use redactor::{
    RedactionEngine, RedactionConfig, RedactionResult, RedactionWarning, PatternMatch,
};

// Convenience function for simple redaction
pub fn redact_text(text: &str) -> String {
    let engine = RedactionEngine::new(RedactionConfig::default());
    let result = engine.redact(text);
    result.redacted
}

// Pattern selector for filtering patterns
pub use pattern_selector::{PatternSelector, CompositePatternSelector, PatternFilter};
pub use metadata_cache::RiskTier as PatternTier;

// NOTE: Pattern info function removed - now using Rust SIMD, not Zig FFI
// pub fn get_all_patterns() -> Vec<scred_detector::PatternInfo> { ... }

pub use streaming::{
    StreamingRedactor, StreamingConfig, StreamingStats, FrameRingRedactor,
};

pub use buffer_pool::{BufferPool, BufferPoolStats};



// ============================================================================
// Metadata Cache (removed - was duplicate definition)
// ============================================================================

pub use metadata_cache::{
    MetadataCache, PatternMetadata, RiskTier, PatternCategory, FFIPath, Charset,
    get_cache, initialize_cache, METADATA_CACHE,
};


// Stub for CLI compatibility (patterns no longer exposed via this API)
#[derive(Debug, Clone)]
pub struct PatternInfo {
    pub name: String,
    pub pattern_type: u8,
    pub prefix: String,
    pub min_len: usize,
    pub max_len: usize,
}

pub fn get_all_patterns() -> Vec<PatternInfo> {
    let mut patterns = Vec::new();
    
    // FastPrefix patterns (type 0)
    for (_idx, p) in scred_detector::SIMPLE_PREFIX_PATTERNS.iter().enumerate() {
        patterns.push(PatternInfo {
            name: p.name.to_string(),
            pattern_type: 0,
            prefix: p.prefix.to_string(),
            min_len: 0,
            max_len: 0,
        });
    }
    
    // PrefixValidation patterns (type 0, same category)
    for (_idx, p) in scred_detector::PREFIX_VALIDATION_PATTERNS.iter().enumerate() {
        patterns.push(PatternInfo {
            name: p.name.to_string(),
            pattern_type: 0,
            prefix: p.prefix.to_string(),
            min_len: p.min_len,
            max_len: p.max_len,
        });
    }
    
    // JWT patterns (type 1)
    for (_idx, p) in scred_detector::JWT_PATTERNS.iter().enumerate() {
        patterns.push(PatternInfo {
            name: p.name.to_string(),
            pattern_type: 1,
            prefix: "eyJ".to_string(),
            min_len: 0,
            max_len: 0,
        });
    }
    
    patterns
}
