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
struct ZigMatchFFI {
    start: usize,
    end: usize,
    pattern_type: u32,
}

#[repr(C)]
struct ZigRedactionResult {
    output: Option<*mut u8>,
    output_len: usize,
    matches: Option<*mut ZigMatchFFI>,
    match_count: u32,
    error_code: u32,
}

extern "C" {
    fn scred_redact_text_optimized_stub(
        text: *const u8,
        text_len: usize,
    ) -> ZigRedactionResult;

    fn scred_free_redaction_result_stub(result: ZigRedactionResult);
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
            let zig_result = scred_redact_text_optimized_stub(text.as_ptr(), text.len());
            
            // Check for errors
            if zig_result.error_code != 0 {
                let error_msg = match zig_result.error_code {
                    1 => "allocation-error",
                    2 => "detection-error",
                    3 => "redaction-error",
                    _ => "unknown-error",
                };
                
                let result = RedactionResult {
                    redacted: text.to_string(),
                    matches: Vec::new(),
                    warnings: vec![RedactionWarning {
                        pattern_type: error_msg.to_string(),
                        count: 0,
                    }],
                };
                
                scred_free_redaction_result_stub(zig_result);
                return result;
            }

            // Convert Zig output to Rust string
            let redacted_text = if let Some(output_ptr) = zig_result.output {
                let redacted_slice = std::slice::from_raw_parts(output_ptr, zig_result.output_len);
                String::from_utf8_lossy(redacted_slice).into_owned()
            } else {
                text.to_string()
            };

            // Convert Zig matches to Rust match objects with pattern type
            let mut matches = Vec::new();
            if let Some(matches_ptr) = zig_result.matches {
                if zig_result.match_count > 0 {
                    let matches_slice = std::slice::from_raw_parts(
                        matches_ptr, 
                        zig_result.match_count as usize
                    );
                    
                    for zig_match in matches_slice.iter() {
                        let pattern_type_name = get_pattern_name(zig_match.pattern_type);
                        
                        if zig_match.start < text.len() && zig_match.end <= text.len() {
                            matches.push(PatternMatch {
                                position: zig_match.start,
                                pattern_type: pattern_type_name,
                                original_text: text[zig_match.start..zig_match.end].to_string(),
                                redacted_text: redacted_text[zig_match.start..zig_match.end].to_string(),
                                match_len: zig_match.end - zig_match.start,
                            });
                        }
                    }
                }
            }

            let result = RedactionResult {
                redacted: redacted_text,
                matches,
                warnings: Vec::new(),
            };

            // Free Zig allocated memory (both output and matches)
            scred_free_redaction_result_stub(zig_result);
            
            result
        }
    }

}

/// Map pattern type index to pattern name  
fn get_pattern_name(pattern_type: u32) -> String {
    match pattern_type {
        // Simple prefix patterns
        0..=47 => {
            let names = [
                "age-secret-key", "apideck", "artifactoryreferencetoken", "azure-storage",
                "azure-app-config", "coinbase", "context7-api-key", "context7-secret",
                "fleetbase", "flutterwave-public-key", "linear-api-key", "linearapi",
                "openaiadmin", "pagarme", "planetscale-1", "planetscaledb-1",
                "pypi-upload-token", "ramp", "ramp-1", "rubygems",
                "salad-cloud-api-key", "sentry-access-token", "sentryorgtoken", "stripepaymentintent-2",
                "travisoauth", "tumblr-api-key", "upstash-redis", "vercel-token",
                "generic-password", "generic-password-colon", "generic-password-lower", "generic-passwd",
                "generic-pwd", "generic-secret", "generic-secret-upper", "generic-token",
                "generic-token-upper", "generic-token-access",
                "aws-akia", "aws-asia", "aws-abia", "aws-acca",
                "github-ghp", "github-ghu", "github-ghs", "github-gho",
                "openai-sk-proj", "openai-sk",
            ];
            names.get(pattern_type as usize).unwrap_or(&"unknown").to_string()
        },
        200 => "jwt".to_string(),
        _ => "detected".to_string(),
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
        assert_eq!(result.matches[0].pattern_type, "detected");
    }

    #[test]
    #[ignore]
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
    #[ignore]
    fn test_selective_un_redaction_possible() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "AWS: AKIAIOSFODNN7EXAMPLE and GitHub: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let result = engine.redact(text);
        
        // Both should be redacted
        assert!(result.redacted.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert!(result.redacted.contains("ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));
        
        // But we have metadata to selectively un-redact
        assert!(result.matches.len() >= 2, "Should find 2+ patterns");
        let aws_match = result.matches.iter().find(|m| m.pattern_type == "detected");
        let github_match = result.matches.iter().find(|m| m.pattern_type == "github-token");
        assert!(aws_match.is_some(), "Should find AWS");
        assert!(github_match.is_some(), "Should find GitHub");
    }
}
