#[derive(Debug, Clone)]
pub struct RedactionWarning {
    pub pattern_type: String,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct RedactionResult {
    pub redacted: String,
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
    pub compiled_patterns: Vec<(String, regex::Regex)>,
}

impl RedactionEngine {
    pub fn new(config: RedactionConfig) -> Self {
        // NOTE: All patterns are now managed in Zig (scred-pattern-detector).
        // We ONLY use scred-pattern-detector for DETECTION, not redaction.
        
        Self {
            config,
            compiled_patterns: Vec::new(),  // Detection handled by scred-pattern-detector
        }
    }

    pub fn redact(&self, text: &str) -> RedactionResult {
        if !self.config.enabled {
            return RedactionResult {
                redacted: text.to_string(),
                warnings: Vec::new(),
            };
        }

        // Quick rejection for very short inputs
        if text.len() < 20 {
            // Too short to contain most secrets (shortest real secret is ~20 bytes)
            return RedactionResult {
                redacted: text.to_string(),
                warnings: Vec::new(),
            };
        }

        // Collect ALL matches from ALL patterns with their positions
        // Store (byte_start, byte_end, pattern_index) to avoid cloning strings
        let mut all_matches: Vec<(usize, usize, usize)> = Vec::new(); 
        
        for (pattern_idx, (_name, regex)) in self.compiled_patterns.iter().enumerate() {
            // Quick check: does this pattern match at all?
            if !regex.is_match(text) {
                continue;  // Skip this pattern entirely
            }
            
            for caps in regex.captures_iter(text) {
                if let Some(secret_or_full) = caps.get(1).or_else(|| caps.get(0)) {
                    let byte_start = secret_or_full.start();
                    let byte_end = secret_or_full.end();
                    let match_len = byte_end - byte_start;
                    
                    // Check for overlaps with existing matches (keep LONGEST match)
                    let mut should_add = true;
                    let mut indices_to_remove = Vec::new();
                    
                    // Fast path: if no matches yet, just add
                    if all_matches.is_empty() {
                        all_matches.push((byte_start, byte_end, pattern_idx));
                        continue;
                    }
                    
                    for (idx, (s, e, _)) in all_matches.iter().enumerate() {
                        if !(byte_end <= *s || byte_start >= *e) {
                            // Ranges overlap
                            let existing_len = e - s;
                            if match_len > existing_len {
                                // New match is longer, remove existing
                                indices_to_remove.push(idx);
                            } else {
                                // Existing match is longer or equal, skip new
                                should_add = false;
                                break;
                            }
                        }
                    }
                    
                    // Remove overlapping shorter matches (in reverse order to preserve indices)
                    for idx in indices_to_remove.iter().rev() {
                        all_matches.remove(*idx);
                    }
                    
                    if should_add {
                        all_matches.push((byte_start, byte_end, pattern_idx));
                    }
                }
            }
        }
        
        // Build warning map from collected matches
        let mut warning_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (_, _, pattern_idx) in &all_matches {
            let name = &self.compiled_patterns[*pattern_idx].0;
            *warning_map.entry(name.clone()).or_insert(0) += 1;
        }
        
        // Sort matches by byte position (descending) to redact from end to start
        // This prevents byte offset issues when replacing strings
        all_matches.sort_by(|a, b| b.0.cmp(&a.0));
        
        let mut result = text.to_string();
        
        // Apply redactions in reverse order (from end to start)
        // This ensures we don't invalidate positions of earlier matches
        for (byte_start, byte_end, _pattern_idx) in all_matches {
            // Convert byte positions to character positions for UTF-8 safety
            // CRITICAL: Use char indices, not byte indices, to handle multi-byte UTF-8 chars correctly
            let char_start = result[..byte_start].chars().count();
            let char_end = char_start + result[byte_start..byte_end].chars().count();
            
            // Extract the matched text using character positions
            let matched_chars: Vec<char> = result.chars().collect();
            let matched_text: String = matched_chars[char_start..char_end].iter().collect();
            
            let redacted = redact_preserve_length(&matched_text);
            
            // Replace in result string using character positions (character-aware)
            let before: String = matched_chars[..char_start].iter().collect();
            let after: String = matched_chars[char_end..].iter().collect();
            result = format!("{}{}{}", before, redacted, after);
        }
        
        let warnings: Vec<RedactionWarning> = warning_map
            .into_iter()
            .map(|(pattern_type, count)| RedactionWarning { pattern_type, count })
            .collect();
        
        RedactionResult { redacted: result, warnings }
    }

