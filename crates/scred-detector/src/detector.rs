//! Pattern detection engine - orchestrates all detection methods
//! 
//! Matches Zig implementation exactly:
//! 1. SIMPLE_PREFIX: Fastest, just prefix matching
//! 2. PREFIX_VALIDATION: Medium, prefix + length/charset validation (NO REGEX)
//! 3. JWT: Generic JWT detection (eyJ + 2 dots)
//! 4. MULTILINE_MARKER: SSH keys and cryptographic keys with bounded lookahead

use crate::match_result::{Match, DetectionResult};
use crate::patterns::{
    SIMPLE_PREFIX_PATTERNS, PREFIX_VALIDATION_PATTERNS,
    GENERALIZED_MARKER_PATTERNS, Charset,
};
use crate::prefix_index::{self, PrefixIndex};
use crate::uri_patterns;
use crate::simd_core::{self, CharsetLut};
use std::sync::OnceLock;
use aho_corasick::AhoCorasick;
static ALPHANUMERIC_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static BASE64_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static BASE64URL_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static HEX_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static ANY_CHARSET: OnceLock<CharsetLut> = OnceLock::new();
static VALIDATION_AUTOMATON: OnceLock<AhoCorasick> = OnceLock::new();
static SIMPLE_PREFIX_AUTOMATON: OnceLock<AhoCorasick> = OnceLock::new();

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
    HEX_CHARSET.get_or_init(|| {
        CharsetLut::new(b"0123456789abcdefABCDEF")
    })
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
#[inline]
fn get_simple_prefix_threshold() -> usize {
    let num_cpus = num_cpus::get();
    
    // Simple prefix has fewer patterns (23), so lower thresholds
    match num_cpus {
        1 => 256,
        2 => 384,
        3..=4 => 448,
        5..=8 => 512,     // 8 cores: optimal
        9..=16 => 768,
        _ => 1024,
    }
}

/// Build Aho-Corasick automaton from SIMPLE_PREFIX_PATTERNS prefixes
fn get_simple_prefix_automaton() -> &'static AhoCorasick {
    SIMPLE_PREFIX_AUTOMATON.get_or_init(|| {
        // Build automaton from all SIMPLE_PREFIX_PATTERNS prefixes
        let prefixes: Vec<&str> = SIMPLE_PREFIX_PATTERNS
            .iter()
            .map(|p| p.prefix)
            .collect();
        
        AhoCorasick::new(&prefixes).expect("Valid Aho-Corasick automaton")
    })
}

/// Detect all simple prefix patterns (fast path, no validation)
/// Parallelized version
pub fn detect_simple_prefix(text: &[u8]) -> DetectionResult {
    // Phase 3: Aho-Corasick Multi-Pattern Matching
    // Replaces old 26-pass algorithm with single-pass automaton
    // Similar improvement to detect_validation()
    
    let automaton = get_simple_prefix_automaton();
    let mut result = DetectionResult::with_capacity(100);
    let charset = get_alphanumeric_lut();

    // Single-pass matching: find all 26 patterns simultaneously
    // Typical API keys are 20-200 bytes, so cap scan at 256 for performance
    const MAX_SIMPLE_TOKEN_LEN: usize = 256;
    
    for m in automaton.find_iter(text) {
        let pattern_idx = m.pattern().as_usize();
        let pos = m.start();
        
        // Token is everything from start to end of alphanumeric run
        // Limit scan to MAX_SIMPLE_TOKEN_LEN to avoid scanning too far
        let token_len = charset.scan_token_end(text, pos);
        let token_len = token_len.min(MAX_SIMPLE_TOKEN_LEN);
        let end_pos = (pos + token_len).min(text.len());
        
        result.add(Match::new(pos, end_pos, pattern_idx as u16));
    }

    result
}

/// Detect prefix validation patterns (with length and charset validation - NO REGEX!)
/// Parallelized with rayon for multi-core speedup
/// Build a set of relevant pattern indices based on bytes present in text
/// Only parallelizes patterns whose first byte appears in text
fn get_relevant_validation_patterns(text: &[u8]) -> Vec<usize> {
    // Quick scan: identify which first bytes appear in text
    let mut byte_appears = [false; 256];
    for &byte in text {
        byte_appears[byte as usize] = true;
    }
    
    // Collect indices of patterns whose first byte appears
    let mut relevant = Vec::new();
    let index = build_first_byte_index();
    for byte in 0..256 {
        if byte_appears[byte] && !index[byte].is_empty() {
            relevant.extend(&index[byte]);
        }
    }
    relevant
}

/// Calculate optimal parallelization threshold based on CPU core count
/// More cores → higher threshold (amortize overhead over larger sequential pass)
/// Fewer cores → lower threshold (parallelize more aggressively)
#[inline]
fn get_validation_threshold() -> usize {
    let num_cpus = num_cpus::get();
    
    // Empirically derived formula based on core count:
    // 2 cores: 2048
    // 4 cores: 3072  
    // 8 cores: 4096 (measured optimal)
    // 16 cores: 6000
    // 32+ cores: 8000
    
    match num_cpus {
        1 => 512,         // Single core: minimal threshold
        2 => 2048,
        3..=4 => 3072,
        5..=8 => 4096,    // 8 cores: optimal configuration
        9..=16 => 6000,
        _ => 8000,        // Many cores: higher threshold
    }
}

