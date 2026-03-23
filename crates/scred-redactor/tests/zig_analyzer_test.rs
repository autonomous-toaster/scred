#[test]
fn test_zig_analyzer_aws() {
    use scred_redactor::analyzer::ZigAnalyzer;
    
    let text = "AKIAIOSFODNN7EXAMPLE";
    
    let has_simple = ZigAnalyzer::has_simple_prefix_pattern(text);
    let has_all = ZigAnalyzer::has_all_patterns(text);
    
    println!("Text: {}", text);
    println!("has_simple_prefix: {}", has_simple);
    println!("has_all: {}", has_all);
    
    assert!(has_simple || has_all, "Should detect AWS AKIA pattern");
}

#[test]
fn test_zig_analyzer_github() {
    use scred_redactor::analyzer::ZigAnalyzer;
    
    let text = "ghp_1234567890123456789012345678901";
    
    let has_simple = ZigAnalyzer::has_simple_prefix_pattern(text);
    let has_all = ZigAnalyzer::has_all_patterns(text);
    
    println!("Text: {}", text);
    println!("has_simple_prefix: {}", has_simple);
    println!("has_all: {}", has_all);
    
    assert!(has_simple || has_all, "Should detect GitHub token");
}
