const std = @import("std");
const Allocator = std.mem.Allocator;
pub const env_redactor = @import("env_redactor.zig");

// ============================================================================
// Import Pattern Definitions and Detectors
// ============================================================================

pub const patterns = @import("patterns.zig");
pub const detectors = @import("detectors.zig");

// Re-export pattern definitions for public API
pub const SIMPLE_PREFIX_PATTERNS = patterns.SIMPLE_PREFIX_PATTERNS;
pub const JWT_PATTERNS = patterns.JWT_PATTERNS;
pub const PREFIX_VALIDATION_PATTERNS = patterns.PREFIX_VALIDATION_PATTERNS;
pub const REGEX_PATTERNS = patterns.REGEX_PATTERNS;

// Re-export pattern counts
pub const SIMPLE_PREFIX_COUNT = patterns.SIMPLE_PREFIX_COUNT;
pub const JWT_COUNT = patterns.JWT_COUNT;
pub const PREFIX_VALIDATION_COUNT = patterns.PREFIX_VALIDATION_COUNT;
pub const REGEX_COUNT = patterns.REGEX_COUNT;
pub const TOTAL_PATTERNS = patterns.TOTAL_PATTERNS;

// Re-export detection functions
pub const detect_simple_prefix = detectors.detect_simple_prefix;
pub const detect_jwt = detectors.detect_jwt;
pub const detect_prefix_validation = detectors.detect_prefix_validation;
pub const detect_regex = detectors.detect_regex;
pub const detect_all_patterns = detectors.detect_all_patterns;

// Backward compatibility: old names
pub const detect_tier1 = detectors.detect_simple_prefix;
pub const detect_tier2 = detectors.detect_prefix_validation;
pub const detect_all_streaming_patterns = detectors.detect_all_patterns;

// ============================================================================
// Pattern Metadata (for FFI)
// ============================================================================

pub const MatchResult = struct {
    found: bool,
    pattern_type: []const u8,
    pattern_name: []const u8,
};

pub const DetectionEvent = extern struct {
    pattern_name: [256]u8 = undefined,
    pattern_type: [64]u8 = undefined,
    position: usize = 0,
    length: usize = 0,
};

pub const PatternDetector = struct {
    allocator: Allocator,

    pub fn init(allocator: Allocator) PatternDetector {
        return .{
            .allocator = allocator,
        };
    }

    pub fn detect_match(self: *PatternDetector, text: []const u8) ?MatchResult {
        _ = self;

        if (detect_simple_prefix(text)) {
            return MatchResult{
                .found = true,
                .pattern_type = "simple_prefix",
                .pattern_name = "matched",
            };
        }

        if (detect_jwt(text)) {
            return MatchResult{
                .found = true,
                .pattern_type = "jwt",
                .pattern_name = "jwt-generic",
            };
        }

        if (detect_prefix_validation(text)) {
            return MatchResult{
                .found = true,
                .pattern_type = "prefix_validation",
                .pattern_name = "matched",
            };
        }

        return null;
    }
};

// ============================================================================
// Pattern Metadata Export (for FFI)
// ============================================================================

pub const ExportedPattern = extern struct {
    name: [128]u8,
    prefix: [256]u8,
    min_len: usize,
};

pub fn get_exported_pattern(index: usize) ?ExportedPattern {
    if (index >= TOTAL_PATTERNS) return null;

    // Handle simple prefix patterns
    if (index < SIMPLE_PREFIX_COUNT) {
        const pattern = SIMPLE_PREFIX_PATTERNS[index];
        var exported: ExportedPattern = undefined;

        @memcpy(exported.name[0..pattern.name.len], pattern.name);
        exported.name[pattern.name.len] = 0;

        @memcpy(exported.prefix[0..pattern.prefix.len], pattern.prefix);
        exported.prefix[pattern.prefix.len] = 0;

        exported.min_len = 0;
        return exported;
    }

    // Handle JWT pattern
    if (index < SIMPLE_PREFIX_COUNT + JWT_COUNT) {
        var exported: ExportedPattern = undefined;
        const jwt_name = "jwt-generic";
        const jwt_prefix = "eyJ";

        @memcpy(exported.name[0..jwt_name.len], jwt_name);
        exported.name[jwt_name.len] = 0;

        @memcpy(exported.prefix[0..jwt_prefix.len], jwt_prefix);
        exported.prefix[jwt_prefix.len] = 0;

        exported.min_len = 7;
        return exported;
    }

    // Handle prefix validation patterns
    if (index < SIMPLE_PREFIX_COUNT + JWT_COUNT + PREFIX_VALIDATION_COUNT) {
        const pv_index = index - SIMPLE_PREFIX_COUNT - JWT_COUNT;
        const pattern = PREFIX_VALIDATION_PATTERNS[pv_index];
        var exported: ExportedPattern = undefined;

        @memcpy(exported.name[0..pattern.name.len], pattern.name);
        exported.name[pattern.name.len] = 0;

        @memcpy(exported.prefix[0..pattern.prefix.len], pattern.prefix);
        exported.prefix[pattern.prefix.len] = 0;

        exported.min_len = pattern.min_len;
        return exported;
    }

    // Handle regex patterns (name only for now)
    if (index < TOTAL_PATTERNS) {
        const regex_index = index - SIMPLE_PREFIX_COUNT - JWT_COUNT - PREFIX_VALIDATION_COUNT;
        const pattern = REGEX_PATTERNS[regex_index];
        var exported: ExportedPattern = undefined;

        @memcpy(exported.name[0..pattern.name.len], pattern.name);
        exported.name[pattern.name.len] = 0;

        exported.prefix[0] = 0;
        exported.min_len = 0;
        return exported;
    }

    return null;
}

