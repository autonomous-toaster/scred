const std = @import("std");
const redaction_impl = @import("redaction_impl.zig");
const redaction_ffi = @import("redaction_ffi.zig");
const detectors = @import("detectors.zig");

var gpa: std.heap.GeneralPurposeAllocator(.{}) = undefined;
var allocator_initialized = false;

fn get_allocator() std.mem.Allocator {
    if (!allocator_initialized) {
        gpa = std.heap.GeneralPurposeAllocator(.{}){};
        allocator_initialized = true;
    }
    return gpa.allocator();
}

pub const RedactionResultFFI = redaction_ffi.RedactionResultFFI;

/// Redact text by finding patterns and replacing them
/// Returns full metadata including pattern type for each match
pub fn scred_redact_text_optimized_stub(
    text: [*]const u8,
    text_len: usize,
) RedactionResultFFI {
    if (text_len == 0) {
        return RedactionResultFFI{
            .output = null,
            .output_len = 0,
            .matches = null,
            .match_count = 0,
            .error_code = 0,
        };
    }

    const allocator = get_allocator();
    const text_slice = text[0..text_len];

    // Find all pattern matches in the text
    const matches = redaction_impl.find_all_matches(text_slice, allocator) catch {
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

    // If no patterns found, return copy of input with no matches
    if (matches.match_count == 0) {
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
    const redacted = redaction_impl.redact_text(text_slice, matches.matches, allocator) catch {
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

    // Allocate matches array for FFI (caller will free)
    const ffi_matches = redaction_ffi.allocate_matches(matches.matches, allocator) catch {
        allocator.free(redacted);
        allocator.free(matches.matches);
        return RedactionResultFFI{
            .output = null,
            .output_len = 0,
            .matches = null,
            .match_count = 0,
            .error_code = 1,
        };
    };

    allocator.free(matches.matches);  // Free original, we have a copy in ffi_matches

    return RedactionResultFFI{
        .output = redacted.ptr,
        .output_len = redacted.len,
        .matches = ffi_matches,
        .match_count = @intCast(matches.match_count),
        .error_code = 0,
    };
}

/// Free redaction result buffer and matches array
pub fn scred_free_redaction_result_stub(result: RedactionResultFFI) void {
    const allocator = get_allocator();

    // Free output
    if (result.output != null and result.output_len > 0) {
        const slice = result.output.?[0..result.output_len];
        allocator.free(slice);
    }

    // Free matches
    if (result.matches != null and result.match_count > 0) {
        redaction_ffi.free_matches(result.matches, result.match_count, allocator);
    }
}
