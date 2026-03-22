/// Content Analysis for Smart Pattern Selection
/// 
/// Analyzes text content to determine:
/// - Content type (HTTP, JSON, env, logs, etc.)
/// - Likely patterns to check (reduces from 270 to ~10-50)
/// - Character distribution and structure hints

const std = @import("std");

pub const ContentType = enum {
    http_request,
    http_response,
    json_data,
    form_data,
    yaml_config,
    env_file,
    private_key,
    log_file,
    mixed_text,
};

pub const ContentCharacteristics = struct {
    has_quotes: bool,
    has_equals: bool,
    has_colon: bool,
    has_braces: bool,
    has_brackets: bool,
    has_slashes: bool,
    has_dots: bool,
    has_spaces: bool,
    has_newlines: bool,
    has_angle_brackets: bool,
    has_ampersands: bool,
    has_question_marks: bool,
    
    line_count: usize,
    avg_line_length: usize,
    has_long_lines: bool,
    
    starts_with_http: bool,
    starts_with_brace: bool,
    starts_with_bracket: bool,
    looks_like_env: bool,
    looks_like_json: bool,
    looks_like_yaml: bool,
    looks_like_log: bool,
};

/// Analyze content characteristics
pub fn analyzeContent(allocator: std.mem.Allocator, text: []const u8) !ContentCharacteristics {
    _ = allocator; // Not used in simple implementation
    
    var chars = ContentCharacteristics{
        .has_quotes = false,
        .has_equals = false,
        .has_colon = false,
        .has_braces = false,
        .has_brackets = false,
        .has_slashes = false,
        .has_dots = false,
        .has_spaces = false,
        .has_newlines = false,
        .has_angle_brackets = false,
        .has_ampersands = false,
        .has_question_marks = false,
        
        .line_count = 1,
        .avg_line_length = text.len,
        .has_long_lines = false,
        
        .starts_with_http = false,
        .starts_with_brace = false,
        .starts_with_bracket = false,
        .looks_like_env = false,
        .looks_like_json = false,
        .looks_like_yaml = false,
        .looks_like_log = false,
    };
    
    // Quick character analysis
    for (text) |byte| {
        switch (byte) {
            '"' => chars.has_quotes = true,
            '=' => chars.has_equals = true,
            ':' => chars.has_colon = true,
            '{', '}' => chars.has_braces = true,
            '[', ']' => chars.has_brackets = true,
            '/', '\\' => chars.has_slashes = true,
            '.' => chars.has_dots = true,
            ' ' => chars.has_spaces = true,
            '\n' => {
                chars.has_newlines = true;
                chars.line_count += 1;
            },
            '<', '>' => chars.has_angle_brackets = true,
            '&' => chars.has_ampersands = true,
            '?' => chars.has_question_marks = true,
            else => {},
        }
    }
    
    // Calculate average line length
    if (chars.line_count > 1) {
        chars.avg_line_length = text.len / chars.line_count;
        chars.has_long_lines = chars.avg_line_length > 200;
    }
    
    // Content type hints
    chars.starts_with_http = text.len >= 4 and std.mem.eql(u8, text[0..4], "HTTP");
    chars.starts_with_brace = text.len >= 1 and text[0] == '{';
    chars.starts_with_bracket = text.len >= 1 and text[0] == '[';
    
    // Simple heuristics
    chars.looks_like_env = chars.has_equals and chars.has_newlines and !chars.has_braces;
    chars.looks_like_json = chars.has_braces or chars.has_brackets;
    chars.looks_like_yaml = chars.has_colon and !chars.has_braces and !chars.has_equals;
    chars.looks_like_log = chars.has_newlines and (text.len >= 10 and std.ascii.isDigit(text[0]) and text[4] == '-');
    
    return chars;
}

