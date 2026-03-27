#[test]
fn test_consistent_x_redaction_pattern() {
    use scred_detector::{detect_all, detector::redact_text};
    
    println!("=== Consistent 'x' Redaction Test ===\n");
    
    let test_cases = vec![
        // SSH Key - should be fully redacted with 'x'
        (
            "This is -----BEGIN RSA PRIVATE KEY-----\ndata\n-----END RSA PRIVATE KEY-----",
            "SSH Key",
            b'x'
        ),
        // API Key - keep first 4, rest with 'x'
        (
            "sk_live_abcd1234efgh5678",
            "API Key",
            b'x'
        ),
        // AWS Key - keep first 4, rest with 'x'
        (
            "AKIA1234567890ABCDEF",
            "AWS Key",
            b'x'
        ),
        // Certificate - should be fully redacted with 'x'
        (
            "-----BEGIN CERTIFICATE-----\ndata\n-----END CERTIFICATE-----",
            "Certificate",
            b'x'
        ),
    ];
    
    for (input, name, expected_char) in test_cases {
        let input_bytes = input.as_bytes();
        let matches = detect_all(input_bytes);
        let output = redact_text(input_bytes, &matches.matches);
        
        println!("{}: {} matches found", name, matches.count());
        println!("  Input:  {}", input);
        println!("  Output: {}", std::str::from_utf8(&output).unwrap_or("<invalid>"));
        
        // Check that redaction uses 'x' character
        if matches.count() > 0 {
            for m in &matches.matches {
                for i in m.start..m.end {
                    let byte = output[i];
                    // First 4 bytes of API keys may be preserved
                    if i < m.start + 4 {
                        // Skip validation for first 4 (may be original chars)
                        continue;
                    }
                    // All redacted portions should be 'x'
                    assert_eq!(byte, expected_char as u8, 
                        "Position {} in {}: expected '{}' but got '{}' ({})", 
                        i, name, expected_char as char, byte as char, byte);
                }
            }
            println!("  ✅ All redactions use '{}' character", expected_char as char);
        }
        println!();
    }
}