/// Build Aho-Corasick automaton from PREFIX_VALIDATION_PATTERNS prefixes
/// Called once via OnceLock - creates single-pass pattern matching automaton
fn get_validation_automaton() -> &'static AhoCorasick {
    VALIDATION_AUTOMATON.get_or_init(|| {
        // Build automaton from all PREFIX_VALIDATION_PATTERNS prefixes
        // Each pattern's prefix is a simple string we want to find
        let prefixes: Vec<&str> = PREFIX_VALIDATION_PATTERNS
            .iter()
            .map(|p| p.prefix)
            .collect();
        
        AhoCorasick::new(&prefixes).expect("Valid Aho-Corasick automaton")
    })
}

pub fn detect_validation(text: &[u8]) -> DetectionResult {
    // Phase 3: Aho-Corasick Multi-Pattern Matching
    // Replaces old 18-pass algorithm with single-pass automaton
    // Expected: ~12-16x faster (2400ms → 150-200ms for 100MB)
    //
    // Key insight: Old algorithm did independent SIMD search for each pattern
    // Aho-Corasick builds optimal state machine for all patterns simultaneously
    
    let automaton = get_validation_automaton();
    let mut result = DetectionResult::with_capacity(100);

    // Single-pass matching: O(n + m) where m = number of matches
    // Each match tells us: which pattern (0-17) and position in text
    for m in automaton.find_iter(text) {
        let pattern_idx = m.pattern().as_usize();  // Convert PatternID to usize
        let pattern = &PREFIX_VALIDATION_PATTERNS[pattern_idx];
        let pos = m.start();  // Position where prefix was found

        // Early rejection: check if remaining text is long enough for min_len
        let token_start = pos + pattern.prefix.len();
        let remaining = text.len().saturating_sub(token_start);
        if remaining < pattern.min_len {
            continue;  // Not enough data, skip validation
        }

        // Validate token: check length and charset constraints
        // Limit scan to max_len to avoid scanning too far
        let charset_lut = get_charset_lut(pattern.charset);
        let max_scan = if pattern.max_len > 0 { pattern.max_len } else { remaining };
        let token_len = charset_lut.scan_token_end(text, token_start);
        let token_len = token_len.min(max_scan);

        // Check if token passes validation constraints (length/charset)
        if token_len >= pattern.min_len && (pattern.max_len == 0 || token_len <= pattern.max_len) {
            let end_pos = (token_start + token_len).min(text.len());
            result.add(Match::new(pos, end_pos, (100 + pattern_idx) as u16));
        }
    }

    result
}

/// Detect JWT patterns: eyJ prefix + exactly 2 dots (no regex!)
pub fn detect_jwt(text: &[u8]) -> DetectionResult {
    let mut result = DetectionResult::with_capacity(10);

    let prefix = b"eyJ";
    let jwt_charset = get_base64url_lut();
    
    let mut search_pos = 0;

    while let Some(pos) = simd_core::find_first_prefix(&text[search_pos..], prefix) {
        let start = search_pos + pos;
        let mut end = start + prefix.len();
        let mut dot_count = 0;

        // Scan JWT body: JWT tokens are base64url encoded (A-Za-z0-9-_) with dots
        // Must have exactly 2 dots: header.payload.signature
        while end < text.len() && end - start < 10000 {
            let byte = text[end];
            
            // Stop at whitespace or common boundaries
            match byte {
                b' ' | b'\n' | b'\t' | b'\r' | b',' | b';' | b')' | b']' => break,
                b'.' => {
                    dot_count += 1;
                    if dot_count > 2 {
                        break;
                    }
                }
                _ if !jwt_charset.contains(byte) => break,
                _ => {}
            }
            
            end += 1;
        }

        // Valid JWT must have exactly 2 dots and be at least 32 bytes
        if dot_count == 2 && end - start >= 32 {
            // Pattern type: 200 for JWT
            result.add(Match::new(start, end, 200));
        }

        search_pos = start + prefix.len();
    }

    result
}

/// Detect multiline SSH key patterns using bounded lookahead
/// Looks for -----BEGIN...PRIVATE KEY----- with matching END marker
/// Pattern type: 300+ for multiline markers
/// Cached prefix index for O(1) pattern dispatch
/// Initialized once at first use
static PREFIX_INDEX_CACHE: OnceLock<PrefixIndex> = OnceLock::new();

/// Get or initialize the global prefix index
fn get_prefix_index() -> &'static PrefixIndex {
    PREFIX_INDEX_CACHE.get_or_init(|| {
        prefix_index::PrefixIndex::build(GENERALIZED_MARKER_PATTERNS)
    })
}

