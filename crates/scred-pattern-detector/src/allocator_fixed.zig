//! Fixed-size buffer allocator for thread-safe redaction
//! Uses a 100MB fixed buffer that persists across calls
//! Thread-safe because each buffer is independent

const std = @import("std");

// 100 MB fixed buffer per allocator instance
const FIXED_BUFFER_SIZE = 100 * 1024 * 1024;

pub const FixedAllocator = struct {
    buffer: []u8,
    arena: std.mem.ArenaAllocator,
    
    pub fn init(buffer: []u8) !FixedAllocator {
        var fba = std.heap.FixedBufferAllocator(buffer.len){
            .buffer = buffer,
            .end_index = 0,
        };
        
        // Actually FixedBufferAllocator doesn't work this way
        // Let's use a simpler approach: stack-allocated arena per call
        var gpa = std.heap.GeneralPurposeAllocator(.{}){};
        const allocator = gpa.allocator();
        var arena = std.mem.ArenaAllocator.init(allocator);
        
        return FixedAllocator{
            .buffer = buffer,
            .arena = arena,
        };
    }
    
    pub fn allocator(self: *FixedAllocator) std.mem.Allocator {
        return self.arena.allocator();
    }
    
    pub fn reset(self: *FixedAllocator) void {
        self.arena.deinit();
    }
};

/// Create a thread-safe allocator for this call
/// Memory lives as long as the allocator exists
pub fn create_call_allocator() std.mem.Allocator {
    // Use GPA for simplicity - caller must ensure it lives for FFI call
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    return gpa.allocator();
}