/// Detect content type from characteristics
pub fn detectContentType(chars: ContentCharacteristics) ContentType {
    if (chars.starts_with_http) {
        return .http_response;
    }
    if (chars.looks_like_json) {
        return .json_data;
    }
    if (chars.looks_like_env) {
        return .env_file;
    }
    if (chars.looks_like_yaml) {
        return .yaml_config;
    }
    if (chars.looks_like_log) {
        return .log_file;
    }
    if (chars.has_angle_brackets and chars.has_slashes) {
        return .http_request;
    }
    if (chars.has_ampersands and chars.has_equals) {
        return .form_data;
    }
    if (chars.has_dots and !chars.has_spaces) {
        return .private_key;
    }
    
    return .mixed_text;
}

/// Get relevant patterns for content type
pub fn getPatternsForContent(allocator: std.mem.Allocator, chars: ContentCharacteristics) ![][]const u8 {
    const content_type = detectContentType(chars);
    var patterns = std.ArrayList([]const u8).init(allocator);
    defer patterns.deinit();
    
    switch (content_type) {
        .http_request, .http_response => {
            // HTTP-related patterns
            try patterns.appendSlice(&[_][]const u8{
                "Authorization header",
                "api_key_header",
                "aws-access-token",
                "github-pat",
                "github-oauth",
                "stripe-api-key",
                "openai-api-key",
                "jwt",
            });
        },
        .json_data => {
            // JSON payload patterns
            try patterns.appendSlice(&[_][]const u8{
                "openai-api-key",
                "stripe-api-key",
                "aws-access-token",
                "github-pat",
                "jwt",
                "contentfulpersonalaccesstoken",
                "sendgrid",
                "twilio-api-key",
                "shopify-shared-secret",
            });
        },
        .env_file => {
            // Environment file patterns
            try patterns.appendSlice(&[_][]const u8{
                "openai-api-key",
                "stripe-api-key",
                "aws-access-token",
                "github-pat",
                "github-oauth",
                "aws-session-token",
                "postgres",
                "mongodb",
                "redis",
                "npmtokenv2",
            });
        },
        .log_file => {
            // Log file patterns
            try patterns.appendSlice(&[_][]const u8{
                "aws-access-token",
                "github-pat",
                "stripe-api-key",
                "postgres",
                "mongodb",
                "jwt",
                "github-oauth",
                "openai-api-key",
            });
        },
        .yaml_config => {
            // YAML configuration patterns
            try patterns.appendSlice(&[_][]const u8{
                "aws-access-token",
                "github-pat",
                "openai-api-key",
                "stripe-api-key",
                "jwt",
                "grafana",
                "pulumi",
            });
        },
        .form_data => {
            // Form data patterns
            try patterns.appendSlice(&[_][]const u8{
                "stripe-api-key",
                "openai-api-key",
                "github-pat",
                "jwt",
            });
        },
        .private_key => {
            // Private key patterns
            try patterns.appendSlice(&[_][]const u8{
                "private-key",
                "privatekey",
            });
        },
        .mixed_text => {
            // Mixed content - check common patterns
            try patterns.appendSlice(&[_][]const u8{
                "aws-access-token",
                "github-pat",
                "stripe-api-key",
                "openai-api-key",
                "jwt",
                "postgres",
                "mongodb",
            });
        },
    }
    
    return patterns.toOwnedSlice();
}

/// Check if content has JWT markers
pub fn hasJwtSignal(text: []const u8) bool {
    // Look for "eyJ" followed by dots
    var i: usize = 0;
    while (i + 3 < text.len) {
        if (text[i] == 'e' and text[i + 1] == 'y' and text[i + 2] == 'J') {
            // Found eyJ, look for dots
            var dot_count: u8 = 0;
            var j = i + 3;
            while (j < text.len and j < i + 200) {  // Reasonable JWT length
                if (text[j] == '.') {
                    dot_count += 1;
                    if (dot_count >= 2) return true;
                }
                j += 1;
            }
        }
        i += 1;
    }
    return false;
}