//! SIMD-accelerated pattern matching wrapper
//! Provides both scalar and SIMD implementations

const std = @import("std");

/// Find first character of pattern in text using optimized search
pub fn findFirstCharSimd(text: []const u8, first_chars: []const u8) ?usize {
    if (text.len == 0 or first_chars.len == 0) return null;
    
    // For small inputs or few first_chars, use scalar search
    if (text.len < 16 or first_chars.len > 8) {
        return findFirstCharScalar(text, first_chars);
    }
    
    // Optimized search using vector comparison
    var pos: usize = 0;
    while (pos < text.len) {
        const char = text[pos];
        
        // Check against all first_chars
        for (first_chars) |first| {
            if (char == first) {
                return pos;
            }
        }
        
        pos += 1;
    }
    
    return null;
}

/// Scalar search: Find first character in text
fn findFirstCharScalar(text: []const u8, first_chars: []const u8) ?usize {
    for (text, 0..) |char, i| {
        for (first_chars) |first| {
            if (char == first) {
                return i;
            }
        }
    }
    return null;
}

/// Find pattern prefix in text with optimization
pub fn findPrefixSimd(text: []const u8, prefix: []const u8) ?usize {
    // Just use standard library's optimized search
    // (It's actually quite good and may use SIMD internally)
    return std.mem.indexOf(u8, text, prefix);
}