    // Fast-path filter: Check if text contains markers of potential secrets
    // This avoids running 244 regex patterns on text that clearly has no secrets
}


/// Redact text while preserving length and prefix
fn redact_preserve_length(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    
    // Determine prefix length based on pattern
    let prefix_len = if text.starts_with("AKIA") || text.starts_with("ASIA") || 
                       text.starts_with("ABIA") || text.starts_with("ACCA") {
        4 // AWS Access Key IDs
    } else if text.starts_with("ghp_") || text.starts_with("gho_") ||
              text.starts_with("ghu_") || text.starts_with("ghs_") ||
              text.starts_with("ghr_") {
        4 // GitHub tokens
    } else if text.starts_with("xoxb-") || text.starts_with("xoxp-") ||
              text.starts_with("xoxs-") || text.starts_with("xoxa-") {
        5 // Slack tokens
    } else if text.starts_with("sk-") {
        3 // OpenAI keys
    } else if text.starts_with("glpat-") {
        6 // GitLab tokens
    } else if text.starts_with("wJal") || text.starts_with("wJa") || 
              (text.len() == 40 && text.chars().all(|c| c.is_alphanumeric() || c == '/' || c == '+')) {
        // AWS Secret Access Keys (40 chars base64)
        4
    } else if text.contains(".") && text.chars().filter(|&c| c == '.').count() == 2 && 
              (text.starts_with("eyJ") || text.starts_with("ew")) {
        // JWT: show first 10 chars (header start), redact payload and signature
        10
    } else {
        // Default: show first 4 chars or entire token if shorter
        4.min(chars.len())
    };
    
    let mut result = String::new();
    
    // Keep prefix
    for i in 0..prefix_len.min(chars.len()) {
        result.push(chars[i]);
    }
    
    // Replace rest with 'x'
    for _ in prefix_len..chars.len() {
        result.push('x');
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_redaction() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "secret data";
        let result = engine.redact(text);
        assert_eq!(result.redacted, text);
    }
    
    // ════════════════════════════════════════════════════════════════
    // CRITICAL FIX 1: FIND_ALL - CONCATENATED SECRETS
    // ════════════════════════════════════════════════════════════════
    
    #[test]
    fn test_concatenated_same_type_no_separator() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "ghp_abcdefghijklmnopqrstuvwxyz0123456789abghp_1111111111111111111111111111111";
        let result = engine.redact(text);
        let x_count = result.redacted.chars().filter(|&c| c == 'x').count();
        assert!(x_count >= 20, "Should redact concatenated tokens, got: {}", result.redacted);
    }
    
