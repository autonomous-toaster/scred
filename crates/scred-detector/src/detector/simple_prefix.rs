use super::*;

fn get_simple_prefix_threshold() -> usize {
    let num_cpus = num_cpus::get();

    // Simple prefix has fewer patterns (23), so lower thresholds
    match num_cpus {
        1 => 256,
        2 => 384,
        3..=4 => 448,
        5..=8 => 512, // 8 cores: optimal
        9..=16 => 768,
        _ => 1024,
    }
}

/// Build Aho-Corasick automaton from SIMPLE_PREFIX_PATTERNS prefixes
fn get_simple_prefix_automaton() -> &'static AhoCorasick {
    SIMPLE_PREFIX_AUTOMATON.get_or_init(|| {
        // Build automaton from all SIMPLE_PREFIX_PATTERNS prefixes
        let prefixes: Vec<&str> = SIMPLE_PREFIX_PATTERNS.iter().map(|p| p.prefix).collect();

        match AhoCorasick::new(&prefixes) {
            Ok(ac) => ac,
            Err(e) => {
                // Unreachable: our patterns are compile-time constants
                unreachable!("AhoCorasick construction failed: {}", e)
            }
        }
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

    let mut last_end = 0; // Track end of last kept match

    for m in automaton.find_iter(text) {
        let pos = m.start();

        // Quick skip: if this match starts before the last match ended, it overlaps
        // We'll remove overlaps later anyway, so skip expensive scan here
        if pos < last_end {
            continue;
        }

        let pattern_idx = m.pattern().as_usize();

        // Token is everything from start to end of alphanumeric run
        // Limit scan to MAX_SIMPLE_TOKEN_LEN to avoid scanning too far
        let token_len = charset.scan_token_end(text, pos);
        let token_len = token_len.min(MAX_SIMPLE_TOKEN_LEN);
        let end_pos = (pos + token_len).min(text.len());

        result.add(Match::new(pos, end_pos, pattern_idx as u16));
        last_end = end_pos;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_simple_prefix_threshold() {
        let threshold = get_simple_prefix_threshold();
        assert!(threshold > 0);
        assert!(threshold < 1000);
    }
}
