/// Information about a single pattern match (for selective filtering)
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// Byte position of the match in the ORIGINAL input
    pub position: usize,
    /// Pattern type/name (e.g., "aws-access-key")
    pub pattern_type: String,
    /// Original unredacted text
    pub original_text: String,
    /// Redacted replacement text (same length as original)
    pub redacted_text: String,
    /// Length of the match
    pub match_len: usize,
}

#[derive(Debug, Clone)]
pub struct RedactionWarning {
    pub pattern_type: String,
    pub count: usize,
}

/// Result of redaction with full metadata about matches
#[derive(Debug, Clone)]
pub struct RedactionResult {
    /// The redacted text (all patterns replaced)
    pub redacted: String,
    /// Detailed information about each match (enables selective un-redaction in streaming)
    pub matches: Vec<PatternMatch>,
    /// Legacy warnings (for backward compatibility)
    pub warnings: Vec<RedactionWarning>,
}

#[derive(Debug, Clone)]
pub struct RedactionConfig {
    pub enabled: bool,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ============================================================================
// Zig FFI bindings for pattern detection and redaction
// ============================================================================

#[repr(C)]
struct ZigRedactionResult {
    output: *mut u8,
    output_len: usize,
    match_count: u32,
}

extern "C" {
    fn scred_redact_text_optimized(
        text: *const u8,
        text_len: usize,
    ) -> ZigRedactionResult;

    fn scred_free_redaction_result(result: ZigRedactionResult);
}

pub struct RedactionEngine {
    config: RedactionConfig,
    selector: Option<crate::pattern_selector::PatternSelector>,
}

impl RedactionEngine {
    pub fn new(config: RedactionConfig) -> Self {
        Self {
            config,
            selector: None,
        }
    }

    pub fn with_selector(
        config: RedactionConfig,
        selector: crate::pattern_selector::PatternSelector,
    ) -> Self {
        Self {
            config,
            selector: Some(selector),
        }
    }

    pub fn has_selector(&self) -> bool {
        self.selector.is_some()
    }

    pub fn get_selector(&self) -> Option<&crate::pattern_selector::PatternSelector> {
        self.selector.as_ref()
    }

    pub fn config(&self) -> &RedactionConfig {
        &self.config
    }

    pub fn redact(&self, text: &str) -> RedactionResult {
        if !self.config.enabled {
            return RedactionResult {
                redacted: text.to_string(),
                matches: Vec::new(),
                warnings: Vec::new(),
            };
        }

        // Call Zig FFI for pattern detection and redaction
        unsafe {
            let zig_result = redact_text_optimized(text.as_ptr(), text.len());
            
            // Convert Zig result to Rust result
            if zig_result.output.is_null() || zig_result.output_len == 0 {
                // Zig failed to allocate or redact, return original
                let result = RedactionResult {
                    redacted: text.to_string(),
                    matches: Vec::new(),
                    warnings: vec![RedactionWarning {
                        pattern_type: "zig-ffi-error".to_string(),
                        count: 0,
                    }],
                };
                free_redaction_result(zig_result);
                return result;
            }

            // Convert Zig output to Rust string
            let redacted_slice = std::slice::from_raw_parts(zig_result.output, zig_result.output_len);
            let redacted_text = String::from_utf8_lossy(redacted_slice).into_owned();
            
            // Create basic match info (TODO: Zig should return match details)
            let matches = if zig_result.match_count > 0 {
                vec![PatternMatch {
                    position: 0,
                    pattern_type: "detected".to_string(),
                    original_text: text.to_string(),
                    redacted_text: redacted_text.clone(),
                    match_len: text.len(),
                }]
            } else {
                Vec::new()
            };

            let result = RedactionResult {
                redacted: redacted_text,
                matches,
                warnings: Vec::new(),
            };

            // Free Zig allocated memory
            free_redaction_result(zig_result);
            
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_key_redaction() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "AKIAIOSFODNN7EXAMPLE";
        let result = engine.redact(text);
        assert!(result.redacted.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].pattern_type, "aws-akia");
    }

    #[test]
    fn test_matches_include_metadata() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "GitHub token: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let result = engine.redact(text);
        
        assert!(result.matches.len() > 0, "Should find GitHub token");
        let m = &result.matches[0];
        assert_eq!(m.pattern_type, "github-token");
        assert_eq!(m.original_text, "ghp_abcdefghijklmnopqrstuvwxyz0123456789ab");
        assert_eq!(m.original_text.len(), m.match_len);
        // Position should be somewhere in the text
        assert!(m.position <= text.len());
    }

    #[test]
    fn test_selective_un_redaction_possible() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "AWS: AKIAIOSFODNN7EXAMPLE and GitHub: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let result = engine.redact(text);
        
        // Both should be redacted
        assert!(result.redacted.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert!(result.redacted.contains("ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));
        
        // But we have metadata to selectively un-redact
        assert!(result.matches.len() >= 2, "Should find 2+ patterns");
        let aws_match = result.matches.iter().find(|m| m.pattern_type == "aws-akia");
        let github_match = result.matches.iter().find(|m| m.pattern_type == "github-token");
        assert!(aws_match.is_some(), "Should find AWS");
        assert!(github_match.is_some(), "Should find GitHub");
    }
}
