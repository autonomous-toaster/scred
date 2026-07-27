//! Streaming-optimized policy replacement
//!
//! Uses Aho-Corasick automaton for single-pass placeholder detection,
//! matching scred-detector's architecture for high throughput.
//!
//! # Performance Characteristics
//! - Single-pass: O(n) for all placeholders combined
//! - Zero-copy when possible (length-preserving replacements)
//! - Chunked processing with lookahead for streaming
//! - SIMD-friendly via Aho-Corasick

use aho_corasick::AhoCorasick;
use std::collections::HashMap;

use crate::placeholder::PlaceholderGenerator;
use crate::PolicyError;

/// Pre-compiled placeholder automaton for fast streaming replacement
///
/// Built once at startup, reused for all requests.
/// Uses Aho-Corasick for single-pass matching of all placeholders.
pub struct PlaceholderAutomaton {
    /// Aho-Corasick automaton for finding placeholders
    ac: AhoCorasick,
    /// Map: placeholder value → (real_secret, secret_name)
    /// Index matches position in patterns vector passed to AhoCorasick
    replacements: Vec<(String, String)>,
    /// Placeholder values (for reverse lookup in response path)
    placeholder_values: Vec<String>,
}

impl PlaceholderAutomaton {
    /// Build automaton from policy config
    ///
    /// This is called once at startup. The automaton is then reused
    /// for all requests, making placeholder detection O(n) single-pass.
    pub fn build(
        secrets: &HashMap<String, String>,
        generator: &mut PlaceholderGenerator,
    ) -> Result<Self, PolicyError> {
        let mut patterns: Vec<String> = Vec::new();
        let mut replacements: Vec<(String, String)> = Vec::new();
        let mut placeholder_values: Vec<String> = Vec::new();

        for (name, value) in secrets {
            let placeholder = generator.generate(name, value);
            patterns.push(placeholder.value.clone());
            replacements.push((value.clone(), name.clone()));
            placeholder_values.push(placeholder.value.clone());
        }

        if patterns.is_empty() {
            // No secrets - return empty automaton
            return Ok(Self {
                ac: AhoCorasick::new([""])?,
                replacements: Vec::new(),
                placeholder_values: Vec::new(),
            });
        }

        let ac = AhoCorasick::builder()
            .ascii_case_insensitive(false)
            .build(&patterns)?;

        Ok(Self {
            ac,
            replacements,
            placeholder_values,
        })
    }

    /// Create an empty automaton (no placeholders to match)
    ///
    /// Used when policy is disabled or no secrets are loaded.
    pub fn empty() -> Self {
        Self {
            ac: match AhoCorasick::new([""]) {
                Ok(ac) => ac,
                Err(e) => unreachable!("AhoCorasick with empty pattern failed: {}", e),
            },
            replacements: Vec::new(),
            placeholder_values: Vec::new(),
        }
    }

    /// Create automaton from pre-built parts
    ///
    /// Used by PolicyEngine to create a PlaceholderAutomaton
    /// without rebuilding the automaton.
    pub fn from_parts(
        ac: AhoCorasick,
        replacements: Vec<(String, String)>,
        placeholder_values: Vec<String>,
    ) -> Self {
        Self {
            ac,
            replacements,
            placeholder_values,
        }
    }

    /// Check if automaton has any patterns to match
    pub fn is_empty(&self) -> bool {
        self.placeholder_values.is_empty()
    }

