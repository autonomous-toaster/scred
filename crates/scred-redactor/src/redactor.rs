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
    pub compiled_patterns: Vec<(String, regex::Regex)>,
    selector: Option<crate::pattern_selector::PatternSelector>,
}

impl RedactionEngine {
    pub fn new(config: RedactionConfig) -> Self {
        Self {
            config,
            compiled_patterns: Vec::new(),
            selector: None,
        }
    }

    pub fn with_selector(
        config: RedactionConfig,
        selector: crate::pattern_selector::PatternSelector,
    ) -> Self {
        Self {
            config,
            compiled_patterns: Vec::new(),
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

        self.redact_with_regex(text)
    }

    /// Core redaction engine: returns redacted text + match metadata
    /// This enables selective filtering in streaming mode
    fn redact_with_regex(&self, text: &str) -> RedactionResult {
        use regex::Regex;

        let patterns: Vec<(&str, &str, &str)> = vec![
            ("ghp_", r"ghp_[a-zA-Z0-9_]{36,}", "github-token"),
            ("gho_", r"gho_[a-zA-Z0-9_]{36,}", "github-oauth"),
            ("ghu_", r"ghu_[a-zA-Z0-9_]{36,}", "github-user"),
            ("AKIA", r"AKIA[0-9A-Z]{16}", "aws-akia"),
            ("ASIA", r"ASIA[0-9A-Z]{16}", "aws-access-token"),
            ("sk-", r"sk-[a-zA-Z0-9_-]{20,}", "openai-api-key"),
            ("glpat-", r"glpat-[a-zA-Z0-9_\-]{20,}", "gitlab-token"),
            ("xoxb-", r"xoxb-[a-zA-Z0-9_-]{10,}", "slack-token"),
            ("xoxp-", r"xoxp-[a-zA-Z0-9_-]{10,}", "slack-token"),
            ("jwt", r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+", "jwt"),
        ];

        // Collect all matches with positions
        let mut all_matches: Vec<(usize, usize, String, String)> = Vec::new();
        let mut warning_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for (_name, pattern_str, warning_type) in patterns {
            if let Ok(regex) = Regex::new(pattern_str) {
                for caps in regex.find_iter(text) {
                    let start = caps.start();
                    let end = caps.end();
                    let match_len = end - start;
                    let original_text = text[start..end].to_string();

                    // Check for overlaps (keep LONGEST)
                    let mut should_add = true;
                    let mut indices_to_remove = Vec::new();

                    for (idx, (s, e, _, _)) in all_matches.iter().enumerate() {
                        if !(end <= *s || start >= *e) {
                            let existing_len = e - s;
                            if match_len > existing_len {
                                indices_to_remove.push(idx);
                            } else {
                                should_add = false;
                                break;
                            }
                        }
                    }

                    for idx in indices_to_remove.iter().rev() {
                        all_matches.remove(*idx);
                    }

                    if should_add {
                        all_matches.push((start, end, warning_type.to_string(), original_text));
                        *warning_map.entry(warning_type.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        if all_matches.is_empty() {
            return RedactionResult {
                redacted: text.to_string(),
                matches: Vec::new(),
                warnings: Vec::new(),
            };
        }

        // Build match metadata BEFORE applying redactions
        let mut pattern_matches: Vec<PatternMatch> = Vec::new();
        for (byte_start, byte_end, pattern_type, original_text) in &all_matches {
            let redacted_text = redact_preserve_length(original_text);
            pattern_matches.push(PatternMatch {
                position: *byte_start,
                pattern_type: pattern_type.clone(),
                original_text: original_text.clone(),
                redacted_text,
                match_len: byte_end - byte_start,
            });
        }

        // Apply redactions in reverse order
        let mut sorted_matches = all_matches.clone();
        sorted_matches.sort_by(|a, b| b.0.cmp(&a.0));

        let mut result = text.to_string();
        for (byte_start, byte_end, _pattern_type, original_text) in sorted_matches {
            let redacted = redact_preserve_length(&original_text);
            if byte_start <= result.len() && byte_end <= result.len() && byte_start <= byte_end {
                result.replace_range(byte_start..byte_end, &redacted);
            }
        }

        let warnings: Vec<RedactionWarning> = warning_map
            .into_iter()
            .map(|(pattern_type, count)| RedactionWarning { pattern_type, count })
            .collect();

        RedactionResult {
            redacted: result,
            matches: pattern_matches,
            warnings,
        }
    }
}

/// Redact text while preserving length and prefix
fn redact_preserve_length(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    
    let prefix_len = if text.starts_with("AKIA") || text.starts_with("ASIA") || 
                       text.starts_with("ABIA") || text.starts_with("ACCA") {
        4
    } else if text.starts_with("ghp_") || text.starts_with("gho_") ||
              text.starts_with("ghu_") || text.starts_with("ghs_") ||
              text.starts_with("ghr_") {
        4
    } else if text.starts_with("xoxb-") || text.starts_with("xoxp-") ||
              text.starts_with("xoxs-") || text.starts_with("xoxa-") {
        5
    } else if text.starts_with("sk-") {
        3
    } else if text.starts_with("glpat-") {
        6
    } else if text.starts_with("wJal") || text.starts_with("wJa") || 
              (text.len() == 40 && text.chars().all(|c| c.is_alphanumeric() || c == '/' || c == '+')) {
        4
    } else if text.contains(".") && text.chars().filter(|&c| c == '.').count() == 2 && 
              (text.starts_with("eyJ") || text.starts_with("ew")) {
        10
    } else {
        4.min(chars.len())
    };
    
    let mut result = String::new();
    
    for i in 0..prefix_len.min(chars.len()) {
        result.push(chars[i]);
    }
    
    for _ in prefix_len..chars.len() {
        result.push('x');
    }
    
    result
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
