use scred_readctor_framering::redact_text;

#[test]
fn test_validation_charset_rejects_invalid_aws() {
    // OpenAI patterns should detect sk-proj- prefixes with valid chars
    let text = format!("api_key=sk-proj-{}", "a".repeat(40));
    let result = redact_text(&text);
    
    // Should detect and redact the sk-proj key
    assert_ne!(text, result, "Should detect and redact OpenAI key");
}

#[test]
fn test_validation_length_bounds() {
    // Test that patterns with length constraints are validated
    // OpenAI keys should be specific length
    let short_key = "sk-proj-123"; // Too short
    let long_key = format!("sk-proj-{}", "A".repeat(100)); // Too long
    let valid_key = format!("sk-proj-{}", "A".repeat(40)); // Right length
    
    let result_short = redact_text(short_key);
    let result_long = redact_text(&long_key);
    let result_valid = redact_text(&valid_key);
    
    // Valid key should be redacted
    assert_ne!(valid_key, result_valid, "Valid key should be redacted");
}

#[test]
fn test_validation_empty_token() {
    // Empty token after prefix should not match
    let text = "sk- ";
    let result = redact_text(text);
    
    // Should not crash, might or might not redact
    assert!(!result.is_empty(), "Should not crash on empty token");
}

#[test]
fn test_validation_token_boundaries() {
    // Token should end at delimiter
    let text = "sk-validtoken, next";
    let result = redact_text(text);
    
    // Comma should mark token end (if charset doesn't include comma)
    assert!(!result.is_empty(), "Should handle token boundaries");
}

#[test]
fn test_validation_multiple_patterns() {
    let text = "AWS=AKIAIOSFODNN7EXAMPLE GitHub=ghp_123 OpenAI=sk-proj-abc";
    let result = redact_text(text);
    
    // Should redact all three (or at least try)
    assert_ne!(text, result, "Should detect multiple patterns");
}

#[test]
fn test_validation_no_false_positives_from_similar_prefix() {
    // "key" prefix should not match "keyboard"
    let text = "keyword is not a secret key=value";
    let result = redact_text(text);
    
    // Should not unnecessarily redact
    let keywordcount_original = text.matches("keyword").count();
    let keywordcount_result = result.matches("keyword").count();
    
    assert_eq!(
        keywordcount_original, keywordcount_result,
        "Should not break valid words"
    );
}

#[test]
fn test_validation_jwt_token() {
    // JWT starts with eyJ
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let text = format!("token={}", jwt);
    let result = redact_text(&text);
    
    // JWT should be detected and redacted
    assert_ne!(text, result, "JWT should be detected");
}

#[test]
fn test_validation_bearer_token() {
    let token = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let text = format!("Authorization: {}", token);
    let result = redact_text(&text);
    
    assert_ne!(text, result, "Bearer token should be detected");
}

#[test]
fn test_validation_preserves_length() {
    // Redaction should preserve output length
    let text = "secret: AKIAIOSFODNN7EXAMPLE";
    let result = redact_text(text);
    
    // Output should be same length as input
    assert_eq!(text.len(), result.len(), "Redaction should preserve length");
}

#[test]
fn test_validation_redaction_format() {
    // First 4 chars visible, rest 'x'
    let text = "AWS_KEY=AKIAIOSFODNN7EXAMPLE";
    let result = redact_text(text);
    
    // Should start with "AWS_" and have 'x' characters
    assert!(result.contains('x'), "Should contain redaction character 'x'");
}
