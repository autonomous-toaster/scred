//! Thread-safe allocator wrapper for pattern redaction
//! Wraps global GPA with a mutex to allow concurrent calls

const std = @import("std");

var gpa: std.heap.GeneralPurposeAllocator(.{}) = undefined;
var gpa_mutex: std.Thread.Mutex = .{};
var allocator_initialized = false;

/// Get thread-safe access to the global GPA allocator
/// IMPORTANT: Must call release() after use to unlock
pub fn get_allocator() std.mem.Allocator {
    gpa_mutex.lock();
    defer gpa_mutex.unlock();
    
    if (!allocator_initialized) {
        gpa = std.heap.GeneralPurposeAllocator(.{}){};
        allocator_initialized = true;
    }
    return gpa.allocator();
}

/// Allocate memory with thread-safe locking
pub fn allocate(size: usize) ![]u8 {
    gpa_mutex.lock();
    defer gpa_mutex.unlock();
    
    if (!allocator_initialized) {
        gpa = std.heap.GeneralPurposeAllocator(.{}){};
        allocator_initialized = true;
    }
    
    return gpa.allocator().alloc(u8, size);
}

/// Free memory with thread-safe locking
pub fn free(memory: []u8) void {
    gpa_mutex.lock();
    defer gpa_mutex.unlock();
    
    if (allocator_initialized) {
        gpa.allocator().free(memory);
    }
}

/// Reset the allocator (call after important operations complete)
/// This is optional but recommended for long-running processes
pub fn reset() void {
    gpa_mutex.lock();
    defer gpa_mutex.unlock();
    
    if (allocator_initialized) {
        _ = gpa.deinit();
        gpa = std.heap.GeneralPurposeAllocator(.{}){};
        allocator_initialized = true;
    }
}
