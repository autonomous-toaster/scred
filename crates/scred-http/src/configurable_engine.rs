//! Detection and Redaction Engine with Pattern Tier Support
//!
//! This module provides a configurable engine that wraps the core RedactionEngine
//! and applies pattern tier filtering to both detection and redaction operations.
//!
//! ## Philosophy
//! - **Conservative Redaction**: All detected patterns are redacted from data streams
//! - **Controlled Visibility**: Users control which patterns appear in logs via `detect_selector`
//! - **Smart Defaults**: Detect broadly (CRITICAL+API_KEYS+INFRASTRUCTURE), redact conservatively (CRITICAL+API_KEYS)
//!
//! ## Usage
//!
//! ```ignore
//! use scred_http::ConfigurableEngine;
//! use scred_redactor::RedactionEngine;
//!
//! let engine = RedactionEngine::new(Default::default());
//! let config_engine = ConfigurableEngine::with_defaults(engine);
//!
//! // Detect only selected patterns (for logging)
//! let warnings = config_engine.detect_only("some secret text");
//!
//! // Redact text (conservative: all patterns redacted)
//! let redacted = config_engine.redact_only("some secret text");
//!
//! // Detect and redact with filtering
//! let result = config_engine.detect_and_redact("some secret text");
//! ```

use scred_redactor::{PatternMatch, RedactionEngine, RedactionWarning};
use std::sync::Arc;

use crate::{get_pattern_tier, PatternSelector};

/// Filtered redaction result that includes detection warnings
#[derive(Debug, Clone)]
pub struct FilteredRedactionResult {
    /// The redacted text (all patterns removed)
    pub redacted: String,
    /// Detection warnings filtered by detect_selector
    pub warnings: Vec<RedactionWarning>,
}

/// Detection and redaction engine with pattern tier support
///
/// This engine combines:
/// - Core redaction logic (from RedactionEngine)
/// - Pattern tier filtering (from PatternSelector)
/// - Selective detection/redaction based on user configuration
///
/// ## Architecture
///
/// The engine maintains two independent selectors:
/// - `detect_selector`: Controls which patterns appear in logs (filtered detection)
/// - `redact_selector`: Controls which patterns are actually redacted (filtered redaction)
///
/// This separation enables the "detect broadly, redact conservatively" philosophy:
/// - Detection can show many patterns (user controls via --detect flag)
/// - Redaction is selective by tier (user controls via --redact flag)
/// - Users see exactly what they're interested in without accidentally leaking other secrets
pub struct ConfigurableEngine {
    /// Core redaction engine that detects and redacts all patterns
    engine: Arc<RedactionEngine>,
    /// Pattern selector for detection (controls logging visibility)
    detect_selector: PatternSelector,
    /// Pattern selector for redaction (future use - currently redacts all)
    redact_selector: PatternSelector,
}

impl ConfigurableEngine {
    /// Create a new ConfigurableEngine with custom selectors
    ///
    /// # Arguments
    /// * `engine` - Core redaction engine
    /// * `detect_selector` - Filters which detected patterns are logged
    /// * `redact_selector` - Filters which patterns are actually redacted (NEW in Phase 8)
    pub fn new(
        engine: Arc<RedactionEngine>,
        detect_selector: PatternSelector,
        redact_selector: PatternSelector,
    ) -> Self {
        Self {
            engine,
            detect_selector,
            redact_selector,
        }
    }

    /// Create ConfigurableEngine with default selectors
    ///
    /// Default behavior:
    /// - **Detect**: CRITICAL + API_KEYS + INFRASTRUCTURE (broad visibility)
    /// - **Redact**: CRITICAL + API_KEYS (conservative, high-confidence)
    pub fn with_defaults(engine: Arc<RedactionEngine>) -> Self {
        Self {
            engine,
            detect_selector: PatternSelector::default_detect(),
            redact_selector: PatternSelector::default_redact(),
        }
    }

    /// Detect patterns in text, returning only those matching detect_selector
    ///
    /// This performs detection but filters warnings to show only patterns
    /// that match the `detect_selector`. Useful for controlled logging.
    ///
    /// # Arguments
    /// * `text` - Input text to analyze
    ///
    /// # Returns
    /// Vector of detection warnings matching the selector
    pub fn detect_only(&self, text: &str) -> Vec<RedactionWarning> {
        // Run core redaction to get all detection
        let result = self.engine.redact(text);

        // Filter warnings by detect_selector
        result
            .warnings
            .into_iter()
            .filter(|warning| {
                let tier = get_pattern_tier(&warning.pattern_type);
                self.detect_selector
                    .matches_pattern(&warning.pattern_type, tier)
            })
            .collect()
    }

    /// Redact text, removing only patterns that match redact_selector
    ///
    /// This detects all patterns but only redacts those matching the selector.
    /// Uses position-based matching for safe, selective redaction.
    ///
    /// # Arguments
    /// * `text` - Input text to redact
    ///
    /// # Returns
    /// Selectively redacted text (same length as input, only selected patterns removed)
    pub fn redact_only(&self, text: &str) -> String {
        let result = self.engine.redact(text);
        // Apply redact_selector to filter which patterns stay redacted
        self.apply_redact_selector(text, &result.matches)
    }

