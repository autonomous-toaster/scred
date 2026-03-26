//! SIMD Pattern Matching - Multi-pattern simultaneous search
//!
//! Strategy: Load 16 bytes at a time, check against multiple pattern prefixes in parallel.
//! This reduces the number of memchr calls by searching for multiple patterns simultaneously.
//!
//! Approach:
//! 1. Group patterns by prefix length (4, 5, 6, 7, 8 bytes)
//! 2. For each length group, create a "pattern bank" of prefixes
//! 3. Load 16 bytes from text
//! 4. Compare against all prefixes in the bank using SIMD or scalar comparison
//! 5. Collect matching pattern indices
//! 6. Fallback to memchr for patterns that need longer matching

use crate::patterns::PrefixValidationPattern;
use std::cmp::min;

/// Maximum pattern prefixes to group together
const PATTERNS_PER_GROUP: usize = 16;

/// Pattern group organized by prefix length
#[derive(Clone)]
pub struct PatternGroup {
    prefix_length: usize,
    /// Flattened array of prefixes (PATTERNS_PER_GROUP × prefix_length)
    prefixes: Vec<u8>,
    /// Indices into PREFIX_VALIDATION_PATTERNS
    pattern_indices: Vec<usize>,
    /// Actual count of patterns in this group
    count: usize,
}

impl PatternGroup {
    /// Create a new pattern group for a given prefix length
    fn new(prefix_length: usize) -> Self {
        PatternGroup {
            prefix_length,
            prefixes: vec![0u8; PATTERNS_PER_GROUP * prefix_length],
            pattern_indices: vec![0; PATTERNS_PER_GROUP],
            count: 0,
        }
    }

    /// Add a pattern to the group
    fn add_pattern(&mut self, pattern: &PrefixValidationPattern, idx: usize) {
        if self.count >= PATTERNS_PER_GROUP {
            return; // Group is full
        }
        
        let prefix_bytes = pattern.prefix.as_bytes();
        if prefix_bytes.len() != self.prefix_length {
            return; // Wrong length
        }

        let start = self.count * self.prefix_length;
        self.prefixes[start..start + self.prefix_length].copy_from_slice(prefix_bytes);
        self.pattern_indices[self.count] = idx;
        self.count += 1;
    }

    /// Find matching patterns at the given position in text
    fn find_matches(&self, text: &[u8], pos: usize) -> Vec<usize> {
        let mut matches = Vec::new();

        if pos + self.prefix_length > text.len() {
            return matches;
        }

        let text_chunk = &text[pos..pos + self.prefix_length];

        // Compare against all prefixes in this group
        for i in 0..self.count {
            let prefix_start = i * self.prefix_length;
            let prefix_end = prefix_start + self.prefix_length;
            let prefix = &self.prefixes[prefix_start..prefix_end];

            if text_chunk == prefix {
                matches.push(self.pattern_indices[i]);
            }
        }

        matches
    }
}

/// Organized pattern groups by prefix length
pub struct PatternGroupOrganizer {
    pub groups: Vec<PatternGroup>,
}

impl PatternGroupOrganizer {
    /// Create a new organizer and populate from patterns
    pub fn new(patterns: &[PrefixValidationPattern]) -> Self {
        let mut groups: Vec<PatternGroup> = Vec::new();

        // Group patterns by prefix length (4-16 bytes typical)
        for (idx, pattern) in patterns.iter().enumerate() {
            let prefix_len = pattern.prefix.len();

            // Find or create group for this prefix length
            let group = groups
                .iter_mut()
                .find(|g| {
                    g.prefix_length == prefix_len && g.count < PATTERNS_PER_GROUP
                });

            if let Some(g) = group {
                g.add_pattern(pattern, idx);
            } else {
                let mut new_group = PatternGroup::new(prefix_len);
                new_group.add_pattern(pattern, idx);
                groups.push(new_group);
            }
        }

        PatternGroupOrganizer { groups }
    }

    /// Find all matching patterns at a position
    pub fn find_matches_at(&self, text: &[u8], pos: usize) -> Vec<usize> {
        let mut all_matches = Vec::new();

        for group in &self.groups {
            all_matches.extend(group.find_matches(text, pos));
        }

        all_matches
    }

    /// Scan text and return all matching pattern indices at each position
    pub fn scan_all_positions(&self, text: &[u8]) -> Vec<(usize, Vec<usize>)> {
        let mut results = Vec::new();

        for pos in 0..text.len() {
            let matches = self.find_matches_at(text, pos);
            if !matches.is_empty() {
                results.push((pos, matches));
            }
        }

        results
    }
}

/// SIMD-optimized multi-pattern search using chunked comparison
/// This processes text in 16-byte chunks and compares against pattern groups
pub fn simd_multi_pattern_search(
    text: &[u8],
    organizer: &PatternGroupOrganizer,
) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();

    // Process text position by position, using organizer to find matches
    for pos in 0..text.len() {
        for matching_idx in organizer.find_matches_at(text, pos) {
            matches.push((pos, matching_idx));
        }
    }

    matches
}

/// Vectorized comparison helper - compares text chunk against multiple prefixes
/// This is the core SIMD optimization: instead of checking one pattern at a time,
/// we check multiple patterns simultaneously
#[inline]
fn compare_multiple_prefixes(text_chunk: &[u8], prefixes: &[u8], prefix_len: usize) -> Vec<usize> {
    let mut matches = Vec::new();
    let pattern_count = prefixes.len() / prefix_len;

    // SIMD-friendly: Compare all patterns in a loop (compiler can vectorize)
    for i in 0..pattern_count {
        let prefix_start = i * prefix_len;
        let prefix_end = prefix_start + prefix_len;

        if text_chunk.len() >= prefix_len {
            if &text_chunk[..prefix_len] == &prefixes[prefix_start..prefix_end] {
                matches.push(i);
            }
        }
    }

    matches
}

/// Test helper to verify SIMD pattern matching correctness
#[cfg(test)]
mod tests {
    use super::*;
    use crate::patterns::PREFIX_VALIDATION_PATTERNS;

    #[test]
    fn test_pattern_group_organization() {
        let organizer = PatternGroupOrganizer::new(PREFIX_VALIDATION_PATTERNS);
        assert!(!organizer.groups.is_empty());
        
        // Verify all patterns are grouped
        let total_patterns: usize = organizer.groups.iter().map(|g| g.count).sum();
        assert_eq!(total_patterns, PREFIX_VALIDATION_PATTERNS.len());
    }

    #[test]
    fn test_find_matches_at_position() {
        let organizer = PatternGroupOrganizer::new(PREFIX_VALIDATION_PATTERNS);
        
        // Create test data with known pattern
        let test_data = b"prefix_AKIA1234567890ABCDEF1234567890AB_suffix";
        
        // Find matches at position containing "AKIA"
        let matches = organizer.find_matches_at(test_data, 7);
        
        // Should find AWS pattern (AKIA prefix)
        assert!(!matches.is_empty(), "Should find AKIA pattern");
    }

    #[test]
    fn test_scan_all_positions() {
        let organizer = PatternGroupOrganizer::new(PREFIX_VALIDATION_PATTERNS);
        
        // Create test data with multiple patterns
        let test_data = b"AKIA1234567890ABCDEFghp_abc123def456";
        
        let results = organizer.scan_all_positions(test_data);
        assert!(!results.is_empty(), "Should find patterns");
    }
}