    /// Replace placeholders with real secrets in a buffer (REQUEST path)
    ///
    /// Single-pass O(n) replacement using pre-compiled automaton.
    /// Returns tracker for response processing.
    ///
    /// # Arguments
    /// * `data` - Buffer to process (modified in-place when possible)
    /// * `domain` - Target domain for domain restrictions
    /// * `domain_checker` - Function to check if secret is allowed for domain
    ///
    /// # Returns
    /// * `ReplacementTracker` - For response processing
    /// * `usize` - Number of replacements made
    #[inline]
    pub fn replace_placeholders(
        &self,
        data: &mut [u8],
        domain: &str,
        domain_checker: impl Fn(&str, &str) -> bool,
    ) -> (ReplacementTracker, usize) {
        if self.placeholder_values.is_empty() {
            return (ReplacementTracker::new(), 0);
        }

        let text = match std::str::from_utf8(data) {
            Ok(t) => t,
            Err(_) => return (ReplacementTracker::new(), 0),
        };

        // Collect all matches first (need to know all positions before modifying)
        let matches: Vec<_> = self.ac.find_iter(text).collect();

        if matches.is_empty() {
            return (ReplacementTracker::new(), 0);
        }

        // Build replacement plan
        let mut tracker = ReplacementTracker::new();
        let mut replacements = 0;

        // For in-place replacement, we need to handle length differences
        // Build the output string with all replacements
        let mut result = String::with_capacity(text.len() + matches.len() * 64);
        let mut last_end = 0;

        for m in &matches {
            let pattern_idx = m.pattern().as_usize();
            let placeholder = &self.placeholder_values[pattern_idx];
            let (real_secret, secret_name) = &self.replacements[pattern_idx];

            // Check domain restriction
            if !domain_checker(secret_name, domain) {
                continue;
            }

            // Copy text before match
            result.push_str(&text[last_end..m.start()]);

            // Replace placeholder with real secret
            result.push_str(real_secret);

            // Track for response path
            tracker.track(
                secret_name.clone(),
                real_secret.clone(),
                placeholder.clone(),
            );
            replacements += 1;

            last_end = m.end();
        }

        // Copy remaining text
        result.push_str(&text[last_end..]);

        // Copy back to buffer
        let bytes = result.as_bytes();
        let copy_len = bytes.len().min(data.len());
        data[..copy_len].copy_from_slice(&bytes[..copy_len]);

        // Zero-fill remainder if output is shorter
        if copy_len < data.len() {
            data[copy_len..].fill(0);
        }

        (tracker, replacements)
    }

    /// Replace real secrets with placeholders in a buffer (RESPONSE path)
    ///
    /// Uses tracker from request to reverse the replacement.
    /// Single-pass O(n) using tracker's known secrets.
    #[inline]
    pub fn replace_secrets(&self, data: &mut [u8], tracker: &ReplacementTracker) -> usize {
        let Some(text) = Self::validate_and_get_text(data, tracker) else {
            return 0;
        };

        let Some(ac) = Self::build_automaton(tracker) else {
            return 0;
        };

        let matches: Vec<_> = ac.find_iter(text).collect();

        if matches.is_empty() {
            return 0;
        }

        let (result, replacements) = Self::build_replacement_string(text, &matches, tracker);
        Self::copy_result_back(data, &result);

        replacements
    }