    /// Detect and redact with filtering
    ///
    /// This detects all patterns and applies selectors:
    /// - `detect_selector`: Controls which patterns appear in warnings (logging)
    /// - `redact_selector`: Controls which patterns are actually redacted
    ///
    /// The algorithm:
    /// 1. Run full detection (all patterns)
    /// 2. Get all matches with position information
    /// 3. Filter warnings by detect_selector (for logging visibility)
    /// 4. Filter redactions by redact_selector (selective redaction at match positions)
    /// 5. Return selectively redacted text + filtered warnings
    ///
    /// # Arguments
    /// * `text` - Input text to analyze and redact
    ///
    /// # Returns
    /// FilteredRedactionResult with selectively redacted text and filtered warnings
    pub fn detect_and_redact(&self, text: &str) -> FilteredRedactionResult {
        let result = self.engine.redact(text);

        // Filter warnings by detect_selector (for logging)
        let filtered_warnings: Vec<RedactionWarning> = result
            .warnings
            .iter()
            .filter(|warning| {
                let tier = get_pattern_tier(&warning.pattern_type);
                self.detect_selector
                    .matches_pattern(&warning.pattern_type, tier)
            })
            .cloned()
            .collect();

        // Apply redact_selector to filter which patterns actually get redacted
        // Uses match position information for safe, selective redaction
        let filtered_redacted = self.apply_redact_selector(text, &result.matches);

        FilteredRedactionResult {
            redacted: filtered_redacted,
            warnings: filtered_warnings,
        }
    }

    /// Apply redact_selector to filter redactions using position-based matching
    ///
    /// This uses the PatternMatch position information to selectively redact only
    /// patterns matching the redact_selector, without un-redacting any patterns.
    ///
    /// Algorithm:
    /// 1. Filter matches by redact_selector
    /// 2. Build output by replacing only selected pattern positions with their redacted text
    /// 3. Leave all other text unchanged (original)
    ///
    /// # Arguments
    /// * `original` - Original unredacted text
    /// * `matches` - All detected pattern matches with position info
    ///
    /// # Returns
    /// Text with only redact_selector-matching patterns redacted
    fn apply_redact_selector(&self, original: &str, matches: &[PatternMatch]) -> String {
        use scred_redactor::metadata_cache::RiskTier;

        // Filter matches: keep only those matching redact_selector
        let selected_matches: Vec<&PatternMatch> = matches
            .iter()
            .filter(|m| {
                // Parse tier name from pattern_type (CRITICAL, API_KEYS, PATTERNS, etc.)
                let tier = match m.pattern_type.as_str() {
                    "CRITICAL" => RiskTier::Critical,
                    "API_KEYS" => RiskTier::ApiKeys,
                    "INFRASTRUCTURE" => RiskTier::Infrastructure,
                    "SERVICES" => RiskTier::Services,
                    "PATTERNS" => RiskTier::Patterns,
                    _ => RiskTier::Patterns, // Default to least critical
                };
                self.redact_selector.matches_pattern(&m.pattern_type, tier)
            })
            .collect();

        // If no patterns match selector, return original unchanged
        if selected_matches.is_empty() {
            return original.to_string();
        }

        // If all patterns match selector, return fully redacted
        if selected_matches.len() == matches.len() {
            // Rebuild from redacted_text at each match position
            let mut result = original.to_string();
            for m in selected_matches.iter().rev() {
                // Work backwards to avoid position shifts
                let start = m.position;
                let end = m.position + m.match_len;
                result.replace_range(start..end, &m.redacted_text);
            }
            return result;
        }

        // Partial selection: build output by selectively replacing
        let original_bytes = original.as_bytes();
        let mut result_bytes = Vec::with_capacity(original.len());
        let mut last_end = 0;

        // Sort matches by position for correct iteration
        let mut sorted_matches = selected_matches.clone();
        sorted_matches.sort_by_key(|m| m.position);

        for m in sorted_matches {
            // Copy unmatched text before this match
            result_bytes.extend_from_slice(&original_bytes[last_end..m.position]);
            // Copy redacted text for this match
            result_bytes.extend_from_slice(m.redacted_text.as_bytes());
            last_end = m.position + m.match_len;
        }

        // Copy remaining text after last match
        result_bytes.extend_from_slice(&original_bytes[last_end..]);

        String::from_utf8_lossy(&result_bytes).into_owned()
    }

    /// Get a description of the detect and redact selectors
    ///
    /// Useful for logging what patterns will be shown/redacted.
    pub fn describe(&self) -> String {
        format!(
            "Detect: {}, Redact: {}",
            self.detect_selector.description(),
            self.redact_selector.description()
        )
    }

    /// Get reference to detect selector
    pub fn detect_selector(&self) -> &PatternSelector {
        &self.detect_selector
    }

    /// Get reference to redact selector
    pub fn redact_selector(&self) -> &PatternSelector {
        &self.redact_selector
    }

    /// Replace detect selector
    pub fn set_detect_selector(&mut self, selector: PatternSelector) {
        self.detect_selector = selector;
    }

    /// Replace redact selector
    pub fn set_redact_selector(&mut self, selector: PatternSelector) {
        self.redact_selector = selector;
    }
}
