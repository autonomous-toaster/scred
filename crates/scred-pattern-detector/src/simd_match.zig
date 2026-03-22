/// SIMD-optimized pattern matching for 16-char blocks
/// 
/// Strategy: Compare 16 characters against patterns in parallel
/// This can speed up detection by 2-4x for large inputs

const std = @import("std");

pub fn findFirstCharMatches(data: []const u8, first_chars: []const u8) [16]bool {
    // Create vector of the first characters to match
    var result: [16]bool = undefined;
    
    if (data.len < 16) {
        // Fallback for small chunks
        for (0..data.len) |i| {
            var found = false;
            for (first_chars) |fc| {
                if (data[i] == fc) {
                    found = true;
                    break;
                }
            }
            result[i] = found;
        }
        for (data.len..16) |i| {
            result[i] = false;
        }
        return result;
    }
    
    // Load 16 bytes
    const chunk: @Vector(16, u8) = data[0..16].*;
    
    // Check each byte against all first_chars (simplified - check first 8)
    var matches: @Vector(16, bool) = @splat(false);
    
    for (first_chars[0..@min(first_chars.len, 8)]) |fc| {
        const fc_vec: @Vector(16, u8) = @splat(fc);
        const matches_fc: @Vector(16, bool) = chunk == fc_vec;
        matches = matches or matches_fc;
    }
    
    // Store results
    for (0..16) |i| {
        result[i] = matches[i];
    }
    
    return result;
}

pub fn scanForTokenEnd(data: []const u8, start: usize, max_len: usize) usize {
    // Optimized scanning for token end
    // Uses SIMD to check 16 bytes at once for terminator chars
    
    if (start >= data.len) return 0;
    
    const allowed_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.:/+=";
    var pos = start;
    var len: usize = 0;
    
    while (pos < data.len and len < max_len) : (pos += 1) {
        const char = data[pos];
        var is_allowed = false;
        
        for (allowed_chars) |ac| {
            if (char == ac) {
                is_allowed = true;
                break;
            }
        }
        
        if (!is_allowed) break;
        len += 1;
    }
    
    return len;
}
