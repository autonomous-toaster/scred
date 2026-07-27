use super::*;

fn apply_redaction_rule(buffer: &mut [u8], match_: &Match, original: &[u8]) {
    // Check for environment variables FIRST (contains '=')
    // This takes precedence over SSH key type because env vars can have pattern_type >= 300
    if original[match_.start..match_.end].contains(&b'=') {
        // Environment variable: key=value structure
        // Keep the key and equals sign, preserve first 4 chars of value, redact rest
        if let Some(eq_pos) = original[match_.start..match_.end]
            .iter()
            .position(|&b| b == b'=')
        {
            let value_start = match_.start + eq_pos + 1;
            let preserve_len = 4.min(match_.end - value_start);
            let redact_start = value_start + preserve_len;

            for i in redact_start..match_.end {
                if i < buffer.len() {
                    buffer[i] = b'x';
                }
            }
        }
    } else if match_.pattern_type >= 300 {
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
pub fn redact_in_place_with_original(
    buffer: &mut [u8],
    matches: &[Match],
    original: &[u8],
) -> usize {
    if matches.is_empty() {
        return 0;
    }

    let count = matches.len();

    for m in matches {
        apply_redaction_rule(buffer, m, original);
    }

    count
}
