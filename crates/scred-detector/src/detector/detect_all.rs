use super::*;

pub fn detect_all(text: &[u8]) -> DetectionResult {
    let mut result = detect_simple_prefix(text);
    result.extend(detect_validation(text));
    result.extend(detect_jwt(text));
    result.extend(detect_ssh_keys(text));
    result.extend(detect_uri_patterns(text));
    result.remove_overlaps();

    // Filter out placeholder matches containing "scrd-" marker
    // These are policy placeholders that should NOT be redacted
    result.matches.retain(|m| {
        if m.end <= text.len() {
            !text[m.start..m.end].windows(4).any(|w| w == b"scrd-")
        } else {
            true
        }
    });

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
