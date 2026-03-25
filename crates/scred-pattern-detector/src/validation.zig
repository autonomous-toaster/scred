//! Validation helper functions for PREFIX_VALIDATION patterns

const std = @import("std");
const patterns = @import("patterns.zig");

/// Check if character matches charset
pub fn charMatchesCharset(char: u8, charset: patterns.Charset) bool {
    return switch (charset) {
        .alphanumeric => isAlphanumeric(char),
        .base64 => isBase64(char),
        .base64url => isBase64Url(char),
        .hex => isHex(char),
        .hex_lowercase => isHexLowercase(char),
        .any => !isDelimiter(char),
    };
}

/// Check if all characters in token match charset
pub fn validateCharset(token: []const u8, charset: patterns.Charset) bool {
    for (token) |char| {
        if (!charMatchesCharset(char, charset)) {
            return false;
        }
    }
    return true;
}

/// Check if token length is within bounds
pub fn validateLength(token: []const u8, min_len: usize, max_len: usize) bool {
    const len = token.len;
    if (min_len > 0 and len < min_len) return false;
    if (max_len > 0 and len > max_len) return false;
    return true;
}

/// Scan forward from position to find token end
/// Returns length of token (not position)
pub fn scanTokenEnd(text: []const u8, start_pos: usize, charset: patterns.Charset, max_len: usize) usize {
    if (start_pos >= text.len) return 0;
    
    var len: usize = 0;
    var pos = start_pos;
    
    while (pos < text.len and len < max_len and charMatchesCharset(text[pos], charset)) {
        pos += 1;
        len += 1;
    }
    
    return len;
}

// Helper functions
fn isAlphanumeric(char: u8) bool {
    return (char >= 'a' and char <= 'z') or
           (char >= 'A' and char <= 'Z') or
           (char >= '0' and char <= '9') or
           char == '-' or char == '_';
}

fn isBase64(char: u8) bool {
    return (char >= 'a' and char <= 'z') or
           (char >= 'A' and char <= 'Z') or
           (char >= '0' and char <= '9') or
           char == '+' or char == '/' or char == '=';
}

fn isBase64Url(char: u8) bool {
    return (char >= 'a' and char <= 'z') or
           (char >= 'A' and char <= 'Z') or
           (char >= '0' and char <= '9') or
           char == '-' or char == '_' or char == '=';
}

fn isHex(char: u8) bool {
    return (char >= '0' and char <= '9') or
           (char >= 'a' and char <= 'f') or
           (char >= 'A' and char <= 'F');
}

fn isHexLowercase(char: u8) bool {
    return (char >= '0' and char <= '9') or
           (char >= 'a' and char <= 'f');
}

fn isDelimiter(char: u8) bool {
    return char == ' ' or char == '\n' or char == '\r' or
           char == '\t' or char == '"' or char == '\'' or
           char == '<' or char == '>' or char == '(' or
           char == ')' or char == '[' or char == ']' or
           char == '{' or char == '}' or char == ',' or
           char == ';' or char == ':' or char == '&' or
           char == '=' or char == '?' or char == '#';
}