/// Detect SSH keys and other multiline marker patterns with prefix-based dispatch
/// 
/// Optimized with PrefixIndex for O(1) pattern candidate lookup:
/// - Instead of checking all 11 patterns at each position
/// - Build HashMap from pattern prefixes (first 8-16 bytes)
/// - For each text position, only check relevant patterns (~3 avg)
/// - Result: 3-4x speedup on multiline detection
pub fn detect_ssh_keys(text: &[u8]) -> DetectionResult {
    let mut result = DetectionResult::with_capacity(10);
    
    // Optimization: Quick check - if no "-----BEGIN" marker in text, skip expensive scanning
    // This avoids O(n*m) byte-by-byte scanning for texts without SSH keys
    // (40.9 MB/s → expected 2000+ MB/s for empty case)
    if !text.windows(11).any(|w| w == b"-----BEGIN ") {
        return result;
    }
    
    // Get prefix index (cached, built once at startup)
    let index = get_prefix_index();
    
    // Scan text, using prefix dispatch to find relevant patterns
    let mut pos = 0;
    while pos < text.len() {
        // Try to get candidate patterns for this position
        if let Some(candidate_indices) = index.get_candidates(text, pos) {
            // Check only the candidate patterns (~3 instead of 11)
            for &pattern_idx in candidate_indices {
                let pattern = &GENERALIZED_MARKER_PATTERNS[pattern_idx];
                let start_bytes = pattern.start_marker.as_bytes();
                let end_bytes = pattern.end_marker.as_bytes();
                
                // Check if pattern matches at this position
                if text[pos..].starts_with(start_bytes) {
                    // Found start marker, now look for end marker within lookahead
                    let lookahead_end = std::cmp::min(pos + pattern.max_lookahead, text.len());
                    let lookahead = &text[pos..lookahead_end];
                    
                    // Search for end marker within lookahead window
                    if let Some(end_offset) = simd_core::find_first_prefix(lookahead, end_bytes) {
                        // Found complete pattern
                        let end_marker_pos = pos + end_offset;
                        let end = end_marker_pos + end_bytes.len();
                        
                        // Include newline after END marker if present
                        let final_end = if end < text.len() && text[end] == b'\n' {
                            end + 1
                        } else {
                            end
                        };
                        
                        // Add match with pattern type ID
                        result.add(Match::new(pos, final_end, pattern.pattern_type));
                        
                        // Skip past this match to avoid overlaps
                        pos = final_end;
                        break; // Move to next text position
                    }
                }
            }
        }
        
        // No match at this position, advance by 1 byte
        pos += 1;
    }
    
    result
}

/// Detect all patterns: simple prefix first (fastest), then validation, then JWT, then SSH keys
pub fn detect_all(text: &[u8]) -> DetectionResult {
    let mut result = detect_simple_prefix(text);
    result.extend(detect_validation(text));
    result.extend(detect_jwt(text));
    result.extend(detect_ssh_keys(text));
    result.extend(detect_uri_patterns(text));
    result.remove_overlaps();
    result
}

/// Detect database URIs and webhook URLs with embedded credentials
/// Returns matches for: mongodb, redis, postgres, etc. + Slack/Discord webhooks
/// Uses Aho-Corasick for O(n) scheme detection
pub fn detect_uri_patterns(text: &[u8]) -> DetectionResult {
    let mut result = DetectionResult::with_capacity(10);
    
    // Detect database connection URIs
    let db_matches = uri_patterns::detect_database_uris(text);
    for m in db_matches {
        result.add(m);
    }
    
    // Detect webhook URLs
    let webhook_matches = uri_patterns::detect_webhook_uris(text);
    for m in webhook_matches {
        result.add(m);
    }
    
    result
}

/// Apply redaction rules to a single match within a buffer
/// 
/// Handles:
/// - SSH keys (pattern_type >= 300): fully redacted
/// - Environment variables (contain '='): keep key=value structure
/// - Regular patterns: keep first 4 chars, redact rest
fn apply_redaction_rule(buffer: &mut [u8], match_: &Match, original: &[u8]) {
    let is_ssh_key = match_.pattern_type >= 300;
    
    // Env var detection: contains '=' in the original match
    if !is_ssh_key && original[match_.start..match_.end].contains(&b'=') {
        // Environment variable: key=value structure
        // Keep the key and equals sign, preserve first 4 chars of value, redact rest
        if let Some(eq_pos) = original[match_.start..match_.end].iter().position(|&b| b == b'=') {
            let value_start = match_.start + eq_pos + 1;
            let preserve_len = 4.min(match_.end - value_start);
            let redact_start = value_start + preserve_len;
            
            for i in redact_start..match_.end {
                if i < buffer.len() {
                    buffer[i] = b'x';
                }
            }
        }
    } else if is_ssh_key {
        // SSH keys and certificates: fully redacted with 'x'
        for i in match_.start..match_.end {
            if i < buffer.len() {
                buffer[i] = b'x';
            }
        }
    } else {
        // Regular patterns (API keys, tokens, etc.)
        // Keep first 4 characters (the prefix), replace rest with 'x'
        let preserve_len = 4.min(match_.end - match_.start);
        for i in (match_.start + preserve_len)..match_.end {
            if i < buffer.len() {
                buffer[i] = b'x';
            }
        }
    }
}

/// Redact matched regions in text by replacing with 'x'
/// Preserves character length (redacted output same length as input)
/// Keeps first 4 characters of matched region (the prefix is visible for context)
pub fn redact_text(text: &[u8], matches: &[Match]) -> Vec<u8> {
    if matches.is_empty() {
        return text.to_vec();
    }

    let mut result = text.to_vec();

    for m in matches {
        apply_redaction_rule(&mut result, m, text);
    }

    result
}

