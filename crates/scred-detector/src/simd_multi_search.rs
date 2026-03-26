//! Multi-pattern SIMD search - Compare text against multiple prefixes simultaneously
//!
//! Idea: Instead of memchr(prefix1), memchr(prefix2), ... for each pattern,
//! load a text chunk once and compare it against all prefixes in parallel.
//!
//! Problem with current approach:
//! - Pattern 1 prefix "AKIA": memchr(text, "AKIA")  -> calls memchr
//! - Pattern 2 prefix "ASIA": memchr(text, "ASIA")  -> calls memchr again
//! - Pattern 3 prefix "AWS4": memchr(text, "AWS4")  -> calls memchr again
//! 
//! Better approach:
//! - Load 16 bytes from text chunk
//! - Compare against all prefixes (4-16 bytes) in a tight loop
//! - Return all matches in this chunk
//! 
//! Trade-off:
//! + Fewer memchr calls (1 per 16 bytes instead of pattern count)
//! + Text loaded once (cache efficiency)
//! - More complex prefix matching logic
//! - Still O(n × pattern_count) worst case

use crate::patterns::PREFIX_VALIDATION_PATTERNS;

/// Pattern prefix for quick lookup
#[derive(Clone, Copy)]
pub struct PrefixEntry {
    /// First 8 bytes of prefix
    prefix_head: [u8; 8],
    /// Remaining bytes (up to 8 more)
    prefix_tail: [u8; 8],
    /// Actual lengths
    head_len: usize,
    tail_len: usize,
    /// Pattern index in PREFIX_VALIDATION_PATTERNS
    pattern_idx: usize,
}

impl PrefixEntry {
    /// Create from pattern
    fn from_pattern(pattern_idx: usize) -> Self {
        let pattern = &PREFIX_VALIDATION_PATTERNS[pattern_idx];
        let prefix_bytes = pattern.prefix.as_bytes();
        
        let mut entry = PrefixEntry {
            prefix_head: [0u8; 8],
            prefix_tail: [0u8; 8],
            head_len: 0,
            tail_len: 0,
            pattern_idx,
        };
        
        // Split prefix into head (8 bytes) and tail (rest)
        if prefix_bytes.len() <= 8 {
            entry.head_len = prefix_bytes.len();
            entry.prefix_head[..entry.head_len].copy_from_slice(prefix_bytes);
        } else {
            entry.head_len = 8;
            entry.prefix_head.copy_from_slice(&prefix_bytes[0..8]);
            
            let remaining = prefix_bytes.len() - 8;
            entry.tail_len = remaining.min(8);
            entry.prefix_tail[..entry.tail_len].copy_from_slice(&prefix_bytes[8..8 + entry.tail_len]);
        }
        
        entry
    }

    /// Check if this prefix matches at text position
    #[inline]
    fn matches_at(&self, text: &[u8], pos: usize) -> bool {
        // Check head
        if pos + self.head_len > text.len() {
            return false;
        }
        if &text[pos..pos + self.head_len] != &self.prefix_head[..self.head_len] {
            return false;
        }
        
        // Check tail if present
        if self.tail_len > 0 {
            let tail_start = pos + 8;
            if tail_start + self.tail_len > text.len() {
                return false;
            }
            if &text[tail_start..tail_start + self.tail_len] != &self.prefix_tail[..self.tail_len] {
                return false;
            }
        }
        
        true
    }
}

/// Multi-pattern searcher
pub struct MultiPatternSearcher {
    entries: Vec<PrefixEntry>,
}

impl MultiPatternSearcher {
    /// Create searcher for specific pattern indices
    pub fn new(pattern_indices: &[usize]) -> Self {
        let entries = pattern_indices
            .iter()
            .map(|&idx| PrefixEntry::from_pattern(idx))
            .collect();
        
        MultiPatternSearcher { entries }
    }

    /// Find all matching patterns starting at position
    /// Returns vector of (pattern_index, position)
    #[inline]
    pub fn find_matches_at(&self, text: &[u8], pos: usize) -> Vec<usize> {
        let mut matches = Vec::new();
        for entry in &self.entries {
            if entry.matches_at(text, pos) {
                matches.push(entry.pattern_idx);
            }
        }
        matches
    }

    /// Scan text and collect all matches
    /// For each text position, check if any pattern matches
    pub fn scan(&self, text: &[u8]) -> Vec<(usize, usize)> {
        let mut matches = Vec::new();
        
        for pos in 0..text.len() {
            for entry in &self.entries {
                if entry.matches_at(text, pos) {
                    matches.push((pos, entry.pattern_idx));
                }
            }
        }
        
        matches
    }

    /// Optimized scan: only check positions where first byte of some pattern appears
    pub fn scan_filtered(&self, text: &[u8]) -> Vec<(usize, usize)> {
        // First pass: identify relevant first bytes
        let mut relevant_bytes = [false; 256];
        for entry in &self.entries {
            relevant_bytes[entry.prefix_head[0] as usize] = true;
        }

        // Second pass: only check positions with relevant first bytes
        let mut matches = Vec::new();
        for (pos, &byte) in text.iter().enumerate() {
            if relevant_bytes[byte as usize] {
                for entry in &self.entries {
                    if entry.matches_at(text, pos) {
                        matches.push((pos, entry.pattern_idx));
                    }
                }
            }
        }
        
        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_entry_creation() {
        // Test that we can create prefix entries
        let entry = PrefixEntry {
            prefix_head: [b'A', b'K', b'I', b'A', 0, 0, 0, 0],
            prefix_tail: [0; 8],
            head_len: 4,
            tail_len: 0,
            pattern_idx: 0,
        };
        
        let text = b"AKIAIOSFODNN7EXAMPLE";
        assert!(entry.matches_at(text, 0), "Should match AKIA at position 0");
        assert!(!entry.matches_at(text, 1), "Should not match at position 1");
    }

    #[test]
    fn test_multi_pattern_searcher_creation() {
        // Test that MultiPatternSearcher can be created
        let searcher = MultiPatternSearcher::new(&[]);
        let text = b"test data";
        let matches = searcher.scan_filtered(text);
        assert_eq!(matches.len(), 0, "Should find no matches with empty searcher");
    }
}
