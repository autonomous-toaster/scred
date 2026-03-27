//! Prefix-based index for efficient multiline pattern matching
//!
//! Optimizes multiline pattern detection by building an index of common
//! pattern prefixes (first 8-16 bytes) and dispatching only to relevant
//! patterns during scanning.
//!
//! Example:
//! ```
//! Pattern: "-----BEGIN RSA PRIVATE KEY-----"
//! Prefix:  "-----BEGIN" (10 bytes)
//! Index maps this prefix to pattern indices that start with it
//! ```

use crate::patterns::GeneralizedMarkerPattern;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Prefix-based index for O(1) pattern dispatch
/// 
/// Maps pattern start_marker prefixes (up to 16 bytes) to their pattern indices
/// Enables scanning text once instead of checking all patterns at each position
#[derive(Debug)]
pub struct PrefixIndex {
    /// Map from prefix string to list of matching pattern indices
    index: HashMap<String, Vec<usize>>,
    
    /// Maximum prefix length to extract (usually 16)
    max_prefix_len: usize,
}

impl PrefixIndex {
    /// Build index from generalized marker patterns
    pub fn build(patterns: &[GeneralizedMarkerPattern]) -> Self {
        let mut index = HashMap::new();
        const MAX_PREFIX: usize = 16;

        for (idx, pattern) in patterns.iter().enumerate() {
            // Extract prefix: up to 16 bytes, or full marker if shorter
            let marker = pattern.start_marker;
            let prefix_len = std::cmp::min(MAX_PREFIX, marker.len());
            let prefix = marker[..prefix_len].to_string();
            
            index
                .entry(prefix)
                .or_insert_with(Vec::new)
                .push(idx);
        }

        PrefixIndex {
            index,
            max_prefix_len: MAX_PREFIX,
        }
    }

    /// Get list of pattern indices that could match at this position
    /// Returns None if prefix doesn't match any pattern
    pub fn get_candidates(&self, text: &[u8], pos: usize) -> Option<&Vec<usize>> {
        // Need at least minimum prefix length
        if pos + 8 > text.len() {
            return None;
        }

        let remaining = text.len() - pos;
        let prefix_len = std::cmp::min(self.max_prefix_len, remaining);
        
        // Convert bytes to string for lookup
        let prefix_bytes = &text[pos..pos + prefix_len];
        if let Ok(prefix_str) = std::str::from_utf8(prefix_bytes) {
            self.index.get(prefix_str)
        } else {
            None
        }
    }

    /// Try progressively shorter prefixes if exact match fails
    /// Useful for text that doesn't start exactly on pattern boundary
    pub fn get_candidates_fuzzy(&self, text: &[u8], pos: usize) -> Option<&Vec<usize>> {
        // Try exact match first
        if let Some(candidates) = self.get_candidates(text, pos) {
            return Some(candidates);
        }

        // Try 8-byte prefixes for failed cases
        if pos + 8 <= text.len() {
            let prefix_bytes = &text[pos..pos + 8];
            if let Ok(prefix_str) = std::str::from_utf8(prefix_bytes) {
                // Look for any pattern that starts with this 8-byte prefix
                for (key, candidates) in &self.index {
                    if key.starts_with(prefix_str) && !candidates.is_empty() {
                        return Some(candidates);
                    }
                }
            }
        }

        None
    }

    /// Get all prefixes in index (for debugging)
    pub fn prefixes(&self) -> Vec<String> {
        self.index.keys().cloned().collect()
    }

    /// Get number of unique prefixes
    pub fn prefix_count(&self) -> usize {
        self.index.len()
    }

    /// Get statistics about the index
    pub fn stats(&self) -> (usize, usize) {
        let total_patterns: usize = self.index.values().map(|v| v.len()).sum();
        (self.prefix_count(), total_patterns)
    }
}

/// Global prefix index for multiline pattern detection
pub static PREFIX_INDEX: OnceLock<PrefixIndex> = OnceLock::new();

/// Initialize the global prefix index (called once at startup)
pub fn init_prefix_index(patterns: &[GeneralizedMarkerPattern]) -> &'static PrefixIndex {
    PREFIX_INDEX.get_or_init(|| {
        let index = PrefixIndex::build(patterns);
        let (prefix_count, pattern_count) = index.stats();
        eprintln!(
            "[PrefixIndex] Built index: {} prefixes → {} patterns",
            prefix_count, pattern_count
        );
        index
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_index_creation() {
        let patterns = &[
            GeneralizedMarkerPattern {
                name: "test-ssh-key",
                start_marker: "-----BEGIN RSA PRIVATE KEY-----",
                end_marker: "-----END RSA PRIVATE KEY-----",
                tier: crate::patterns::PatternTier::Critical,
                max_lookahead: 4096,
                contains_keyword: None,
                exclude_keyword: None,
                min_body_len: 0,
                pattern_type: 300,
            },
        ];

        let index = PrefixIndex::build(patterns);
        assert!(index.prefixes().len() > 0, "Should have at least one prefix");
        assert_eq!(index.prefix_count(), 1);
    }

    #[test]
    fn test_prefix_lookup() {
        let patterns = &[
            GeneralizedMarkerPattern {
                name: "test-ssh-key",
                start_marker: "-----BEGIN RSA PRIVATE",
                end_marker: "-----END RSA-----",
                tier: crate::patterns::PatternTier::Critical,
                max_lookahead: 4096,
                contains_keyword: None,
                exclude_keyword: None,
                min_body_len: 0,
                pattern_type: 300,
            },
        ];

        let index = PrefixIndex::build(patterns);
        let text = b"-----BEGIN RSA PRIVATE KEY-----";
        
        let candidates = index.get_candidates(text, 0);
        assert!(candidates.is_some(), "Should find candidates at position 0");
        assert_eq!(candidates.unwrap().len(), 1);
    }

    #[test]
    fn test_prefix_no_match() {
        let patterns = &[
            GeneralizedMarkerPattern {
                name: "test-ssh-key",
                start_marker: "-----BEGIN RSA",
                end_marker: "-----END RSA-----",
                tier: crate::patterns::PatternTier::Critical,
                max_lookahead: 4096,
                contains_keyword: None,
                exclude_keyword: None,
                min_body_len: 0,
                pattern_type: 300,
            },
        ];

        let index = PrefixIndex::build(patterns);
        let text = b"some random text";
        
        let candidates = index.get_candidates(text, 0);
        assert!(candidates.is_none() || candidates.unwrap().is_empty(), 
                "Should return no candidates for non-matching text");
    }

    #[test]
    fn test_multiple_patterns() {
        let patterns = &[
            GeneralizedMarkerPattern {
                name: "rsa-key",
                start_marker: "-----BEGIN RSA PRIVATE KEY-----",
                end_marker: "-----END RSA-----",
                tier: crate::patterns::PatternTier::Critical,
                max_lookahead: 4096,
                contains_keyword: None,
                exclude_keyword: None,
                min_body_len: 0,
                pattern_type: 300,
            },
            GeneralizedMarkerPattern {
                name: "openssh-key",
                start_marker: "-----BEGIN OPENSSH PRIVATE KEY-----",
                end_marker: "-----END OPENSSH-----",
                tier: crate::patterns::PatternTier::Critical,
                max_lookahead: 4096,
                contains_keyword: None,
                exclude_keyword: None,
                min_body_len: 0,
                pattern_type: 301,
            },
        ];

        let index = PrefixIndex::build(patterns);
        assert_eq!(index.prefix_count(), 2, "Should have 2 distinct prefixes");
    }
}
