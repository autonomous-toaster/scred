//! Vectorized pattern matching - Process multiple patterns per memchr result
//!
//! Strategy: When memchr finds a byte, check all patterns that start with that byte
//! using batched comparison instead of individual pattern checks.

use crate::patterns::PrefixValidationPattern;

/// Group patterns that share the same starting byte for batch processing
#[derive(Clone)]
pub struct BytePatternGroup {
    pub start_byte: u8,
    pub pattern_indices: Vec<usize>,
}

impl BytePatternGroup {
    /// Build groups organized by starting byte
    pub fn organize_by_start_byte(patterns: &[PrefixValidationPattern]) -> Vec<BytePatternGroup> {
        let mut groups: Vec<Option<BytePatternGroup>> = vec![None; 256];

        for (idx, pattern) in patterns.iter().enumerate() {
            if let Some(first_byte) = pattern.prefix.as_bytes().first() {
                let byte_idx = *first_byte as usize;
                if let Some(ref mut group) = groups[byte_idx] {
                    group.pattern_indices.push(idx);
                } else {
                    groups[byte_idx] = Some(BytePatternGroup {
                        start_byte: *first_byte,
                        pattern_indices: vec![idx],
                    });
                }
            }
        }

        groups.into_iter().flatten().collect()
    }

    /// Fast prefix matching for all patterns in this group
    /// This is where SIMD could theoretically help by comparing multiple prefixes
    /// in parallel, but empirically memchr is already highly optimized
    #[inline]
    pub fn match_patterns(&self, text: &[u8], pos: usize, patterns: &[PrefixValidationPattern]) -> Vec<usize> {
        let mut matches = Vec::new();

        for &idx in &self.pattern_indices {
            let pattern = &patterns[idx];
            let prefix_bytes = pattern.prefix.as_bytes();

            // Quick bounds check
            if pos + prefix_bytes.len() > text.len() {
                continue;
            }

            // Compare prefix
            if &text[pos..pos + prefix_bytes.len()] == prefix_bytes {
                matches.push(idx);
            }
        }

        matches
    }
}

/// SIMD-friendly pattern matching using memchr + batch prefix verification
pub fn vectorized_pattern_search(
    text: &[u8],
    patterns: &[PrefixValidationPattern],
) -> Vec<(usize, usize)> {
    let mut results = Vec::new();

    // Group patterns by starting byte for better cache locality
    let byte_groups = BytePatternGroup::organize_by_start_byte(patterns);

    // Create a lookup table for quick access to pattern groups
    let mut group_map: [Option<&BytePatternGroup>; 256] = [None; 256];
    for group in &byte_groups {
        group_map[group.start_byte as usize] = Some(group);
    }

    // Scan text looking for starting bytes
    for (pos, &byte) in text.iter().enumerate() {
        if let Some(Some(group)) = group_map.get(byte as usize) {
            // Found patterns starting with this byte - check them all
            for idx in group.match_patterns(text, pos, patterns) {
                results.push((pos, idx));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patterns::PREFIX_VALIDATION_PATTERNS;

    #[test]
    fn test_byte_group_organization() {
        let groups = BytePatternGroup::organize_by_start_byte(PREFIX_VALIDATION_PATTERNS);
        assert!(!groups.is_empty());
        
        // All patterns should be in some group
        let total: usize = groups.iter().map(|g| g.pattern_indices.len()).sum();
        assert_eq!(total, PREFIX_VALIDATION_PATTERNS.len());
    }

    #[test]
    fn test_vectorized_search() {
        let test_data = b"start AKIA1234567890ABCDEF1234567890AB end";
        let results = vectorized_pattern_search(test_data, PREFIX_VALIDATION_PATTERNS);
        
        // Should find AWS pattern
        assert!(!results.is_empty(), "Should find patterns");
    }
}
