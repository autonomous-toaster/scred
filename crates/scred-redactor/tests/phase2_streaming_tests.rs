//! Phase 2 Streaming Integration Tests
//! Tests detection of all 72 patterns in streaming scenarios

use scred_redactor::analyzer::ZigAnalyzer;

#[test]
fn test_tier1_streaming() {
    let test_cases = vec![
        ("prefix AGE-SECRET-KEY-1suffix", true, "Tier 1: age-secret-key"),
        ("text sk_live_abcdef middle", true, "Tier 1: apideck"),
        ("no pattern here", false, "Tier 1: no match"),
        ("AGE-SECRET-KEY-1", true, "Tier 1: exact match"),
    ];
    
    for (input, expected, desc) in test_cases {
        let result = ZigAnalyzer::has_tier1_pattern(input);
        assert_eq!(result, expected, "{}: {}", desc, input);
    }
}

#[test]
fn test_jwt_streaming() {
    let valid_jwts = vec![
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
        "eyJ.payload.signature",
        "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.data.sig rest",
        "start eyJ1234.1234.1234 end",
    ];
    
    for jwt in valid_jwts {
        assert!(ZigAnalyzer::has_jwt_pattern(jwt), "Should detect: {}", jwt);
    }
    
    let invalid_jwts = vec![
        "eyJhbGciOiJIUzI1NiJ9",
        "eyJ",
        "data.signature",
        "random text",
    ];
    
    for jwt in invalid_jwts {
        assert!(!ZigAnalyzer::has_jwt_pattern(jwt), "Should NOT detect: {}", jwt);
    }
}

#[test]
fn test_tier2_streaming() {
    let test_cases = vec![
        (format!("sk-ant-{}", "x".repeat(90)), true, "Tier 2: anthropic valid"),
        ("sk-ant-short".to_string(), false, "Tier 2: anthropic too short"),
        (format!("pat-{}", "x".repeat(40)), true, "Tier 2: hubspot valid"),
        ("no tier2 patterns".to_string(), false, "Tier 2: no match"),
    ];
    
    for (input, expected, desc) in test_cases {
        let result = ZigAnalyzer::has_tier2_pattern(&input);
        assert_eq!(result, expected, "{}: {}", desc, input);
    }
}

#[test]
fn test_phase2_combined_streaming() {
    let test_cases = vec![
        ("Tier1: sk_live_xxx Tier2: pat-xxx JWT: eyJ.d.s".to_string(), true),
        ("Tier1: AGE-SECRET-KEY-1abc".to_string(), true),
        ("JWT only: eyJhbGciOiJIUzI1NiJ9.data.sig".to_string(), true),
        (format!("Tier2 only: sk-ant-{}", "x".repeat(90)), true),
        ("no secrets here at all".to_string(), false),
    ];
    
    for (input, expected) in test_cases {
        let result = ZigAnalyzer::has_phase2_pattern(&input);
        assert_eq!(result, expected, "Input: {}", input);
    }
}

#[test]
fn test_pattern_at_chunk_boundary() {
    let lookahead = "text ends with ey";
    let new_chunk = "JhbGciOiJIUzI1NiJ9.data.sig rest of data";
    let combined = format!("{}{}", lookahead, new_chunk);
    
    assert!(ZigAnalyzer::has_jwt_pattern(&combined), 
            "Should detect JWT spanning chunks");
}

#[test]
fn test_multiple_patterns_in_chunk() {
    let pat_token = format!("pat-{}", "x".repeat(40));
    let input = format!("start sk_live_token1 middle eyJhbGciOiJIUzI1NiJ9.data.sig {} end", pat_token);
    
    assert!(ZigAnalyzer::has_tier1_pattern(&input));
    assert!(ZigAnalyzer::has_jwt_pattern(&input));
    assert!(ZigAnalyzer::has_tier2_pattern(&input));
    assert!(ZigAnalyzer::has_phase2_pattern(&input));
}

#[test]
fn test_large_streaming_buffer() {
    let mut large_buffer = "safe text: ".repeat(1000);
    large_buffer.push_str("sk_live_secret_token");
    large_buffer.push_str(&" more safe text ".repeat(1000));
    
    assert!(ZigAnalyzer::has_tier1_pattern(&large_buffer));
    assert!(ZigAnalyzer::has_phase2_pattern(&large_buffer));
}

#[test]
fn test_pattern_at_different_positions() {
    let positions = vec![
        ("AGE-SECRET-KEY-1 rest", "start"),
        ("prefix AGE-SECRET-KEY-1 suffix", "middle"),
        ("prefix AGE-SECRET-KEY-1", "end"),
    ];
    
    for (input, pos) in positions {
        assert!(ZigAnalyzer::has_tier1_pattern(input), 
                "Should detect pattern at {}: {}", pos, input);
    }
}

#[test]
fn test_false_positives() {
    let false_cases = vec![
        "The age is 25 years old",
        "Authorization header missing",
        "Bearer token required",
        "Key-value storage",
        "password123456789",
        "normal text without patterns",
    ];
    
    for input in false_cases {
        assert!(!ZigAnalyzer::has_phase2_pattern(input), 
                "False positive detected: {}", input);
    }
}

#[test]
fn test_special_characters_around_patterns() {
    let delimiters = vec![
        "sk_live_token ",
        " sk_live_token",
        "\"sk_live_token\"",
        "'sk_live_token'",
        "(sk_live_token)",
        "[sk_live_token]",
        "{sk_live_token}",
        "sk_live_token\n",
        "sk_live_token\t",
        "sk_live_token;",
        "sk_live_token,",
    ];
    
    for input in delimiters {
        assert!(ZigAnalyzer::has_tier1_pattern(input), 
                "Should detect with delimiters: {:?}", input);
    }
}

#[test]
fn test_empty_and_small_inputs() {
    assert!(!ZigAnalyzer::has_phase2_pattern(""));
    assert!(!ZigAnalyzer::has_phase2_pattern("x"));
    assert!(!ZigAnalyzer::has_phase2_pattern("xy"));
    assert!(!ZigAnalyzer::has_phase2_pattern("xyz"));
}

#[test]
fn test_unicode_content() {
    let input = "こんにちは sk_live_token 世界";
    assert!(ZigAnalyzer::has_tier1_pattern(input), 
            "Should detect pattern in unicode content");
}

#[test]
fn test_streaming_efficiency_many_chunks() {
    let mut found_tier1 = false;
    let mut found_jwt = false;
    let mut found_tier2 = false;
    
    let tier2_chunk = format!("chunk with sk-ant-{}", "x".repeat(90));
    let chunks = vec![
        "safe chunk 1",
        "safe chunk 2", 
        "chunk with sk_live_token",
        "safe chunk 3",
        "chunk with eyJhbGciOiJIUzI1NiJ9.data.sig",
        "safe chunk 4",
        &tier2_chunk,
    ];
    
    for chunk in chunks {
        if !found_tier1 && ZigAnalyzer::has_tier1_pattern(chunk) {
            found_tier1 = true;
        }
        if !found_jwt && ZigAnalyzer::has_jwt_pattern(chunk) {
            found_jwt = true;
        }
        if !found_tier2 && ZigAnalyzer::has_tier2_pattern(chunk) {
            found_tier2 = true;
        }
    }
    
    assert!(found_tier1, "Should find Tier 1 pattern across chunks");
    assert!(found_jwt, "Should find JWT pattern across chunks");
    assert!(found_tier2, "Should find Tier 2 pattern across chunks");
}
