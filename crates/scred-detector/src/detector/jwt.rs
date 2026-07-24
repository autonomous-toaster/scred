use super::*;

pub fn detect_jwt(text: &[u8]) -> DetectionResult {
    let mut result = DetectionResult::with_capacity(10);

    let prefix = b"eyJ";
    let jwt_charset = get_base64url_lut();

    let mut search_pos = 0;

    while let Some(pos) = find_first_prefix(&text[search_pos..], prefix) {
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
