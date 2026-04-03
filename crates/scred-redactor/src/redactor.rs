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

    #[inline(never)]
    pub fn redact(&self, text: &str) -> RedactionResult {
        if !self.config.enabled {
            return RedactionResult {
                redacted: text.to_string(),
                matches: Vec::new(),
                warnings: Vec::new(),
            };
        }

        // Use pure Rust SIMD pattern detection
        use scred_detector::{detect_all, redact_text};

        let text_bytes = text.as_bytes();

        // Detect all patterns using Rust implementation
        let detection_result = detect_all(text_bytes);

        // Redact matched regions
        let redacted_bytes = if detection_result.count() > 0 {
            redact_text(text_bytes, &detection_result.matches)
        } else {
            text_bytes.to_vec()
        };

        let redacted_text = String::from_utf8_lossy(&redacted_bytes).into_owned();

        // Map pattern types to tier names for selector filtering
        let mut tier_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for m in &detection_result.matches {
            let tier_name = match m.pattern_type {
                0..=99 => "CRITICAL",          // Simple prefix patterns (AWS, etc.)
                100..=199 => "API_KEYS",       // Validation patterns (GitHub, Stripe, etc.)
                200..=299 => "PATTERNS",       // JWT, regex-based patterns
                300..=399 => "INFRASTRUCTURE", // SSH keys, certificates, etc.
                _ => "SERVICES",               // Everything else
            };
            *tier_counts.entry(tier_name.to_string()).or_insert(0) += 1;
        }

        // Populate warnings with tier names for ConfigurableEngine filtering
        let warnings: Vec<RedactionWarning> = tier_counts
            .into_iter()
            .map(|(tier, count)| RedactionWarning {
                pattern_type: tier,
                count,
            })
            .collect();

        // Create match information for each detected pattern
        // Use tier name as pattern_type for proper selector matching
        let matches = detection_result
            .matches
            .iter()
            .map(|m| {
                let original = &text_bytes[m.start..m.end];
                let redacted = &redacted_bytes[m.start..m.end];

                // Map pattern type number to tier name (CRITICAL, API_KEYS, PATTERNS, etc.)
                let tier_name = match m.pattern_type {
                    0..=99 => "CRITICAL",          // Simple prefix patterns
                    100..=199 => "API_KEYS",       // Validation patterns
                    200..=299 => "PATTERNS",       // JWT and regex patterns
                    300..=399 => "INFRASTRUCTURE", // SSH keys, certificates
                    _ => "SERVICES",               // Other patterns
                };

                PatternMatch {
                    position: m.start,
                    pattern_type: tier_name.to_string(),
                    original_text: String::from_utf8_lossy(original).into_owned(),
                    redacted_text: String::from_utf8_lossy(redacted).into_owned(),
                    match_len: m.end - m.start,
                }
            })
            .collect();

        RedactionResult {
            redacted: redacted_text,
            matches,
            warnings,
        }
    }
}
