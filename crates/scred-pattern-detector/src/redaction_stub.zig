const std = @import("std");
const redaction_impl = @import("redaction_impl.zig");
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

pub const RedactionResultFFI = extern struct {
    output: ?[*]u8,
    output_len: usize,
    match_count: u32,
};

/// Redact text by finding patterns and replacing them
/// This is the main FFI entry point for redaction
pub fn scred_redact_text_optimized_stub(
    text: [*]const u8,
    text_len: usize,
) RedactionResultFFI {
    if (text_len == 0) {
        return RedactionResultFFI{
            .output = null,
            .output_len = 0,
            .match_count = 0,
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
                .match_count = 0,
            };
        };

        return RedactionResultFFI{
            .output = output.ptr,
            .output_len = output.len,
            .match_count = 0,
        };
    };

    // If no patterns found, return copy of input
    if (matches.match_count == 0) {
        const output = allocator.dupe(u8, text_slice) catch {
            allocator.free(matches.matches);
            return RedactionResultFFI{
                .output = null,
                .output_len = 0,
                .match_count = 0,
            };
        };

        allocator.free(matches.matches);
        return RedactionResultFFI{
            .output = output.ptr,
            .output_len = output.len,
            .match_count = @intCast(matches.match_count),
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
                .match_count = 0,
            };
        };

        allocator.free(matches.matches);
        return RedactionResultFFI{
            .output = output.ptr,
            .output_len = output.len,
            .match_count = 0,
        };
    };

    allocator.free(matches.matches);
    return RedactionResultFFI{
        .output = redacted.ptr,
        .output_len = redacted.len,
        .match_count = @intCast(matches.match_count),
    };
}

/// Free redaction result buffer
pub fn scred_free_redaction_result_stub(result: RedactionResultFFI) void {
    if (result.output == null or result.output_len == 0) return;

    const allocator = get_allocator();
    const slice = result.output.?[0..result.output_len];
    allocator.free(slice);
}
