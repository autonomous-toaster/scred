//! Redaction Implementation - Uses pattern detection to find and redact secrets
const std = @import("std");
const detectors = @import("detectors.zig");
const patterns = @import("patterns.zig");
const redaction_ffi = @import("redaction_ffi.zig");
const validation = @import("validation.zig");
const simd_core = @import("simd_core.zig");

pub const Match = redaction_ffi.MatchFFI;  // Alias for clarity



pub const MAX_MATCHES = 1000;

pub const RedactionResult = struct {
    output: []u8,
    matches: []redaction_ffi.MatchFFI,
    match_count: usize,
};

/// Find all matches in text using pattern detection
pub fn find_all_matches(
    text: []const u8,
    allocator: std.mem.Allocator,
) !RedactionResult {
    var matches_buf: [MAX_MATCHES]Match = undefined;
    var match_count: usize = 0;

    // Check simple prefix patterns first (fast path with SIMD)
    for (patterns.SIMPLE_PREFIX_PATTERNS, 0..) |prefix_pattern, idx| {
        if (match_count >= MAX_MATCHES) break;

        var search_pos: usize = 0;
        while (search_pos < text.len and match_count < MAX_MATCHES) {
            // Use SIMD-accelerated search via simd_core
            if (simd_core.findFirstPrefix(text[search_pos..], prefix_pattern.prefix)) |match_pos| {
                const absolute_pos = search_pos + match_pos;
                const end_pos = @min(absolute_pos + prefix_pattern.prefix.len + 20, text.len);

                matches_buf[match_count] = Match{
                    .start = absolute_pos,
                    .end = end_pos,
                    .pattern_type = @intCast(idx),
                };
                match_count += 1;

                search_pos = absolute_pos + prefix_pattern.prefix.len;
            } else {
                break;
            }
        }
    }

    // Check prefix validation patterns (with proper validation)
    for (patterns.PREFIX_VALIDATION_PATTERNS, 0..) |pattern, idx| {
        if (match_count >= MAX_MATCHES) break;

        var search_pos: usize = 0;
        while (search_pos < text.len and match_count < MAX_MATCHES) {
            if (std.mem.indexOf(u8, text[search_pos..], pattern.prefix)) |match_pos| {
                const absolute_pos = search_pos + match_pos;
                const token_start = absolute_pos + pattern.prefix.len;
                
                // Scan token end using validation charset
                const token_len = validation.scanTokenEnd(
                    text,
                    token_start,
                    pattern.charset,
                    pattern.max_len
                );
                
                // Validate token meets length requirements
                if (token_len > 0 and validation.validateLength(text[token_start..@min(token_start + token_len, text.len)], pattern.min_len, pattern.max_len)) {
                    const end_pos = @min(token_start + token_len, text.len);
                    
                    matches_buf[match_count] = Match{
                        .start = absolute_pos,
                        .end = end_pos,
                        .pattern_type = @intCast(100 + idx),
                    };
                    match_count += 1;
                }

                search_pos = absolute_pos + pattern.prefix.len;
            } else {
                break;
            }
        }
    }

    // Check JWT patterns
    if (match_count < MAX_MATCHES and detectors.detect_jwt(text)) {
        var i: usize = 0;
        while (i + 3 <= text.len and match_count < MAX_MATCHES) {
            if (text[i] == 'e' and text[i + 1] == 'y' and text[i + 2] == 'J') {
                // Find JWT end
                var end = i + 3;
                while (end < text.len and text[end] != ' ' and text[end] != '\n' and text[end] != '\t') {
                    end += 1;
                    if (end - i > 10000) break;
                }

                matches_buf[match_count] = Match{
                    .start = i,
                    .end = end,
                    .pattern_type = 200,
                };
                match_count += 1;

                i = end;
            } else {
                i += 1;
            }
        }
    }

    const matches_slice = try allocator.dupe(Match, matches_buf[0..match_count]);
    return RedactionResult{
        .output = &[_]u8{},
        .matches = matches_slice,
        .match_count = match_count,
    };
}

/// Redact text by replacing matched patterns with X's
pub fn redact_text(
    text: []const u8,
    matches: []const Match,
    allocator: std.mem.Allocator,
) ![]u8 {
    var output = try allocator.alloc(u8, text.len);

    // Copy text as-is initially
    @memcpy(output, text);

    // Sort matches by start position (descending) to avoid offset issues
    var sorted_matches = try allocator.dupe(Match, matches);

    // Simple bubble sort to avoid comparator issues
    for (0..sorted_matches.len) |i| {
        for (i + 1..sorted_matches.len) |j| {
            if (sorted_matches[i].start < sorted_matches[j].start) {
                const temp = sorted_matches[i];
                sorted_matches[i] = sorted_matches[j];
                sorted_matches[j] = temp;
            }
        }
    }

    // Redact each match
    for (sorted_matches) |match_item| {
        if (match_item.end <= output.len) {
            const keep = @min(4, match_item.end - match_item.start);
            for (match_item.start + keep..match_item.end) |pos| {
                output[pos] = 'x';
            }
        }
    }

    allocator.free(sorted_matches);
    return output;
}
