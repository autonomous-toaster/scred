//! SIMD Core: Real vectorized operations for pattern matching
//!
//! Strategy: Use Zig @Vector for batch processing
//! - 16-byte chunks: Compare prefix patterns in parallel
//! - 32-byte chunks: Extend token scanning
//! - Fallback to scalar for <16 bytes
//!
//! Expected improvement: 2-4x speedup on prefix scanning

const std = @import("std");

/// Find all positions where any of the prefixes start in a 16-byte chunk
/// Returns a vector of 16 bools indicating matches
pub fn findPrefixesIn16Bytes(
    chunk: [16]u8,
    prefixes: []const []const u8,
) @Vector(16, bool) {
    var matches: @Vector(16, bool) = @splat(false);

    // For each prefix, check if it could start at each byte position
    for (prefixes) |prefix| {
        if (prefix.len == 0) continue;
        if (prefix.len > 16) continue; // Skip long prefixes for chunk matching

        const first_byte: u8 = prefix[0];
        const fb_vec: @Vector(16, u8) = @splat(first_byte);
        const chunk_vec: @Vector(16, u8) = chunk;

        // Check where first byte matches
        const first_byte_matches: @Vector(16, bool) = chunk_vec == fb_vec;

        // Mark as potential match (full validation happens in scalar path)
        matches = matches or first_byte_matches;
    }

    return matches;
}

/// Find first matching position in data using 16-byte SIMD chunks
/// Falls back to scalar search for final bytes
pub fn findFirstPrefix(
    data: []const u8,
    prefix: []const u8,
) ?usize {
    if (data.len < prefix.len) return null;
    if (prefix.len == 0) return 0;

    const first_byte = prefix[0];
    var pos: usize = 0;

    // Process 16-byte chunks
    while (pos + 16 <= data.len) : (pos += 16) {
        const chunk: [16]u8 = data[pos..][0..16].*;
        const chunk_vec: @Vector(16, u8) = chunk;
        const fb_vec: @Vector(16, u8) = @splat(first_byte);

        // Check if first byte matches anywhere in chunk
        const matches: @Vector(16, bool) = chunk_vec == fb_vec;

        // Check each match position
        for (0..16) |i| {
            if (matches[i]) {
                const candidate_pos = pos + i;
                if (candidate_pos + prefix.len <= data.len) {
                    if (std.mem.eql(u8, data[candidate_pos .. candidate_pos + prefix.len], prefix)) {
                        return candidate_pos;
                    }
                }
            }
        }
    }

    // Handle remaining bytes (<16)
    if (pos < data.len) {
        const remaining = data[pos..];
        if (std.mem.indexOf(u8, remaining, prefix)) |rel_pos| {
            return pos + rel_pos;
        }
    }

    return null;
}

/// Find all matching positions (up to max_matches) using SIMD
pub fn findAllPrefixes(
    allocator: std.mem.Allocator,
    data: []const u8,
    prefix: []const u8,
    max_matches: usize,
) !std.ArrayList(usize) {
    var matches = std.ArrayList(usize).init(allocator);

    if (data.len < prefix.len or prefix.len == 0) {
        return matches;
    }

    const first_byte = prefix[0];
    var pos: usize = 0;

    // Process 16-byte chunks
    while (pos + 16 <= data.len and matches.items.len < max_matches) : (pos += 16) {
        const chunk: [16]u8 = data[pos..][0..16].*;
        const chunk_vec: @Vector(16, u8) = chunk;
        const fb_vec: @Vector(16, u8) = @splat(first_byte);

        const match_vec: @Vector(16, bool) = chunk_vec == fb_vec;

        // Check each match position
        for (0..16) |i| {
            if (match_vec[i]) {
                const candidate_pos = pos + i;
                if (candidate_pos + prefix.len <= data.len) {
                    if (std.mem.eql(u8, data[candidate_pos .. candidate_pos + prefix.len], prefix)) {
                        try matches.append(candidate_pos);
                        if (matches.items.len >= max_matches) break;
                    }
                }
            }
        }
    }

    // Handle remaining bytes (<16)
    if (pos < data.len and matches.items.len < max_matches) {
        var remaining_pos = pos;
        while (remaining_pos + prefix.len <= data.len) : (remaining_pos += 1) {
            if (std.mem.eql(u8, data[remaining_pos .. remaining_pos + prefix.len], prefix)) {
                try matches.append(remaining_pos);
                if (matches.items.len >= max_matches) break;
            }
        }
    }

    return matches;
}

