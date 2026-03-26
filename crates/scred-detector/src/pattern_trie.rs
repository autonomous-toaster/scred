//! Pattern Trie - Data structure for efficient multi-pattern searching
//! 
//! Instead of checking each pattern independently with memchr,
//! build a prefix tree and traverse it at each text position.
//! This reduces the number of memchr calls and enables prefix sharing.

use crate::patterns::{PrefixValidationPattern, PREFIX_VALIDATION_PATTERNS};
use std::collections::HashMap;

/// A node in the pattern trie
#[derive(Clone)]
pub struct TrieNode {
    /// Children indexed by byte value
    children: HashMap<u8, Box<TrieNode>>,
    /// If Some, this is a leaf node pointing to pattern index
    pattern_idx: Option<usize>,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            pattern_idx: None,
        }
    }
}

/// Pattern trie for efficient multi-pattern prefix matching
pub struct PatternTrie {
    root: TrieNode,
    max_prefix_len: usize,
}

impl PatternTrie {
    /// Build trie from pattern list
    pub fn new(patterns: &[PrefixValidationPattern]) -> Self {
        let mut root = TrieNode::new();
        let mut max_prefix_len = 0;

        for (idx, pattern) in patterns.iter().enumerate() {
            let prefix = pattern.prefix.as_bytes();
            max_prefix_len = max_prefix_len.max(prefix.len());
            
            let mut node = &mut root;
            for (i, &byte) in prefix.iter().enumerate() {
                node = node.children
                    .entry(byte)
                    .or_insert_with(|| Box::new(TrieNode::new()));
                
                // Mark leaf node with pattern index
                if i == prefix.len() - 1 {
                    node.pattern_idx = Some(idx);
                }
            }
        }

        PatternTrie { root, max_prefix_len }
    }

    /// Find all patterns matching at a position in text
    /// Returns vector of pattern indices that match at this position
    pub fn find_matches_at(&self, text: &[u8], pos: usize) -> Vec<usize> {
        let mut matches = Vec::new();
        let mut node = &self.root;
        
        for i in 0..self.max_prefix_len {
            if pos + i >= text.len() {
                break;
            }
            
            let byte = text[pos + i];
            match node.children.get(&byte) {
                Some(next_node) => {
                    if let Some(idx) = next_node.pattern_idx {
                        matches.push(idx);
                    }
                    node = next_node;
                }
                None => break,
            }
        }
        
        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_trie_basic() {
        // Create simple test patterns
        let patterns = vec![
            PrefixValidationPattern {
                name: "test1",
                prefix: "AKIA",
                tier: crate::patterns::PatternTier::Critical,
                min_len: 20,
                max_len: 200,
                charset: crate::patterns::Charset::Alphanumeric,
            },
            PrefixValidationPattern {
                name: "test2",
                prefix: "AK",
                tier: crate::patterns::PatternTier::Critical,
                min_len: 20,
                max_len: 200,
                charset: crate::patterns::Charset::Alphanumeric,
            },
        ];

        let trie = PatternTrie::new(&patterns);
        
        // Test matching
        let text = b"AKIA1234567890ABCDEF";
        let matches = trie.find_matches_at(text, 0);
        assert!(!matches.is_empty(), "Should find patterns at position 0");
    }

    #[test]
    fn test_full_pattern_trie() {
        let trie = PatternTrie::new(PREFIX_VALIDATION_PATTERNS);
        
        // Test with AWS key
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let matches = trie.find_matches_at(text, 0);
        assert!(!matches.is_empty(), "Should find AWS pattern");
    }
}
