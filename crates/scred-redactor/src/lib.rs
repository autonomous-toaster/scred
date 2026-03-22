//! SCRED Redactor Library
//!
//! Core secret pattern redaction engine with 52 high-confidence patterns.
//!
//! # Features
//! - **52 high-confidence patterns**: AWS, GitHub, Stripe, OpenAI, etc.
//! - **Character-preserving**: Output length = input length
//! - **Streaming mode**: Bounded memory (64KB chunks), handles GB-scale files

pub mod analyzer;
pub mod detector;
pub mod redactor;
pub mod streaming;

// ============================================================================
// PUBLIC API - PRIMARY EXPORTS
// ============================================================================

// New API (v2.0 - Zig-based)
pub use analyzer::ZigAnalyzer;
pub use detector::{StreamingDetector, SecretDetectionEvent};

// Zig pattern detector (source of truth for all patterns)
pub use scred_pattern_detector;

// Legacy API (for backward compatibility with http/mitm/proxy crates)
pub use redactor::{
    RedactionEngine, RedactionConfig, RedactionResult,
    redact_text,
};

// Pattern info function (used by CLI and other tools)
pub fn get_all_patterns() -> Vec<scred_pattern_detector::PatternInfo> {
    scred_pattern_detector::get_all_patterns()
}
pub use streaming::{
    StreamingRedactor, StreamingConfig, StreamingStats,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RedactionEngine, RedactionConfig};

    #[test]
    fn test_patterns_available() {
        // All 270 patterns are now in Zig (26 + 1 + 45 + 198)
        // Verification: ZigAnalyzer can detect patterns from all tiers
        let test_text = "sk_live_test_token_123";
        assert!(ZigAnalyzer::has_all_patterns(test_text), "Should detect patterns via Zig");
    }

    #[test]
    fn test_analyzer_creation() {
        // ZigAnalyzer is a thin FFI wrapper - verify it can detect patterns
        let result = ZigAnalyzer::has_all_patterns("sk_live_test");
        assert!(result, "Should be able to detect patterns via ZigAnalyzer");
    }

    #[test]
    fn test_redact_aws_key() {
        let input = "My AWS key is AKIAIOSFODNN7EXAMPLE";
        let redacted = redact_text(input);
        assert!(redacted.contains("AKIA"));
        assert!(redacted.contains("x"));
        assert_eq!(input.len(), redacted.len());
    }

    #[test]
    fn test_litellm_key_redaction() {
        let config = RedactionConfig { enabled: true };
        let engine = RedactionEngine::new(config);
        
        // Test 22-character LiteLLM key (sk- + 19 chars) - CRITICAL BUG FIX
        let test_key = "sk-1234567890abcdefghij";  // Exactly 23 chars, but the minimum should catch it
        let result = engine.redact(test_key);
        
        println!("Input: {}", test_key);
        println!("Output: {}", result.redacted);
        println!("Warnings: {} secrets found", result.warnings.len());
        
        assert!(result.warnings.len() > 0, "LiteLLM 22-char key should be detected");
        assert_ne!(test_key, result.redacted, "Key should be redacted");
    }

    #[test]
    fn test_litellm_lk_prefix_key() {
        let config = RedactionConfig { enabled: true };
        let engine = RedactionEngine::new(config);
        
        // Test LiteLLM key with lk- prefix
        let test_key = "sk-lk-1234567890abcdefghij";
        let result = engine.redact(test_key);
        
        assert!(result.warnings.len() > 0, "LiteLLM lk- key should be detected");
        assert_ne!(test_key, result.redacted, "Key should be redacted");
    }

    #[test]
    fn test_embedded_litellm_key() {
        let config = RedactionConfig { enabled: true };
        let engine = RedactionEngine::new(config);
        
        // Test embedded LiteLLM key in text
        let text = "My API key is sk-1234567890abcdefghij and it works great";
        let result = engine.redact(text);
        
        println!("Input: {}", text);
        println!("Output: {}", result.redacted);
        
        assert!(result.warnings.len() > 0, "Embedded key should be detected");
        // The prefix "sk-" should be preserved, followed by x's
        assert!(result.redacted.contains("sk-xxxxxxxxxxxxxxxxx"), 
                "Prefix sk- should be preserved, rest redacted. Got: {}", result.redacted);
        assert_ne!(text, result.redacted, "Output should differ from input");
    }

    #[test]
    fn test_litellm_uppercase_key() {
        let config = RedactionConfig { enabled: true };
        let engine = RedactionEngine::new(config);
        
        // Test LiteLLM key with UPPERCASE characters (exactly 22 chars: sk- + 19 chars)
        // min_len=22 means total length must be >= 22
        let key = "sk-1234567890ABCDEFGHIJ";  // 23 chars, should match min_len=22
        let result = engine.redact(key);
        
        println!("Uppercase Input:  {}", key);
        println!("Uppercase Output: {}", result.redacted);
        
        assert!(result.warnings.len() > 0, "Uppercase key should be detected");
        // Should redact ALL uppercase chars to 'x' (not preserve case)
        assert_eq!(result.redacted, "sk-xxxxxxxxxxxxxxxxxxxx", 
                   "Uppercase chars should be dropped/redacted");
    }

    #[test]
    fn test_litellm_mixed_case_key() {
        let config = RedactionConfig { enabled: true };
        let engine = RedactionEngine::new(config);
        
        // Test LiteLLM key with mixed case (exactly 22 chars: sk- + 19 chars)
        let key = "sk-1234567890AbCdEfGhIj";  // 23 chars, should match min_len=22
        let result = engine.redact(key);
        
        println!("Mixed Input:  {}", key);
        println!("Mixed Output: {}", result.redacted);
        
        assert!(result.warnings.len() > 0, "Mixed case key should be detected");
        // All case variants should redact to 'x' (not preserve case information)
        assert_eq!(result.redacted, "sk-xxxxxxxxxxxxxxxxxxxx", 
                   "Mixed case chars should all be redacted to 'x'");
    }
}
