const std = @import("std");
const redaction_impl = @import("redaction_impl.zig");
const redaction_ffi = @import("redaction_ffi.zig");
const allocator_safe = @import("allocator_safe.zig");

pub const RedactionResultFFI = redaction_ffi.RedactionResultFFI;

/// Redact text by finding patterns and replacing them
/// Returns full metadata including pattern type for each match
/// IMPORTANT: Uses global GPA (not thread-safe, needs mutex for production)
pub fn scred_redact_text_optimized_stub(
    text: [*]const u8,
    text_len: usize,
) RedactionResultFFI {
    std.debug.print("[ZIG-FFI-1] called with {d} bytes\n", .{text_len});
    
    if (text_len == 0) {
        std.debug.print("[ZIG-FFI-1b] Empty input\n", .{});
        return RedactionResultFFI{
            .output = null,
            .output_len = 0,
            .matches = null,
            .match_count = 0,
            .error_code = 0,
        };
    }

    std.debug.print("[ZIG-FFI-2] Getting allocator\n", .{});
    const allocator = allocator_safe.get_allocator();
    const text_slice = text[0..text_len];

    // Find all pattern matches in the text
    std.debug.print("[ZIG-FFI-3] About to call find_all_matches\n", .{});
    const matches = redaction_impl.find_all_matches(text_slice, allocator) catch {
        std.debug.print("[ZIG-FFI-ERROR] find_all_matches failed\n", .{});
        // Fallback: just copy input if detection fails
        const output = allocator.dupe(u8, text_slice) catch {
            return RedactionResultFFI{
                .output = null,
                .output_len = 0,
                .matches = null,
                .match_count = 0,
                .error_code = 1,  // allocation error
            };
        };

        return RedactionResultFFI{
            .output = output.ptr,
            .output_len = output.len,
            .matches = null,
            .match_count = 0,
            .error_code = 2,  // detection error
        };
    };
    
    std.debug.print("[ZIG-FFI-4] find_all_matches returned {d} matches\n", .{matches.match_count});

    // If no patterns found, return copy of input with no matches
    if (matches.match_count == 0) {
        std.debug.print("[ZIG-FFI-5] No matches found\n", .{});
        const output = allocator.dupe(u8, text_slice) catch {
            allocator.free(matches.matches);
            return RedactionResultFFI{
                .output = null,
                .output_len = 0,
                .matches = null,
                .match_count = 0,
                .error_code = 1,
            };
        };

        allocator.free(matches.matches);
        return RedactionResultFFI{
            .output = output.ptr,
            .output_len = output.len,
            .matches = null,
            .match_count = 0,
            .error_code = 0,
        };
    }

    // Redact the matches
    std.debug.print("[ZIG-FFI-6] About to redact {d} matches\n", .{matches.match_count});
    const redacted = redaction_impl.redact_text(text_slice, matches.matches, allocator) catch {
        std.debug.print("[ZIG-FFI-ERROR] redact_text failed\n", .{});
        // Fallback: return original if redaction fails
        const output = allocator.dupe(u8, text_slice) catch {
            allocator.free(matches.matches);
            return RedactionResultFFI{
                .output = null,
                .output_len = 0,
                .matches = null,
                .match_count = 0,
                .error_code = 1,
            };
        };

        allocator.free(matches.matches);
        return RedactionResultFFI{
            .output = output.ptr,
            .output_len = output.len,
            .matches = null,
            .match_count = 0,
            .error_code = 3,  // redaction error
        };
    };

    // Keep matches array in memory for return to Rust
    // (Memory is in global GPA, remains valid after return)
    std.debug.print("[ZIG-FFI-7] Redaction complete\n", .{});
    const ffi_matches = if (matches.match_count > 0) matches.matches.ptr else null;

    return RedactionResultFFI{
        .output = redacted.ptr,
        .output_len = redacted.len,
        .matches = ffi_matches,
        .match_count = @intCast(matches.match_count),
        .error_code = 0,
    };
}

/// Free redaction result buffer and matches array
/// Note: Currently global GPA doesn't support selective freeing
/// TODO: Implement proper allocator management for production
pub fn scred_free_redaction_result_stub(result: RedactionResultFFI) void {
    // With global GPA, we can't free individual allocations
    // Just mark as consumed. Full cleanup would require reset_allocator()
    _ = result;
}