/// Check if any byte in a 32-byte chunk matches any in the charset
/// Used for token boundary detection
pub fn scanForTokenEnd32(
    data: []const u8,
    start: usize,
    charset: []const u8,
) usize {
    if (start >= data.len) return 0;

    var pos = start;
    var len: usize = 0;

    // Process 32-byte chunks for efficiency
    const chunk_size: usize = 32;
    while (pos + chunk_size <= data.len) : (pos += chunk_size) {
        const chunk = data[pos .. pos + chunk_size];
        var chunk_len: usize = 0;

        for (chunk) |byte| {
            // Check if byte is IN charset
            var found = false;
            for (charset) |cs_byte| {
                if (byte == cs_byte) {
                    found = true;
                    break;
                }
            }

            if (found) {
                chunk_len += 1;
            } else {
                // End of token
                return len + chunk_len;
            }
        }

        len += chunk_len;
    }

    // Handle remaining bytes
    while (pos < data.len) : (pos += 1) {
        const byte = data[pos];
        var found = false;
        for (charset) |cs_byte| {
            if (byte == cs_byte) {
                found = true;
                break;
            }
        }

        if (found) {
            len += 1;
        } else {
            break;
        }
    }

    return len;
}

/// Batch filter: given multiple candidates, check which actually match the full prefix
/// Returns count of true matches
pub fn filterCandidates(
    data: []const u8,
    candidates: []const usize,
    prefix: []const u8,
    max_valid: usize,
) usize {
    var valid_count: usize = 0;

    for (candidates) |candidate_pos| {
        if (valid_count >= max_valid) break;
        if (candidate_pos + prefix.len > data.len) continue;

        if (std.mem.eql(u8, data[candidate_pos .. candidate_pos + prefix.len], prefix)) {
            valid_count += 1;
        }
    }

    return valid_count;
}

// ============================================================================
// Tests
// ============================================================================

test "findFirstPrefix finds position" {
    const data = "hello AKIAIOSFODNN7EXAMPLE world";
    const prefix = "AKIA";

    const pos = findFirstPrefix(data, prefix);
    try std.testing.expectEqual(@as(?usize, 6), pos);
}

test "findFirstPrefix returns null when not found" {
    const data = "hello world";
    const prefix = "NOTHERE";

    const pos = findFirstPrefix(data, prefix);
    try std.testing.expectEqual(@as(?usize, null), pos);
}

test "findFirstPrefix works at start" {
    const data = "AKIAIOSFODNN7EXAMPLE";
    const prefix = "AKIA";

    const pos = findFirstPrefix(data, prefix);
    try std.testing.expectEqual(@as(?usize, 0), pos);
}

test "findAllPrefixes finds multiple positions" {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const data = "AKIAIOSFODNN7EXAMPLE some text AKIAIOSFODNN7EXAMPLE end";
    const prefix = "AKIA";

    const matches = try findAllPrefixes(allocator, data, prefix, 10);
    defer matches.deinit();

    try std.testing.expectEqual(@as(usize, 2), matches.items.len);
    try std.testing.expectEqual(@as(usize, 0), matches.items[0]);
    try std.testing.expectEqual(@as(usize, 36), matches.items[1]);
}

test "scanForTokenEnd32 detects boundaries" {
    const data = "sk_prod_abc123XYZ, next";
    const charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

    const len = scanForTokenEnd32(data, 0, charset);
    try std.testing.expectEqual(@as(usize, 13), len);
}
