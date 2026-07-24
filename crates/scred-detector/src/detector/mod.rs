//! Pattern detection engine - orchestrates all detection methods
//!
//! Matches Zig implementation exactly:
//! 1. SIMPLE_PREFIX: Fastest, just prefix matching
//! 2. PREFIX_VALIDATION: Medium, prefix + length/charset validation (NO REGEX)
//! 3. JWT: Generic JWT detection (eyJ + 2 dots)
//! 4. MULTILINE_MARKER: SSH keys and cryptographic keys with bounded lookahead

use crate::match_result::{DetectionResult, Match};
use crate::patterns::{
    Charset, GENERALIZED_MARKER_PATTERNS, PREFIX_VALIDATION_PATTERNS, SIMPLE_PREFIX_PATTERNS,
};
use crate::prefix_index::{self, PrefixIndex};
use crate::uri_patterns;
use aho_corasick::AhoCorasick;
use memchr::memchr;
use std::sync::OnceLock;

/// Charset lookup table for fast token scanning
#[derive(Clone, Copy)]
pub struct CharsetLut {
    table: [bool; 256],
}

impl CharsetLut {
    fn new(charset: &[u8]) -> Self {
        let mut table = [false; 256];
        for &byte in charset {
            table[byte as usize] = true;
        }
        CharsetLut { table }
    }

    #[inline]
    fn contains(&self, byte: u8) -> bool {
        self.table[byte as usize]
    }

    /// Scan data for end of token (first byte NOT in charset)
    #[inline]
    fn scan_token_end(&self, data: &[u8], start: usize) -> usize {
        if start >= data.len() {
            return 0;
        }

        // Scalar implementation: process 8 bytes at a time
        let mut i = start;
        let len = data.len();

        while i + 8 <= len {
            if !self.contains(data[i]) {
                return i - start;
            }
            if !self.contains(data[i + 1]) {
                return i + 1 - start;
            }
            if !self.contains(data[i + 2]) {
                return i + 2 - start;
            }
            if !self.contains(data[i + 3]) {
                return i + 3 - start;
            }
            if !self.contains(data[i + 4]) {
                return i + 4 - start;
            }
            if !self.contains(data[i + 5]) {
                return i + 5 - start;
            }
            if !self.contains(data[i + 6]) {
                return i + 6 - start;
            }
            if !self.contains(data[i + 7]) {
                return i + 7 - start;
            }
            i += 8;
        }

        while i < len {
            if !self.contains(data[i]) {
                return i - start;
            }
            i += 1;
        }

        len - start
    }
}

/// Find first occurrence of prefix in data
/// Uses memchr for first byte, then validates full prefix
#[inline]
fn find_first_prefix(data: &[u8], prefix: &[u8]) -> Option<usize> {
    if data.is_empty() || prefix.is_empty() {
        return if prefix.is_empty() { Some(0) } else { None };
    }

    if prefix.len() > data.len() {
        return None;
    }

    let first_byte = prefix[0];

    // Fast path: single-byte prefix
    if prefix.len() == 1 {
        return memchr(first_byte, data);
    }

    // Multi-byte prefix: use memchr to find candidates, then validate
    let mut search_start = 0;
    while let Some(pos) = memchr(first_byte, &data[search_start..]) {
        let absolute_pos = search_start + pos;

        // Check if we have enough bytes for full prefix
        if absolute_pos + prefix.len() <= data.len() {
            // Validate full prefix at this position
            if &data[absolute_pos..absolute_pos + prefix.len()] == prefix {
                return Some(absolute_pos);
            }
        }

        // Move search forward
        search_start = absolute_pos + 1;
    }

    None
}

static ALPHANUMERIC_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static BASE64_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static BASE64URL_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static HEX_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static ANY_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static VALIDATION_AUTOMATON: OnceLock<AhoCorasick> = OnceLock::new();
static SIMPLE_PREFIX_AUTOMATON: OnceLock<AhoCorasick> = OnceLock::new();
static PREFIX_INDEX_CACHE: OnceLock<PrefixIndex> = OnceLock::new();

fn get_alphanumeric_lut() -> &'static CharsetLut {
    ALPHANUMERIC_CHARSET.get_or_init(|| {
        CharsetLut::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-")
    })
}

fn get_base64_lut() -> &'static CharsetLut {
    BASE64_CHARSET.get_or_init(|| {
        CharsetLut::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=")
    })
}

fn get_base64url_lut() -> &'static CharsetLut {
    BASE64URL_CHARSET.get_or_init(|| {
        CharsetLut::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_=")
    })
}

fn get_hex_lut() -> &'static CharsetLut {
    HEX_CHARSET.get_or_init(|| CharsetLut::new(b"0123456789abcdefABCDEF"))
}

fn get_any_lut() -> &'static CharsetLut {
    ANY_CHARSET.get_or_init(|| {
        CharsetLut::new(b" !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~")
    })
}

/// Build a simple first-byte index for patterns (computed once, cached)
/// Maps first byte -> vec of pattern indices
fn build_first_byte_index() -> &'static Vec<Vec<usize>> {
    // Use OnceLock to build once and cache
    static INDEX: OnceLock<Vec<Vec<usize>>> = OnceLock::new();

    INDEX.get_or_init(|| {
        // Initialize empty vecs for all 256 bytes
        let mut index: Vec<Vec<usize>> = vec![Vec::new(); 256];

        // Index PREFIX_VALIDATION_PATTERNS by first byte
        for (idx, pattern) in PREFIX_VALIDATION_PATTERNS.iter().enumerate() {
            if !pattern.prefix.is_empty() {
                let first_byte = pattern.prefix.as_bytes()[0] as usize;
                index[first_byte].push(idx);
            }
        }

        index
    })
}

/// Get charset lookup table for a charset type
pub fn get_charset_lut(charset: Charset) -> &'static CharsetLut {
    match charset {
        Charset::Alphanumeric => get_alphanumeric_lut(),
        Charset::Base64 => get_base64_lut(),
        Charset::Base64Url => get_base64url_lut(),
        Charset::Hex => get_hex_lut(),
        Charset::Any => get_any_lut(),
    }
}

/// Calculate optimal threshold for simple_prefix based on CPU core count

pub mod detect_all;
pub mod jwt;
pub mod redact;
pub mod simple_prefix;
pub mod ssh;
pub mod tests;
pub mod validation;
pub use detect_all::*;
pub use jwt::*;
pub use redact::*;
pub use simple_prefix::*;
pub use ssh::*;
pub use validation::*;
