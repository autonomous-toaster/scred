//! SIMD Core: Fast prefix search with portable SIMD for charset validation
//! 
//! Strategy (matching Zig optimization patterns):
//! - Use memchr for single-byte matching (2-4x faster than manual loop)
//! - Process in chunks (16-32 bytes) for batch operations
//! - Charset validation using lookup tables (not regex)
//! - SIMD for candidate filtering (portable via slice iterators)
//! - Fallback to memchr for multi-byte prefixes

use memchr::memchr;

/// Portable SIMD-style charset validation via lookup table
/// This is faster than scanning every byte
#[derive(Clone, Copy)]
pub struct CharsetLut {
    pub table: [bool; 256],  // ← Now public for SIMD access
}

impl CharsetLut {
    /// Create a lookup table for a charset
    pub fn new(charset: &[u8]) -> Self {
        let mut table = [false; 256];
        for &byte in charset {
            table[byte as usize] = true;
        }
        CharsetLut { table }
    }

    #[inline]
    pub fn contains(&self, byte: u8) -> bool {
        self.table[byte as usize]
    }

    /// Scan data for end of token (first byte NOT in charset)
    /// Zig optimization: scanForTokenEnd32 but portable
    /// Optimized: Uses SIMD when available, falls back to scalar
    #[inline]
    pub fn scan_token_end(&self, data: &[u8], start: usize) -> usize {
        crate::simd_charset::scan_token_end_fast(data, self, start)
    }
}

/// Find first occurrence of prefix in data
/// Uses memchr for first byte, then validates full prefix
#[inline]
pub fn find_first_prefix(data: &[u8], prefix: &[u8]) -> Option<usize> {
    if data.is_empty() || prefix.is_empty() {
        return if prefix.is_empty() { Some(0) } else { None };
    }

    if prefix.len() > data.len() {
        return None;
    }

    let first_byte = prefix[0];

    // Fast path: single-byte prefix
    if prefix.len() == 1 {
        return memchr(first_byte, data);
    }

    // Multi-byte prefix: use memchr to find candidates, then validate
    let mut search_start = 0;
    while let Some(pos) = memchr(first_byte, &data[search_start..]) {
        let absolute_pos = search_start + pos;
        
        // Check if we have enough bytes for full prefix
        if absolute_pos + prefix.len() <= data.len() {
            // Validate full prefix at this position
            if &data[absolute_pos..absolute_pos + prefix.len()] == prefix {
                return Some(absolute_pos);
            }
        }
        
        // Move search forward
        search_start = absolute_pos + 1;
    }

    None
}

/// Find all occurrences of prefix in data (up to max_matches)
pub fn find_all_prefixes(data: &[u8], prefix: &[u8], max_matches: usize) -> Vec<usize> {
    let mut matches = Vec::with_capacity(max_matches.min(10));

    if data.is_empty() || prefix.is_empty() || prefix.len() > data.len() {
        return matches;
    }

    if max_matches == 0 {
        return matches;
    }

    let first_byte = prefix[0];
    let mut search_start = 0;

    while let Some(pos) = memchr(first_byte, &data[search_start..]) {
        if matches.len() >= max_matches {
            break;
        }

        let absolute_pos = search_start + pos;

        // Check if we have enough bytes
        if absolute_pos + prefix.len() <= data.len() {
            // Validate full prefix
            if &data[absolute_pos..absolute_pos + prefix.len()] == prefix {
                matches.push(absolute_pos);
            }
        }

        // Continue search from next byte
        search_start = absolute_pos + 1;
    }

    matches
}

/// Validate token length and charset (Zig optimization pattern)
pub fn validate_token(
    data: &[u8],
    start: usize,
    min_len: usize,
    max_len: usize,
    charset: &CharsetLut,
) -> Option<(usize, usize)> {
    if start >= data.len() {
        return None;
    }

    // Scan token end using charset LUT (O(n) where n is token len)
    let token_len = charset.scan_token_end(data, start);

    // Check constraints
    if token_len < min_len {
        return None;
    }

    if max_len > 0 && token_len > max_len {
        return None;
    }

    Some((start, start + token_len))
}

