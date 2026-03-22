/// Fast regex implementation for common secret patterns
/// No external dependencies - pure Zig
/// 
/// Strategy:
/// 1. For each chunk, detect content type (HTTP, JSON, env, etc.)
/// 2. Apply only relevant patterns (10-40 instead of 198)
/// 3. Use optimized character matching for each pattern family

const std = @import("std");

pub const Pattern = struct {
    name: []const u8,
    // Simple character class matching: [0-9a-zA-Z], [0-9a-f], etc.
    // No full regex - just fast checks
    prefix: []const u8,
    allowed_chars: []const u8,  // e.g. "0-9a-zA-Z_-" 
    min_len: usize,
    max_len: usize,
};

pub fn isCharInClass(char: u8, class: []const u8) bool {
    var i: usize = 0;
    while (i < class.len) : (i += 1) {
        if (class[i] == char) return true;
        // Handle ranges like "0-9"
        if (i + 2 < class.len and class[i + 1] == '-') {
            if (char >= class[i] and char <= class[i + 2]) return true;
            i += 2;
        }
    }
    return false;
}

pub fn matchPattern(input: []const u8, pos: usize, pattern: Pattern) ?usize {
    if (pos >= input.len) return null;
    
    // Check prefix
    if (!std.mem.startsWith(u8, input[pos..], pattern.prefix)) {
        return null;
    }
    
    var token_len = pattern.prefix.len;
    var scan_pos = pos + pattern.prefix.len;
    
    // Scan for allowed characters
    while (scan_pos < input.len and token_len < pattern.max_len) {
        const char = input[scan_pos];
        if (!isCharInClass(char, pattern.allowed_chars)) {
            break;
        }
        token_len += 1;
        scan_pos += 1;
    }
    
    // Check length constraints
    if (token_len >= pattern.min_len) {
        return token_len;
    }
    
    return null;
}
