const std = @import("std");
const redaction_ffi = @import("redaction_ffi.zig");

pub const RedactionResultFFI = redaction_ffi.RedactionResultFFI;

/// MINIMAL TEST - just return a hardcoded result
pub fn scred_redact_text_optimized_stub(
    text: [*]const u8,
    text_len: usize,
) RedactionResultFFI {
    // Don't even allocate - just return null
    _ = text;
    _ = text_len;
    return .{
        .output = null,
        .output_len = 0,
        .matches = null,
        .match_count = 0,
        .error_code = 0,
    };
}

/// Free redaction result buffer and matches array
pub fn scred_free_redaction_result_stub(result: RedactionResultFFI) void {
    _ = result;
}