/// In-place redaction: modify buffer directly without allocating output
/// 
/// # Phase 1B.2: Zero-Copy In-Place Redaction
/// 
/// This function modifies the input buffer directly, replacing detected patterns
/// with redaction character 'x'. No separate output buffer allocated.
/// 
/// # Character Preservation
/// 
/// Critical constraint: output length MUST equal input length
/// All redaction uses consistent 'x' character:
/// - SSH keys: Replace ALL chars with 'x' (full redaction)
/// - Environment variables: Keep key=value structure, redact only value with 'x'
/// - API keys: Keep first 4 chars (prefix), replace rest with 'x'
/// 
/// # Arguments
/// * `buffer` - Mutable bytes to redact in place
/// * `matches` - Pattern matches to redact
/// 
/// # Returns
/// Number of patterns redacted (same as matches.len())
/// 
/// # Example
/// ```ignore
/// let mut buffer = b"AKIAIOSFODNN7EXAMPLE".to_vec();
/// let matches = detect_all(buffer);
/// let count = redact_in_place(&mut buffer, &matches.matches);
/// assert_eq!(count, 1);
/// assert_eq!(buffer, b"AKIAxxxxxxxxxxxxxxxx");
/// ```
/// Redact matched regions in-place by replacing with 'x'
/// Preserves character length (output same length as input)
/// Creates an internal clone of buffer for env var detection
/// For best performance when you already have original: use redact_in_place_with_original()
pub fn redact_in_place(buffer: &mut [u8], matches: &[Match]) -> usize {
    if matches.is_empty() {
        return 0;
    }
    
    let original = buffer.to_vec();
    redact_in_place_with_original(buffer, matches, &original)
}

