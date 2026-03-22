const std = @import("std");
const lib = @import("lib.zig");

// ============================================================================
// Environment Variable Aware Redaction
// ============================================================================
// Detects and redacts environment variable formats:
// - KEY=VALUE format: redacts VALUE if KEY looks like a secret
// - Multiline key=value (AWS config style): redacts values
// - YAML-like format: key = value (with spaces)
//
// This is CLI-specific optimization to handle `env` output directly
// ============================================================================

pub const EnvRedactorConfig = struct {
    /// If true, redact any value after KEY= if KEY contains secret keywords
    enable_var_name_detection: bool = true,
    /// If true, also scan the value part for patterns
    also_scan_values: bool = true,
};

const SECRET_KEYWORDS = [_][]const u8{
    "KEY",
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "CREDENTIAL",
    "API",
    "ACCESS",
    "PRIVATE",
    "PASSPHRASE",
    "AWS",
    "AZURE",
    "GCP",
};

/// Check if a variable name looks like it contains a secret
fn isSecretVariableName(name: []const u8) bool {
    // Fast path: check for common keywords
    for (SECRET_KEYWORDS) |keyword| {
        if (std.mem.containsAtLeast(u8, name, 1, keyword)) {
            return true;
        }
    }
    return false;
}

/// Parse and redact a single environment variable line
/// Input: "KEY=VALUE" or "KEY = VALUE" (AWS config style)
/// Returns: redacted line
pub fn redactEnvLine(
    allocator: std.mem.Allocator,
    line: []const u8,
) ![]u8 {
    if (line.len == 0) return allocator.dupe(u8, line);
    
    // Find the separator (= or :)
    var sep_pos: ?usize = null;
    var sep_char: u8 = '=';
    
    for (0..line.len) |i| {
        if (line[i] == '=') {
            sep_pos = i;
            sep_char = '=';
            break;
        }
        // Also check for colon (for Key: Value format)
        if (i > 0 and line[i] == ':' and line[i - 1] != '/' and (i + 1 >= line.len or line[i + 1] != '/')) {
            sep_pos = i;
            sep_char = ':';
            break;
        }
    }
    
    // No separator found - just scan for patterns
    if (sep_pos == null) {
        return lib.redact_optimized(line);
    }
    
    const sep = sep_pos.?;
    const key = std.mem.trim(u8, line[0..sep], " \t");
    const value = std.mem.trim(u8, line[sep + 1..], " \t");
    
    // Uppercase key for comparison
    var key_upper_buf: [256]u8 = undefined;
    var key_len = key.len;
    if (key_len > 256) key_len = 256;  // truncate for comparison
    
    for (0..key_len) |i| {
        key_upper_buf[i] = std.ascii.toUpper(key[i]);
    }
    const key_upper = key_upper_buf[0..key_len];
    
    var result = std.ArrayList(u8).init(allocator);
    defer result.deinit();
    
    // Write key part as-is
    try result.appendSlice(key);
    try result.append(sep_char);
    
    // Check if key looks like a secret
    if (isSecretVariableName(key_upper)) {
        // Redact the entire value
        try result.appendNTimes('x', value.len);
    } else {
        // Scan value for patterns
        const redacted_value = lib.redact_optimized(value);
        try result.appendSlice(redacted_value);
    }
    
    return try result.toOwnedSlice();
}

/// Process stdin/stdout as environment variable lines
/// This intelligently detects KEY=VALUE and secret keywords
pub fn processEnvStream(
    allocator: std.mem.Allocator,
    input: []const u8,
    config: EnvRedactorConfig,
) ![]u8 {
    if (input.len == 0) return allocator.dupe(u8, "");
    
    var result = std.ArrayList(u8).init(allocator);
    defer result.deinit();
    
    var lines = std.mem.splitSequence(u8, input, "\n");
    var first_line = true;
    
    while (lines.next()) |line| {
        if (!first_line) {
            try result.append('\n');
        }
        first_line = false;
        
        if (config.enable_var_name_detection and std.mem.containsAtLeast(u8, line, 1, "=")) {
            const redacted = try redactEnvLine(allocator, line);
            defer allocator.free(redacted);
            try result.appendSlice(redacted);
        } else {
            // No KEY=VALUE format, just scan for patterns
            const redacted = lib.redact_optimized(line);
            try result.appendSlice(redacted);
        }
    }
    
    return try result.toOwnedSlice();
}

// ============================================================================
// C FFI for Rust bindings
// ============================================================================

pub export fn env_redactor_redact_line(
    line_ptr: [*]const u8,
    line_len: usize,
    output_ptr: [*]u8,
    output_len: usize,
) usize {
    const line = line_ptr[0..line_len];
    const output = output_ptr[0..output_len];
    
    // For now, use simple allocation
    // In real usage, this should use the same detector as the main flow
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    
    const redacted = lib.redact_optimized(line) catch return 0;
    
    if (redacted.len > output_len) return 0;
    @memcpy(output[0..redacted.len], redacted);
    
    return redacted.len;
}
