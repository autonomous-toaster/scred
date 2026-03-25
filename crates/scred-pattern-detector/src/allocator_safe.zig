//! Thread-safe allocator wrapper for pattern redaction
//! SIMPLIFIED: Removed mutex for debugging

const std = @import("std");

var gpa: std.heap.GeneralPurposeAllocator(.{}) = undefined;
var allocator_initialized = false;

/// Get allocator (no mutex for debugging)
pub fn get_allocator() std.mem.Allocator {
    if (!allocator_initialized) {
        std.debug.print("[ALLOC] Initializing GPA\n", .{});
        gpa = std.heap.GeneralPurposeAllocator(.{}){};
        allocator_initialized = true;
        std.debug.print("[ALLOC] GPA initialized\n", .{});
    }
    std.debug.print("[ALLOC] Returning allocator\n", .{});
    return gpa.allocator();
}

/// Allocate memory
pub fn allocate(size: usize) ![]u8 {
    if (!allocator_initialized) {
        gpa = std.heap.GeneralPurposeAllocator(.{}){};
        allocator_initialized = true;
    }
    return gpa.allocator().alloc(u8, size);
}

/// Free memory
pub fn free(memory: []u8) void {
    if (allocator_initialized) {
        gpa.allocator().free(memory);
    }
}

/// Reset the allocator
pub fn reset() void {
    if (allocator_initialized) {
        _ = gpa.deinit();
        gpa = std.heap.GeneralPurposeAllocator(.{}){};
        allocator_initialized = true;
    }
}
