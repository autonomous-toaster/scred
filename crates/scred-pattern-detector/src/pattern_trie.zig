/// Fast prefix lookup using character-based indexing
/// 
/// Instead of checking "does input[pos:pos+len] match pattern?"
/// Use first character as quick filter: input[pos] != prefix[0]? skip.

const std = @import("std");

pub fn buildFirstCharMap(patterns: []const struct { name: []const u8, prefix: []const u8, min_len: usize }) ![256]std.ArrayList(usize) {
    var map: [256]std.ArrayList(usize) = undefined;
    for (&map) |*list| {
        list.* = std.ArrayList(usize).init(std.heap.page_allocator);
    }
    
    for (patterns, 0..) |pattern, idx| {
        if (pattern.prefix.len > 0) {
            const first_char = pattern.prefix[0];
            try map[first_char].append(idx);
        }
    }
    
    return map;
}

pub fn getCandidatePatterns(first_char_map: [256]std.ArrayList(usize), char: u8) []usize {
    return first_char_map[char].items;
}