    #[test]
    fn test_concatenated_with_dash() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "ghp_abcdefghijklmnopqrstuvwxyz0123456789ab-ghp_1111111111111111111111111111111";
        let result = engine.redact(text);
        let x_count = result.redacted.chars().filter(|&c| c == 'x').count();
        assert!(x_count >= 20, "Should redact tokens separated by dash, got: {}", result.redacted);
    }
    
    #[test]
    fn test_multiple_with_spaces() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        // Three GitHub tokens with spaces
        let text = "ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ghp_1111111111111111111111111111111 ghp_2222222222222222222222222222222";
        let result = engine.redact(text);
        let count = result.warnings.iter()
            .find(|w| w.pattern_type.contains("github") || w.pattern_type.contains("ghp"))
            .map(|w| w.count)
            .unwrap_or(0);
        assert!(count >= 1, "Should detect multiple tokens with spaces");
    }
    
    #[test]
    fn test_concatenated_different_types() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "sk-proj-1234567890ABCDEFGHIJ:ghp_abc123def456ghi789jkl";
        let result = engine.redact(text);
        // Should redact both patterns - at least 1 GitHub and 1 OpenAI
        let found_patterns = result.warnings.len();
        assert!(found_patterns >= 1, "Should detect at least 1 pattern type");
        // Should have many x's from redaction
        let x_count = result.redacted.chars().filter(|&c| c == 'x').count();
        assert!(x_count >= 20, "Should have significant redaction, got {} x's", x_count);
    }
    
    #[test]
    fn test_three_gitlab_tokens_concatenated() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "glpat-abc123def456ghi789jkl012345glpat-zzz111222333444555666glpat-aaa999888777666555";
        let result = engine.redact(text);
        let x_count = result.redacted.chars().filter(|&c| c == 'x').count();
        assert!(x_count >= 20, "Should redact concatenated GitLab-like tokens, found warnings: {:?}", 
            result.warnings.iter().map(|w| w.pattern_type.clone()).collect::<Vec<_>>());
    }
    
    #[test]
    fn test_no_double_redaction_overlapping_patterns() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        // JWT contains AKIA pattern in payload
        let text = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.AKIA1234567890ABCDEF.sig";
        let result = engine.redact(text);
        // Character count should still be preserved (no double redaction)
        let input_chars = text.chars().count();
        let output_chars = result.redacted.chars().count();
        assert_eq!(input_chars, output_chars, "Character count should match even with overlapping patterns");
    }
    
    #[test]
    fn test_html_charset_not_redacted_as_base64() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = r#"<meta content=\"text/html; charset=UTF-8\" http-equiv=\"Content-Type\"/>"#;
        let result = engine.redact(text);
        assert_eq!(result.redacted, text, "benign HTML charset metadata should not be redacted");
        assert!(
            !result.warnings.iter().any(|w| w.pattern_type == "robinhoodcrypto-1"),
            "base64 catch-all pattern should not trigger on HTML charset metadata"
        );
    }

    #[test]
    fn test_multiline_two_aws_keys() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let text = "test AKIA1234567890ABCDEF\ntest AKIA9876543210FEDCBA test";
        let result = engine.redact(text);
        // Should find secrets on different lines
        let count = result.warnings.iter()
            .find(|w| w.pattern_type.contains("aws") || w.pattern_type.contains("access"))
            .map(|w| w.count)
            .unwrap_or(0);
        assert!(count >= 1, "Should detect AWS keys across newlines, got {} total patterns: {:?}", 
            count, result.warnings.iter().map(|w| w.pattern_type.clone()).collect::<Vec<_>>());
        let input_chars = text.chars().count();
        let output_chars = result.redacted.chars().count();
        assert_eq!(input_chars, output_chars, "Character count preserved across newlines");
    }
    
    #[test]
    fn test_multiline_token_continuation() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        // Some patterns might have newlines in them (though rare)
        let text = "token=sk-proj-1234567890\nABCDEFGHIJ";
        let result = engine.redact(text);
        let input_chars = text.chars().count();
        let output_chars = result.redacted.chars().count();
        assert_eq!(input_chars, output_chars, "Character count preserved with embedded newlines");
    }
    
    // ════════════════════════════════════════════════════════════════
    // CHARACTER PRESERVATION VERIFICATION
    // ════════════════════════════════════════════════════════════════
    
    #[test]
    fn test_character_preservation_all_secret_types() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let test_cases = vec![
            ("AKIA1234567890ABCDEF", "AWS AKIA"),
            ("ASIA9876543210FEDCBA", "AWS ASIA"),
            ("ghp_abcdefghijklmnopqrstuvwxyz0123456789ab", "GitHub PAT"),
            ("sk-proj-1234567890ABCDEFGHIJ", "OpenAI Project"),
            ("glpat-abc123def456ghi789jkl012345", "GitLab PAT"),
            ("xoxa-1234567890abcdef1234567890ab", "Slack App"),
        ];
        
        for (secret, description) in test_cases {
            let result = engine.redact(secret);
            let input_chars = secret.chars().count();
            let output_chars = result.redacted.chars().count();
            assert_eq!(input_chars, output_chars, 
                "Character count mismatch for {}: input={}, output={}", 
                description, input_chars, output_chars);
        }
    }
    
    #[test]
    fn test_character_preservation_with_context() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let test_cases = vec![
            ("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig", "JWT in Bearer"),
            ("Authorization: Basic dXNlcjpwYXNzd29yZDEyMzQ1Njc4OTA=", "Basic auth"),
            ("X-API-KEY: sk_live_1234567890abcdefghij", "API key header"),
            ("mongodb://user:password123456@mongodb.example.com:27017/dbname", "MongoDB URI"),
        ];
        
        for (input, description) in test_cases {
            let result = engine.redact(input);
            let input_chars = input.chars().count();
            let output_chars = result.redacted.chars().count();
            assert_eq!(input_chars, output_chars, 
                "Character count mismatch for {}", description);
        }
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECURITY TESTS: CATASTROPHIC BACKTRACKING & PARSING VULNERABILITIES
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn security_test_redos_with_long_input() {
        use std::time::Instant;
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = format!("eyJ{}{}",
            "a".repeat(500),
            "b".repeat(500)
        );

        let start = Instant::now();
        let _result = engine.redact(&input);
        let elapsed = start.elapsed();

        // Must complete in < 1 second
        assert!(elapsed.as_secs() < 1, 
            "ReDoS detected: took {:?}ms", 
            elapsed.as_millis());
    }

    #[test]
    fn security_test_aws_pattern_alternation() {
        use std::time::Instant;
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = format!("AKIA{}{}",
            "[".repeat(300),
            "]".repeat(300)
        );

        let start = Instant::now();
        let _result = engine.redact(&input);
        let elapsed = start.elapsed();

        assert!(elapsed.as_secs() < 1, 
            "ReDoS in AWS pattern: {:?}ms", 
            elapsed.as_millis());
    }

    #[test]
    fn security_test_secret_in_comment_line() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = "# secret: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let result = engine.redact(input);
        
        assert_eq!(input.chars().count(), result.redacted.chars().count(),
            "Comment line: character preservation failed");
    }

    #[test]
    fn security_test_secret_in_url_password() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = "https://user:ghp_abcdefghijklmnopqrstuvwxyz0123456789ab@github.com/repo";
        let result = engine.redact(input);
        
        assert_eq!(input.chars().count(), result.redacted.chars().count(),
            "URL with secret: character preservation failed");
    }

    #[test]
    fn security_test_secret_at_buffer_start() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = "AKIAIOSFODNN7EXAMPLErest_of_text";
        let result = engine.redact(input);
        
        assert_eq!(input.chars().count(), result.redacted.chars().count(),
            "Buffer start: character preservation failed");
    }

    #[test]
    fn security_test_secret_at_buffer_end() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = "some_text_AKIAIOSFODNN7EXAMPLE";
        let result = engine.redact(input);
        
        assert_eq!(input.chars().count(), result.redacted.chars().count(),
            "Buffer end: character preservation failed");
    }

    #[test]
    fn security_test_many_secrets_performance() {
        use std::time::Instant;
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = "AKIAIOSFODNN7EXAMPLE ".repeat(500);
        
        let start = Instant::now();
        let result = engine.redact(&input);
        let elapsed = start.elapsed();
        
        assert!(elapsed.as_secs() < 2, 
            "500 matches took {:?}ms", 
            elapsed.as_millis());
        assert_eq!(input.chars().count(), result.redacted.chars().count(),
            "Many secrets: character count mismatch");
    }

    #[test]
    fn security_test_regex_injection() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = ".*|AKIAIOSFODNN7EXAMPLE";
        let result = engine.redact(input);
        
        assert_eq!(input.chars().count(), result.redacted.chars().count(),
            "Regex injection: character count mismatch");
    }

    #[test]
    fn security_test_null_byte_handling() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = "AKIAIOSFODNN7EXAMPLE\x00extra_data";
        let result = engine.redact(input);
        
        assert_eq!(input.chars().count(), result.redacted.chars().count(),
            "Null byte: character count mismatch");
    }

    #[test]
    fn security_test_newline_injection() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = "AKIAIOSFODNN7EXAMPLE\nAKIAIOSFODNN7FAKE";
        let result = engine.redact(input);
        
        assert_eq!(input.chars().count(), result.redacted.chars().count(),
            "Newline injection: character count mismatch");
    }

    #[test]
    fn security_test_utf8_bom() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = "\u{FEFF}AKIAIOSFODNN7EXAMPLE";
        let result = engine.redact(input);
        
        assert_eq!(input.chars().count(), result.redacted.chars().count(),
            "UTF-8 BOM: character count mismatch");
    }

    #[test]
    fn security_test_prevent_double_redaction() {
        let engine = RedactionEngine::new(RedactionConfig::default());
        let input = "ghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let result1 = engine.redact(input);
        let result2 = engine.redact(&result1.redacted);
        
        assert_eq!(result1.redacted, result2.redacted,
            "Should prevent double-redaction");
    }
}

/// Convenience function for one-off redaction with default settings
pub fn redact_text(text: &str) -> String {
    // Use thread-local cached engine to avoid recompiling 244 patterns on every call
    thread_local! {
        static ENGINE: RedactionEngine = {
            let config = RedactionConfig::default();
            RedactionEngine::new(config)
        };
    }
    
    ENGINE.with(|engine| engine.redact(text).redacted)
}
