use super::*;

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
        1 => 512, // Single core: minimal threshold
        2 => 2048,
        3..=4 => 3072,
        5..=8 => 4096, // 8 cores: optimal configuration
        9..=16 => 6000,
        _ => 8000, // Many cores: higher threshold
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

        match AhoCorasick::new(&prefixes) {
            Ok(ac) => ac,
            Err(e) => {
                // Unreachable: our patterns are compile-time constants
                unreachable!("AhoCorasick construction failed: {}", e)
            }
        }
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

    let mut last_end = 0; // Track end of last kept match

    // Single-pass matching: O(n + m) where m = number of matches
    // Each match tells us: which pattern (0-17) and position in text
    for m in automaton.find_iter(text) {
        let pos = m.start();

        // Quick skip: if this match starts before the last match ended, it overlaps
        // We'll remove overlaps later anyway, so skip expensive scan here
        if pos < last_end {
            continue;
        }

        let pattern_idx = m.pattern().as_usize(); // Convert PatternID to usize
        let pattern = &PREFIX_VALIDATION_PATTERNS[pattern_idx];

        // Early rejection: check if remaining text is long enough for min_len
        let token_start = pos + pattern.prefix.len();
        let remaining = text.len().saturating_sub(token_start);
        if remaining < pattern.min_len {
            continue; // Not enough data, skip validation
        }

        // Validate token: check length and charset constraints
        // Limit scan to max_len to avoid scanning too far
        let charset_lut = get_charset_lut(pattern.charset);
        let max_scan = if pattern.max_len > 0 {
            pattern.max_len
        } else {
            remaining
        };
        let token_len = charset_lut.scan_token_end(text, token_start);
        let token_len = token_len.min(max_scan);

        // Check if token passes validation constraints (length/charset)
        if token_len >= pattern.min_len && (pattern.max_len == 0 || token_len <= pattern.max_len) {
            let end_pos = (token_start + token_len).min(text.len());
            result.add(Match::new(pos, end_pos, (100 + pattern_idx) as u16));
            last_end = end_pos;
        }
    }

    result
}
