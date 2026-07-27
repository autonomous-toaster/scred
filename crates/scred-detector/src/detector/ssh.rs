use super::*;

fn get_prefix_index() -> &'static PrefixIndex {
    PREFIX_INDEX_CACHE.get_or_init(|| prefix_index::PrefixIndex::build(GENERALIZED_MARKER_PATTERNS))
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
                    if let Some(end_offset) = find_first_prefix(lookahead, end_bytes) {
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