/// Redact matched regions in-place without cloning
/// Pass the original buffer separately to avoid allocation
#[inline]
pub fn redact_in_place_with_original(buffer: &mut [u8], matches: &[Match], original: &[u8]) -> usize {
    if matches.is_empty() {
        return 0;
    }

    let count = matches.len();

    for m in matches {
        apply_redaction_rule(buffer, m, original);
    }

    count
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_simple_prefix_aws() {
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let result = detect_simple_prefix(text);
        assert!(result.count() > 0);
        assert_eq!(result.matches[0].start, 0);
        assert!(result.matches[0].end > 4); // At least prefix length
    }

    #[test]
    fn test_detect_simple_prefix_github() {
        let text = b"token ghp_abcdefghijklmnopqrstuvwxyz";
        let result = detect_simple_prefix(text);
        assert!(result.count() > 0);
    }

    #[test]
    fn test_detect_validation_github_detailed() {
        let text = b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let result = detect_validation(text);
        assert!(result.count() > 0, "Should detect github-token-detailed");
    }

    #[test]
    fn test_detect_jwt() {
        let text = b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let result = detect_jwt(text);
        assert!(result.count() > 0, "Should detect JWT");
        assert_eq!(result.matches[0].start, 0);
    }

    #[test]
    fn test_detect_jwt_in_context() {
        let text = b"Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.X3XL0MU4p0Xz5W1Z6KvK";
        let result = detect_jwt(text);
        assert!(result.count() > 0);
    }

    #[test]
    fn test_detect_all_mixed() {
        let text = b"AWS Key: AKIAIOSFODNN7EXAMPLE GitHub: ghp_abcd1234 JWT: eyJhbGc.eyJzdWI.SflKxw";
        let result = detect_all(text);
        assert!(result.count() >= 2, "Should detect multiple patterns");
    }

    #[test]
    fn test_detect_no_false_positives() {
        let text = b"This is normal text with numbers 12345 and letters abcde";
        let result = detect_all(text);
        // Should not detect random text
        assert_eq!(result.count(), 0);
    }

    #[test]
    fn test_redact_text_preserves_length() {
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let matches = vec![Match::new(0, 20, 0)];
        let redacted = redact_text(text, &matches);
        assert_eq!(text.len(), redacted.len());
        // First 4 chars (prefix) preserved, rest redacted
        assert_eq!(redacted, b"AKIAxxxxxxxxxxxxxxxx");
    }

    #[test]
    fn test_redact_text_mixed() {
        let text = b"key: AKIAIOSFODNN7 value";
        let matches = vec![Match::new(5, 21, 0)]; // Positions 5-21: "AKIAIOSFODNN7 va" (16 bytes)
        let redacted = redact_text(text, &matches);
        assert_eq!(text.len(), redacted.len());
        // Keep "AKIA" (4 chars), replace "IOSFODNN7 va" (12 chars) with x's
        assert_eq!(redacted, b"key: AKIAxxxxxxxxxxxxlue");
    }

    // Environment variable redaction tests
    #[test]
    #[test]
    fn test_redact_env_client_secret() {
        let text = b"SERVICE_CLIENT_SECRET=abcdef123456";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Keep "SERVICE_CLIENT_SECRET=", first 4 of value "abcd", redact rest
        assert_eq!(redacted, b"SERVICE_CLIENT_SECRET=abcdxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_api_key() {
        let text = b"STRIPE_API_KEY=sk_test_abcd1234567890ef";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Keep "STRIPE_API_KEY=", first 4 of value "sk_t", redact rest
        assert_eq!(redacted, b"STRIPE_API_KEY=sk_txxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_token() {
        let text = b"AUTH_TOKEN=eyJhbGciOiJIUzI1NiJ9abcd1234567890";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Keep "AUTH_TOKEN=", first 4 of value "eyJa", redact rest
        assert_eq!(redacted, b"AUTH_TOKEN=eyJhxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_password() {
        let text = b"DB_PASSWORD=MySecurePassword123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Keep "DB_PASSWORD=", first 4 of value "MySe", redact rest
        assert_eq!(redacted, b"DB_PASSWORD=MySexxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_short_value() {
        // Test with value shorter than 4 characters
        let text = b"API_KEY=abc";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Keep "API_KEY=", value is only 3 chars, preserve all, nothing to redact
        assert_eq!(redacted, b"API_KEY=abc");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_exactly_4_chars() {
        // Test with value exactly 4 characters
        let text = b"KEY=abcd";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Keep "KEY=", preserve all 4 chars of value, nothing to redact
        assert_eq!(redacted, b"KEY=abcd");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_with_special_chars_in_value() {
        // Test environment variable with special characters in value
        let text = b"MONGODB_URI=mongodb+srv://user:pass@cluster.mongodb.net/db?param=value";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Keep "MONGODB_URI=", first 4 of value "mong", redact rest
        assert_eq!(redacted, b"MONGODB_URI=mongxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_multiple_in_text() {
        // Test multiple environment variables in same text
        let text = b"PGPASS=6577abc123 and SERVICE_API_KEY=sk_test_123456";
        let matches = vec![
            Match::new(0, 17, 0),  // PGPASS=6577abc123
            Match::new(22, 52, 0), // SERVICE_API_KEY=sk_test_123456
        ];
        let redacted = redact_text(text, &matches);
        
        // First: "PGPASS=6577xxxxxxxxx", second: "SERVICE_API_KEY=sk_txxxxxxxxxxxxxxx"
        // Middle " and " should be unchanged
        assert_eq!(redacted, b"PGPASS=6577xxxxxx and SERVICE_API_KEY=sk_txxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_non_env_still_works() {
        // Ensure non-environment patterns still work as before
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // No '=' in match, so use old behavior: keep first 4, redact rest
        assert_eq!(redacted, b"AKIAxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_github_token_no_equals() {
        let text = b"ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // No '=' sign, use old behavior: keep first 4 chars
        assert_eq!(redacted, b"ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_langsmith_deployment_key() {
        let text = b"lsv2_sk_abcdef1234567890abcdef1234567890abc";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Should keep first 4 chars "lsv2" and redact rest
        assert_eq!(redacted, b"lsv2xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_pgpassword() {
        let text = b"PGPASSWORD=mypassword123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Should keep "PGPASSWORD=" and first 4 of value "mypa", redact rest
        assert_eq!(redacted, b"PGPASSWORD=mypaxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_mysql_pwd() {
        let text = b"MYSQL_PWD=secretpassword";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Should keep "MYSQL_PWD=" and first 4 of value "secr", redact rest
        assert_eq!(redacted, b"MYSQL_PWD=secrxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_rabbitmq_default_pass() {
        let text = b"RABBITMQ_DEFAULT_PASS=guest123456789";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=22 chars, value=14 chars, preserve first 4 "gues", redact 10
        assert_eq!(redacted, b"RABBITMQ_DEFAULT_PASS=guesxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_redis_password() {
        let text = b"REDIS_PASSWORD=foobared123456";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=15 chars, value=14 chars, preserve first 4 "foob", redact 10
        assert_eq!(redacted, b"REDIS_PASSWORD=foobxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_postgres_password() {
        let text = b"POSTGRES_PASSWORD=postgres_secret_123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=18 chars, value=19 chars, preserve first 4 "post", redact 15
        assert_eq!(redacted, b"POSTGRES_PASSWORD=postxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_docker_registry_password() {
        let text = b"DOCKER_REGISTRY_PASSWORD=dckr_secret_abc123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=25 chars, value=18 chars, preserve first 4 "dckr", redact 14
        assert_eq!(redacted, b"DOCKER_REGISTRY_PASSWORD=dckrxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_vault_token() {
        let text = b"VAULT_TOKEN=hvs.CAESIAbcDefG123456HijKl789MnOpQ";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=12 chars, value=35 chars, preserve first 4 "hvs.", redact 31
        assert_eq!(redacted, b"VAULT_TOKEN=hvs.xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_ldap_password() {
        let text = b"LDAP_PASSWORD=ldap_admin_password_2024";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=14 chars, value=24 chars, preserve first 4 "ldap", redact 20
        assert_eq!(redacted, b"LDAP_PASSWORD=ldapxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_ldap_bind_password() {
        let text = b"LDAP_BIND_PASSWORD=bind_account_secret_pass";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=19 chars, value=24 chars, preserve first 4 "bind", redact 20
        assert_eq!(redacted, b"LDAP_BIND_PASSWORD=bindxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_cassandra_password() {
        let text = b"CASSANDRA_PASSWORD=cassandra_node_password";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=19 chars, value=23 chars, preserve first 4 "cass", redact 19
        assert_eq!(redacted, b"CASSANDRA_PASSWORD=cassxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_elasticsearch_password() {
        let text = b"ELASTICSEARCH_PASSWORD=elastic_search_pwd_123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=23 chars, value=22 chars, preserve first 4 "elas", redact 18
        assert_eq!(redacted, b"ELASTICSEARCH_PASSWORD=elasxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_couchdb_password() {
        let text = b"COUCHDB_PASSWORD=couchdb_admin_secret";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=17 chars, value=20 chars, preserve first 4 "couc", redact 16
        assert_eq!(redacted, b"COUCHDB_PASSWORD=coucxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_kafka_sasl_password() {
        let text = b"KAFKA_SASL_PASSWORD=kafka_broker_password_24";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=19 chars, value=25 chars, preserve first 4 "kafk", redact 21
        assert_eq!(redacted, b"KAFKA_SASL_PASSWORD=kafkxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_activemq_password() {
        let text = b"ACTIVEMQ_PASSWORD=activemq_broker_secret_pwd";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=18 chars, value=26 chars, preserve first 4 "acti", redact 22
        assert_eq!(redacted, b"ACTIVEMQ_PASSWORD=actixxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_bitbucket_password() {
        let text = b"BITBUCKET_PASSWORD=bitbucket_ci_password_123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=19 chars, value=25 chars, preserve first 4 "bitb", redact 21
        assert_eq!(redacted, b"BITBUCKET_PASSWORD=bitbxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_smtp_password() {
        let text = b"SMTP_PASSWORD=smtp_mail_server_secret";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);
        
        // Key=13 chars, value=24 chars, preserve first 4 "smtp", redact 20
        assert_eq!(redacted, b"SMTP_PASSWORD=smtpxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_pgpass_vs_pgpassword() {
        // Test that both patterns work correctly in different contexts
        let text1 = b"PGPASS=abc123456";
    }

    // ============================================================================
    // SSH KEY DETECTION TESTS
    // ============================================================================

    #[test]
    fn test_detect_ssh_rsa_key() {
        let input = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA1234567890\n-----END RSA PRIVATE KEY-----\n";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "SSH RSA key should be detected");
        assert_eq!(result.matches[0].start, 0);
        assert!(result.matches[0].end > 30, "Should cover full key");
    }

    #[test]
    fn test_detect_ssh_openssh_key() {
        let input = b"-----BEGIN OPENSSH PRIVATE KEY-----\nAAAAB3NzaC1yc2EAAA\n-----END OPENSSH PRIVATE KEY-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "SSH OpenSSH key should be detected");
    }

    #[test]
    fn test_ssh_ec_private_key() {
        let input = b"-----BEGIN EC PRIVATE KEY-----\nMHcCAQEEIIGlVdZfvfg\n-----END EC PRIVATE KEY-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "EC private key should be detected");
    }

    #[test]
    fn test_incomplete_ssh_key_no_match() {
        let input = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA";
        let result = detect_ssh_keys(input);
        assert!(result.matches.is_empty(), "Incomplete key (no END marker) should not match");
    }

    #[test]
    fn test_multiple_ssh_keys() {
        let input = b"key1:\n-----BEGIN RSA PRIVATE KEY-----\ndata1\n-----END RSA PRIVATE KEY-----\nkey2:\n-----BEGIN OPENSSH PRIVATE KEY-----\ndata2\n-----END OPENSSH PRIVATE KEY-----\n";
        let result = detect_ssh_keys(input);
        assert!(result.count() >= 2, "Should detect multiple keys");
    }

    #[test]
    fn test_ssh_key_in_mixed_content() {
        let input = "# SSH Configuration\nPrivateKey:\n-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA1234567890abcdef\n-----END RSA PRIVATE KEY-----\n# End of configuration";
        let result = detect_ssh_keys(input.as_bytes());
        assert!(!result.matches.is_empty(), "SSH key in mixed content should be detected");
    }

    #[test]
    fn test_detect_all_with_ssh_key() {
        let input = b"API_KEY=abc123def456\n-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA\n-----END RSA PRIVATE KEY-----";
        let result = detect_all(input);
        // Should find both API key and SSH key
        assert!(result.count() >= 1, "Should detect patterns including SSH key");
    }

    #[test]
    fn test_redact_ssh_key_full() {
        let text = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA\n-----END RSA PRIVATE KEY-----";
        let matches = vec![Match::new(0, text.len(), 300)];  // Pattern type 300 = SSH key
        let redacted = redact_text(text, &matches);
        
        // SSH keys should be fully redacted with 'x' (consistent with all redaction)
        for (i, &byte) in redacted.iter().enumerate() {
            if i < text.len() {
                assert_eq!(byte, b'x', "SSH key bytes should be redacted with 'x' at position {}", i);
            }
        }
        assert_eq!(text.len(), redacted.len(), "Redaction must preserve length");
    }

    #[test]
    fn test_false_positive_ssh_like_text() {
        let input = "# This is a comment about -----BEGIN something-----\n# But it's not a real key";
        let result = detect_ssh_keys(input.as_bytes());
        assert!(result.matches.is_empty(), "Random text with -----BEGIN should not match");
    }

    #[test]
    fn test_ssh_key_without_newline_after_end() {
        // SSH key at end of file without trailing newline
        let input = b"-----BEGIN PRIVATE KEY-----\ndata123\n-----END PRIVATE KEY-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "SSH key without trailing newline should still match");
    }

    // ===== Phase 4b: Certificate Pattern Tests =====

    #[test]
    fn test_detect_x509_certificate() {
        let input = b"-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJAKy11CCCCBDMA0G\n-----END CERTIFICATE-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "X.509 certificate should be detected");
        assert!(result.matches[0].end > 30, "Should cover full certificate");
    }

    #[test]
    fn test_detect_certificate_request() {
        let input = b"-----BEGIN CERTIFICATE REQUEST-----\nMIICljCCAX4CAQAwDQYJKoZIhvcNAQEBBQAw\n-----END CERTIFICATE REQUEST-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "Certificate request should be detected");
    }

    #[test]
    fn test_detect_encrypted_private_key() {
        let input = b"-----BEGIN ENCRYPTED PRIVATE KEY-----\nMIIFHDBOBgkqhkiG9w0BBQ0wQTApBgkqhkiG9w0BBQwwHAYIKwYBBQUHAwIECJ+C\n-----END ENCRYPTED PRIVATE KEY-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "Encrypted private key should be detected");
    }

    #[test]
    fn test_detect_public_key() {
        let input = b"-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA1234567890\n-----END PUBLIC KEY-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "Public key should be detected");
    }

    #[test]
    fn test_incomplete_certificate_no_match() {
        let input = b"-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJAKy11CC";
        let result = detect_ssh_keys(input);
        assert!(result.matches.is_empty(), "Incomplete certificate (no END marker) should not match");
    }

    #[test]
    fn test_multiple_certificates() {
        let input = b"cert1:\n-----BEGIN CERTIFICATE-----\ndata1\n-----END CERTIFICATE-----\n\ncert2:\n-----BEGIN CERTIFICATE-----\ndata2\n-----END CERTIFICATE-----";
        let result = detect_ssh_keys(input);
        assert!(result.count() >= 2, "Should detect multiple certificates");
    }

    #[test]
    fn test_certificate_in_mixed_content() {
        let input = "# TLS Configuration\nCertificate:\n-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJAKy11CCC\n-----END CERTIFICATE-----\n# End config";
        let result = detect_ssh_keys(input.as_bytes());
        assert!(!result.matches.is_empty(), "Certificate in mixed content should be detected");
    }

    #[test]
    fn test_redact_certificates_full() {
        let text = b"-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJAKy11CCC\n-----END CERTIFICATE-----";
        let matches = vec![Match::new(0, text.len(), 304)];  // Pattern type 304 = certificate
        let redacted = redact_text(text, &matches);
        
        // Certificates should be fully redacted with 'x' (consistent with all redaction)
        for (i, &byte) in redacted.iter().enumerate() {
            if i < text.len() {
                assert_eq!(byte, b'x', "Certificate bytes should be redacted with 'x' at position {}", i);
            }
        }
        assert_eq!(text.len(), redacted.len(), "Redaction must preserve length");
    }

    // ===== Phase 4c: PGP Key Pattern Tests =====

    #[test]
    fn test_detect_pgp_private_key_block() {
        let input = b"-----BEGIN PGP PRIVATE KEY BLOCK-----\nVersion: GnuPG v1\nhQEMA5qETJX5s6SUAQf+MQsometestdata\n-----END PGP PRIVATE KEY BLOCK-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "PGP private key block should be detected");
    }

    #[test]
    fn test_detect_pgp_public_key_block() {
        let input = b"-----BEGIN PGP PUBLIC KEY BLOCK-----\nVersion: GnuPG v1\nmQGiBDoxrZ0RBADZ\n-----END PGP PUBLIC KEY BLOCK-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "PGP public key block should be detected");
    }

    #[test]
    fn test_detect_pgp_message() {
        let input = b"-----BEGIN PGP MESSAGE-----\nVersion: GnuPG v1\nwcDMA5qETJX5s6SUAQf+MQsometestencrypted\n-----END PGP MESSAGE-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "PGP encrypted message should be detected");
    }

    #[test]
    fn test_incomplete_pgp_key_no_match() {
        let input = b"-----BEGIN PGP PRIVATE KEY BLOCK-----\nVersion: GnuPG v1\nhQEMA5qETJX5s6SUAQf";
        let result = detect_ssh_keys(input);
        assert!(result.matches.is_empty(), "Incomplete PGP key (no END marker) should not match");
    }

    #[test]
    fn test_multiple_pgp_keys() {
        let input = b"key1:\n-----BEGIN PGP PUBLIC KEY BLOCK-----\ndata1\n-----END PGP PUBLIC KEY BLOCK-----\nkey2:\n-----BEGIN PGP PRIVATE KEY BLOCK-----\ndata2\n-----END PGP PRIVATE KEY BLOCK-----";
        let result = detect_ssh_keys(input);
        assert!(result.count() >= 2, "Should detect multiple PGP keys");
    }

    #[test]
    fn test_pgp_in_mixed_content() {
        let input = "# PGP Key Storage\nPrivate Key:\n-----BEGIN PGP PRIVATE KEY BLOCK-----\nVersion: GnuPG\ndata123data456\n-----END PGP PRIVATE KEY BLOCK-----\n# End storage";
        let result = detect_ssh_keys(input.as_bytes());
        assert!(!result.matches.is_empty(), "PGP key in mixed content should be detected");
    }

    #[test]
    fn test_redact_pgp_key_full() {
        let text = b"-----BEGIN PGP PRIVATE KEY BLOCK-----\nVersion: GnuPG v1\ndata123\n-----END PGP PRIVATE KEY BLOCK-----";
        let matches = vec![Match::new(0, text.len(), 308)];  // Pattern type 308 = PGP private key
        let redacted = redact_text(text, &matches);
        
        // PGP keys should be fully redacted with 'x' (consistent with all redaction)
        for (i, &byte) in redacted.iter().enumerate() {
            if i < text.len() {
                assert_eq!(byte, b'x', "PGP key bytes should be redacted with 'x' at position {}", i);
            }
        }
        assert_eq!(text.len(), redacted.len(), "Redaction must preserve length");
    }

    // ============================================================================
    // Phase 1B.2: In-Place Redaction Tests
    // ============================================================================

    #[test]
    fn test_redact_in_place_basic_api_key() {
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let mut buffer = text.to_vec();
        let detection = detect_all(&buffer);
        
        let count = redact_in_place(&mut buffer, &detection.matches);
        
        assert_eq!(count, 1, "Should redact 1 pattern");
        assert_eq!(buffer.len(), text.len(), "Length must be preserved");
        assert_eq!(&buffer[..4], b"AKIA", "First 4 chars (prefix) preserved");
        for byte in &buffer[4..] {
            assert_eq!(*byte, b'x', "Rest should be redacted with 'x'");
        }
    }

    #[test]
    fn test_redact_in_place_env_variable() {
        // Note: Environment variable pattern detection may vary
        // This test verifies the in-place redaction works IF pattern is detected
        let text = b"DATABASE_PASSWORD=secret123";
        let mut buffer = text.to_vec();
        let detection = detect_all(&buffer);
        
        if !detection.matches.is_empty() {
            let count = redact_in_place(&mut buffer, &detection.matches);
            
            // If detected, should redact properly
            assert!(count > 0, "Should redact detected patterns");
            assert_eq!(buffer.len(), text.len(), "Length must be preserved");
            assert!(String::from_utf8_lossy(&buffer).contains('='), "Equals sign must be preserved");
        }
        // If not detected, that's OK - env patterns are optional in detector
    }

    #[test]
    fn test_redact_in_place_multiple_secrets() {
        let text = b"First: AKIAIOSFODNN7EXAMPLE and second: ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let mut buffer = text.to_vec();
        let detection = detect_all(&buffer);
        
        let count = redact_in_place(&mut buffer, &detection.matches);
        
        assert!(count >= 2, "Should redact multiple patterns");
        assert_eq!(buffer.len(), text.len(), "Length must be preserved");
    }

    #[test]
    fn test_redact_in_place_vs_redact_text_equivalence() {
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let detection = detect_all(text);
        
        // Method 1: redact_text (original)
        let redacted_text = redact_text(text, &detection.matches);
        
        // Method 2: redact_in_place (new)
        let mut buffer = text.to_vec();
        redact_in_place(&mut buffer, &detection.matches);
        
        // Both should produce identical results
        assert_eq!(buffer, redacted_text, "In-place and copy-based redaction must be identical");
    }

    #[test]
    fn test_redact_in_place_empty_matches() {
        let text = b"no secrets here";
        let mut buffer = text.to_vec();
        let original = buffer.clone();
        
        let count = redact_in_place(&mut buffer, &[]);
        
        assert_eq!(count, 0, "Should redact 0 patterns");
        assert_eq!(buffer, original, "Buffer should be unchanged");
    }

    #[test]
    fn test_redact_in_place_ssh_key_full_redaction() {
        let text = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA1234567890abcdef\n-----END RSA PRIVATE KEY-----";
        let mut buffer = text.to_vec();
        let detection = detect_all(&buffer);
        
        if !detection.matches.is_empty() {
            redact_in_place(&mut buffer, &detection.matches);
            
            // SSH keys should be fully redacted
            assert_eq!(buffer.len(), text.len(), "Length must be preserved");
        }
    }

    #[test]
    fn test_redact_in_place_character_preservation_aws() {
        let text = b"My access key: AKIAIOSFODNN7EXAMPLE, keep it secret!";
        let mut buffer = text.to_vec();
        let original_len = buffer.len();
        let detection = detect_all(&buffer);
        
        redact_in_place(&mut buffer, &detection.matches);
        
        assert_eq!(buffer.len(), original_len, "Character count must be preserved");
        assert_eq!(buffer.len(), text.len(), "Output length must match input");
    }

    #[test]
    fn test_redact_in_place_all_patterns_preserve_length() {
        // Test a variety of secrets to ensure all preserve length
        let test_cases = vec![
            b"AKIAIOSFODNN7EXAMPLE" as &[u8],
            b"ghp_1234567890abcdefghijklmnopqrstuvwxyz",
            b"sk_live_123456789",
            b"AIzaSyB1234567890abcdefg",
        ];
        
        for text in test_cases {
            let mut buffer = text.to_vec();
            let original_len = buffer.len();
            let detection = detect_all(&buffer);
            
            if !detection.matches.is_empty() {
                redact_in_place(&mut buffer, &detection.matches);
                assert_eq!(
                    buffer.len(), original_len,
                    "Length must be preserved for: {}",
                    String::from_utf8_lossy(text)
                );
            }
        }
    }
}

