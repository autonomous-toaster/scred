//! FFI-compatible match structure with metadata
const std = @import("std");

pub const MatchFFI = extern struct {
    start: usize,
    end: usize,
    pattern_type: u32,  // Index into pattern array
    // Note: pattern name kept in Zig, not transmitted (too expensive for FFI)
};

pub const RedactionResultFFI = extern struct {
    output: ?[*]u8,
    output_len: usize,
    matches: ?[*]MatchFFI,       // Array of matches
    match_count: u32,            // Number of matches
    error_code: u32,             // 0 = success, >0 = error
};

/// Allocate and return match array for FFI consumption
/// Caller (Rust) is responsible for freeing
pub fn allocate_matches(
    matches: []const MatchFFI,
    allocator: std.mem.Allocator,
) !?[*]MatchFFI {
    if (matches.len == 0) {
        return null;
    }
    
    const allocated = try allocator.dupe(MatchFFI, matches);
    return allocated.ptr;
}

/// Free match array allocated for FFI
pub fn free_matches(
    matches: ?[*]MatchFFI,
    match_count: u32,
    allocator: std.mem.Allocator,
) void {
    if (matches == null or match_count == 0) return;
    
    const slice = matches.?[0..match_count];
    allocator.free(slice);
}

/// Get pattern name from match type (for logging/debugging)
pub fn get_pattern_name(pattern_type: u32, context: anytype) []const u8 {
    if (pattern_type < 36) {
        // SIMPLE_PREFIX_PATTERNS
        if (pattern_type < context.SIMPLE_PREFIX_PATTERNS.len) {
            return context.SIMPLE_PREFIX_PATTERNS[pattern_type].name;
        }
    } else if (pattern_type < 100) {
        // JWT
        return "jwt";
    } else if (pattern_type < 200) {
        // PREFIX_VALIDATION_PATTERNS
        const idx = pattern_type - 100;
        if (idx < context.PREFIX_VALIDATION_PATTERNS.len) {
            return context.PREFIX_VALIDATION_PATTERNS[idx].name;
        }
    }
    return "unknown";
}