pub fn get_pattern_count() usize {
    return TOTAL_PATTERNS;
}

// ============================================================================
// FFI EXPORTS
// ============================================================================

/// FFI: Detect simple prefix patterns
export fn scred_detector_simple_prefix(text: [*]const u8, len: usize) c_int {
    const input = text[0..len];
    return if (detect_simple_prefix(input)) 1 else 0;
}

/// FFI: Detect JWT patterns
export fn scred_detector_jwt(text: [*]const u8, len: usize) c_int {
    const input = text[0..len];
    return if (detect_jwt(input)) 1 else 0;
}

/// FFI: Detect prefix + validation patterns
export fn scred_detector_prefix_validation(text: [*]const u8, len: usize) c_int {
    const input = text[0..len];
    return if (detect_prefix_validation(input)) 1 else 0;
}

/// FFI: Detect all patterns (combined)
export fn scred_detector_all(text: [*]const u8, len: usize) c_int {
    const input = text[0..len];
    return if (detect_all_patterns(input)) 1 else 0;
}

/// FFI: Detect regex pattern (skeleton - TBD)
export fn scred_detector_regex(text: [*]const u8, len: usize, pattern_idx: usize) c_int {
    const input = text[0..len];
    return if (detect_regex(input, pattern_idx)) 1 else 0;
}

// ============================================================================
// Legacy FFI Exports (backward compatibility)
// ============================================================================

/// Legacy: Detect phase2 tier1 patterns
export fn scred_detector_phase2_tier1(text: [*]const u8, len: usize) c_int {
    const input = text[0..len];
    return if (detect_simple_prefix(input)) 1 else 0;
}

/// Legacy: Detect phase2 JWT patterns
export fn scred_detector_phase2_jwt(text: [*]const u8, len: usize) c_int {
    const input = text[0..len];
    return if (detect_jwt(input)) 1 else 0;
}

/// Legacy: Detect phase2 tier2 patterns
export fn scred_detector_phase2_tier2(text: [*]const u8, len: usize) c_int {
    const input = text[0..len];
    return if (detect_prefix_validation(input)) 1 else 0;
}

/// Legacy: Detect all phase2 patterns
export fn scred_detector_phase2_all(text: [*]const u8, len: usize) c_int {
    const input = text[0..len];
    return if (detect_all_patterns(input)) 1 else 0;
}

// ============================================================================
// Pattern Metadata FFI Exports
// ============================================================================

export fn scred_detector_get_pattern_count() usize {
    return get_pattern_count();
}

export fn scred_detector_get_pattern(index: usize, exported: *ExportedPattern) c_int {
    if (get_exported_pattern(index)) |pattern| {
        exported.* = pattern;
        return 1;
    }
    return 0;
}

// ============================================================================
// Legacy Detector API (for internal tests)
// ============================================================================


// ============================================================================
// Legacy Detector API (for internal tests)
// ============================================================================

pub const Detector = opaque {};

export fn scred_detector_new() *Detector {
    return @ptrFromInt(1);
}

export fn scred_detector_process(
    detector: *Detector,
    input: [*]const u8,
    input_len: usize,
    is_eof: bool,
) *u8 {
    _ = detector; _ = input; _ = input_len; _ = is_eof;
    return @ptrFromInt(1);
}

export fn scred_detector_get_redacted_output(detector: *const Detector) *const u8 {
    _ = detector;
    return @ptrFromInt(1);
}

export fn scred_detector_get_output_length(detector: *const Detector) usize {
    _ = detector;
    return 0;
}

export fn scred_detector_get_events(detector: *const Detector) *const u8 {
    _ = detector;
    return @ptrFromInt(1);
}

export fn scred_detector_get_event_count(detector: *const Detector) usize {
    _ = detector;
    return 0;
}

export fn scred_detector_free(detector: *Detector) void {
    _ = detector;
}