    /// Validate input and get text slice, returning None if no work to do
    fn validate_and_get_text<'a>(data: &'a [u8], tracker: &ReplacementTracker) -> Option<&'a str> {
        if tracker.replacements().is_empty() {
            return None;
        }
        std::str::from_utf8(data).ok()
    }

    /// Build Aho-Corasick automaton from tracked secrets
    fn build_automaton(tracker: &ReplacementTracker) -> Option<AhoCorasick> {
        let tracked_secrets: Vec<&str> =
            tracker.replacements().keys().map(|s| s.as_str()).collect();

        if tracked_secrets.is_empty() {
            return None;
        }

        AhoCorasick::new(&tracked_secrets).ok()
    }

    /// Build replacement string with placeholders
    fn build_replacement_string(
        text: &str,
        matches: &[aho_corasick::Match],
        tracker: &ReplacementTracker,
    ) -> (String, usize) {
        let mut result = String::with_capacity(text.len());
        let mut last_end = 0;
        let mut replacements = 0;

        for m in matches {
            let secret_value = &text[m.start()..m.end()];

            if let Some(placeholder) = tracker.get_placeholder(secret_value) {
                result.push_str(&text[last_end..m.start()]);
                result.push_str(placeholder);
                replacements += 1;
                last_end = m.end();
            }
        }

        result.push_str(&text[last_end..]);
        (result, replacements)
    }

    /// Copy replacement result back to data buffer
    fn copy_result_back(data: &mut [u8], result: &str) {
        let bytes = result.as_bytes();
        let copy_len = bytes.len().min(data.len());
        data[..copy_len].copy_from_slice(&bytes[..copy_len]);
        if copy_len < data.len() {
            data[copy_len..].fill(0);
        }
    }

    /// Streaming chunk processor for request path
    ///
    /// Designed for chunked HTTP body processing. Uses lookahead
    /// to handle placeholders split across chunk boundaries.
    ///
    /// # Arguments
    /// * `chunk` - Current chunk to process
    /// * `lookahead` - Buffer for incomplete match from previous chunk
    /// * `is_eof` - True if this is the last chunk
    ///
    /// # Returns
    /// * `Vec<u8>` - Processed output (may be longer/shorter than input)
    /// * `usize` - Number of replacements made
    pub fn process_chunk_request(
        &self,
        chunk: &[u8],
        lookahead: &mut Vec<u8>,
        is_eof: bool,
    ) -> (Vec<u8>, usize) {
        // Combine lookahead + chunk for cross-boundary matching
        let mut combined = Vec::with_capacity(lookahead.len() + chunk.len());
        combined.extend_from_slice(lookahead);
        combined.extend_from_slice(chunk);

        if self.placeholder_values.is_empty() {
            lookahead.clear();
            return (combined, 0);
        }

        // Find matches in combined buffer
        let text = match std::str::from_utf8(&combined) {
            Ok(t) => t,
            Err(_) => {
                // Not valid UTF-8 at boundary - keep in lookahead
                if is_eof {
                    lookahead.clear();
                    return (combined, 0);
                }
                // Keep last 64 bytes for cross-boundary matching
                let keep = combined.len().saturating_sub(64);
                let output = combined[..keep].to_vec();
                lookahead.clear();
                lookahead.extend_from_slice(&combined[keep..]);
                return (output, 0);
            }
        };

        let matches: Vec<_> = self.ac.find_iter(text).collect();

        if matches.is_empty() {
            if is_eof {
                lookahead.clear();
                return (combined, 0);
            }
            // Keep last N bytes for cross-boundary matching
            let max_placeholder_len = self
                .placeholder_values
                .iter()
                .map(|p| p.len())
                .max()
                .unwrap_or(64);
            let keep = combined.len().saturating_sub(max_placeholder_len);
            let output = combined[..keep].to_vec();
            lookahead.clear();
            lookahead.extend_from_slice(&combined[keep..]);
            return (output, 0);
        }

        // Build output with replacements
        let mut result = Vec::with_capacity(combined.len() + matches.len() * 64);
        let mut last_end = 0;
        let mut replacements = 0;

        for m in &matches {
            let pattern_idx = m.pattern().as_usize();
            let (real_secret, _) = &self.replacements[pattern_idx];

            result.extend_from_slice(&combined[last_end..m.start()]);
            result.extend_from_slice(real_secret.as_bytes());
            replacements += 1;
            last_end = m.end();
        }

        if is_eof {
            result.extend_from_slice(&combined[last_end..]);
            lookahead.clear();
        } else {
            // Keep potential partial match in lookahead
            let max_placeholder_len = self
                .placeholder_values
                .iter()
                .map(|p| p.len())
                .max()
                .unwrap_or(64);
            let boundary = combined.len().saturating_sub(max_placeholder_len);
            let keep_from = last_end.max(boundary);
            result.extend_from_slice(&combined[last_end..keep_from]);
            lookahead.clear();
            lookahead.extend_from_slice(&combined[keep_from..]);
        }

        (result, replacements)
    }

    /// Streaming chunk processor for response path
    ///
    /// Replaces tracked secrets with placeholders. Must be given
    /// tracker from request processing.
    pub fn process_chunk_response(
        &self,
        chunk: &[u8],
        lookahead: &mut Vec<u8>,
        tracker: &ReplacementTracker,
        is_eof: bool,
    ) -> (Vec<u8>, usize) {
        if tracker.replacements().is_empty() {
            lookahead.clear();
            return (chunk.to_vec(), 0);
        }

        // Combine lookahead + chunk
        let mut combined = Vec::with_capacity(lookahead.len() + chunk.len());
        combined.extend_from_slice(lookahead);
        combined.extend_from_slice(chunk);

        // Build automaton from tracked secrets
        let tracked_secrets: Vec<&str> =
            tracker.replacements().keys().map(|s| s.as_str()).collect();

        let Ok(ac) = AhoCorasick::new(&tracked_secrets) else {
            lookahead.clear();
            return (combined, 0);
        };

        let Ok(text) = std::str::from_utf8(&combined) else {
            lookahead.clear();
            return (combined, 0);
        };

        let matches: Vec<_> = ac.find_iter(text).collect();

        if matches.is_empty() {
            return Self::handle_no_matches_response(&combined, lookahead, tracker, is_eof);
        }

        Self::process_matches_response(&combined, text, &matches, lookahead, tracker, is_eof)
    }

    /// Handle case when no matches found in response chunk
    fn handle_no_matches_response(
        combined: &[u8],
        lookahead: &mut Vec<u8>,
        tracker: &ReplacementTracker,
        is_eof: bool,
    ) -> (Vec<u8>, usize) {
        if is_eof {
            lookahead.clear();
            return (combined.to_vec(), 0);
        }
        let max_secret_len = tracker
            .replacements()
            .keys()
            .map(|s| s.len())
            .max()
            .unwrap_or(64);
        let keep = combined.len().saturating_sub(max_secret_len);
        let output = combined[..keep].to_vec();
        lookahead.clear();
        lookahead.extend_from_slice(&combined[keep..]);
        (output, 0)
    }

    /// Process matches in response chunk with lookahead handling
    fn process_matches_response(
        combined: &[u8],
        text: &str,
        matches: &[aho_corasick::Match],
        lookahead: &mut Vec<u8>,
        tracker: &ReplacementTracker,
        is_eof: bool,
    ) -> (Vec<u8>, usize) {
        let mut result = Vec::with_capacity(combined.len());
        let mut last_end = 0;
        let mut replacements = 0;

        for m in matches {
            let secret_value = &text[m.start()..m.end()];

            if let Some(placeholder) = tracker.get_placeholder(secret_value) {
                result.extend_from_slice(&text.as_bytes()[last_end..m.start()]);
                result.extend_from_slice(placeholder.as_bytes());
                replacements += 1;
                last_end = m.end();
            }
        }

        if is_eof {
            result.extend_from_slice(&text.as_bytes()[last_end..]);
            lookahead.clear();
        } else {
            let max_secret_len = tracker
                .replacements()
                .keys()
                .map(|s| s.len())
                .max()
                .unwrap_or(64);
            let boundary = combined.len().saturating_sub(max_secret_len);
            let keep_from = last_end.max(boundary);
            result.extend_from_slice(&combined[last_end..keep_from]);
            lookahead.clear();
            lookahead.extend_from_slice(&combined[keep_from..]);
        }

        (result, replacements)
    }
}

/// Replacement tracker (same as before, but now used with streaming)
#[derive(Debug, Clone, Default)]
pub struct ReplacementTracker {
    /// Map: real_secret_value → (placeholder, secret_name)
    replacements: HashMap<String, (String, String)>,
}

impl ReplacementTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn track(&mut self, secret_name: String, real_value: String, placeholder: String) {
        self.replacements
            .insert(real_value, (placeholder, secret_name));
    }

    pub fn contains_secret(&self, value: &str) -> bool {
        self.replacements.contains_key(value)
    }

    pub fn get_placeholder(&self, value: &str) -> Option<&str> {
        self.replacements.get(value).map(|(p, _)| p.as_str())
    }

    pub fn replacements(&self) -> &HashMap<String, (String, String)> {
        &self.replacements
    }
}
