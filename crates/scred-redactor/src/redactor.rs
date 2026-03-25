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

        // Use Zig FFI for all pattern detection (274 patterns from patterns.zig)
        // Patterns are decomposed: 72 simple prefix, 200 prefix+validation, rest regex-only
        RedactionResult {
            redacted: text.to_string(),
            matches: Vec::new(), // TODO: Implement Zig FFI integration
            warnings: vec![RedactionWarning {
                pattern_type: "zig-ffi-pending".to_string(),
                count: 0,
            }],
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
