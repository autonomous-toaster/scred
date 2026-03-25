const std = @import("std");

pub const RedactionResultFFI = extern struct {
    output: ?[*]u8,
    output_len: usize,
    match_count: u32,
};

/// Minimal redaction function for FFI testing
/// Returns the input text as-is (no redaction yet)
pub fn scred_redact_text_optimized_stub(
    text: [*]const u8,
    text_len: usize,
) RedactionResultFFI {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const text_slice = text[0..text_len];
    
    // Allocate output buffer and copy input
    const output = allocator.dupe(u8, text_slice) catch {
        return RedactionResultFFI{
            .output = null,
            .output_len = 0,
            .match_count = 0,
        };
    };

    return RedactionResultFFI{
        .output = output.ptr,
        .output_len = output.len,
        .match_count = 0,
    };
}

/// Free redaction result buffer
pub fn scred_free_redaction_result_stub(result: RedactionResultFFI) void {
    if (result.output == null or result.output_len == 0) return;

    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const slice = result.output.?[0..result.output_len];
    allocator.free(slice);
}
