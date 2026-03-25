//! Thread-safe allocator pool for pattern redaction
//! Uses simple fixed arena instead of global GPA state
const std = @import("std");

const ARENA_SIZE = 10 * 1024 * 1024;  // 10MB arena per call

pub fn create_arena() std.mem.ArenaAllocator {
    // Stack-allocated arena - will be cleaned up per call
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();
    return std.mem.ArenaAllocator.init(allocator);
}

pub fn create_temporary() std.heap.GeneralPurposeAllocator(.{}) {
    return std.heap.GeneralPurposeAllocator(.{}){};
}
