// Test pattern classification FFI

#[test]
fn test_pattern_classification() {
    use scred_pattern_detector::{get_all_patterns, pattern_type_name, pattern_type_description};
    
    let patterns = get_all_patterns();
    let count = patterns.len();
    
    println!("\n\n=== PATTERN CLASSIFICATION VERIFICATION ===");
    println!("Total patterns: {}\n", count);
    assert!(count > 200, "Expected 200+ patterns, got {}", count);
    
    println!("{:<35} | {:<20} | {:<30}", 
             "Pattern Name", "PatternType", "Description");
    println!("{}", "-".repeat(90));
    
    let mut type_counts = [0; 3];  // FastPrefix, StructuredFormat, RegexBased
    
    for pattern in patterns.iter().take(30) {
        let type_name = pattern_type_name(pattern.pattern_type);
        let type_desc = pattern_type_description(pattern.pattern_type);
        
        println!("{:<35} | {:<20} | {:<30}",
                 pattern.name,
                 type_name,
                 type_desc);
        
        // Count distributions
        if pattern.pattern_type < 3 {
            type_counts[pattern.pattern_type as usize] += 1;
        }
        
        // Verify pattern_type is in valid range
        assert!(pattern.pattern_type <= 2, 
                "Invalid pattern_type for {}: {}", pattern.name, pattern.pattern_type);
    }
    
    println!("\n=== PATTERN TYPE DISTRIBUTION ===");
    println!("\nPattern Types (first 30 patterns):");
    println!("  FastPrefix (0): {}", type_counts[0]);
    println!("  StructuredFormat (1): {}", type_counts[1]);
    println!("  RegexBased (2): {}", type_counts[2]);
    
    // Verify all patterns in entire set
    let mut all_type_counts = [0; 3];
    for pattern in patterns.iter() {
        if pattern.pattern_type < 3 {
            all_type_counts[pattern.pattern_type as usize] += 1;
        }
    }
    
    println!("\nAll {} patterns breakdown:", count);
    println!("  FastPrefix (0): {} patterns", all_type_counts[0]);
    println!("  StructuredFormat (1): {} patterns", all_type_counts[1]);
    println!("  RegexBased (2): {} patterns", all_type_counts[2]);
    
    // Verify the expected distribution
    assert!(all_type_counts[0] > 60, "Expected 60+ FastPrefix patterns, got {}", all_type_counts[0]);
    assert!(all_type_counts[1] == 1, "Expected 1 StructuredFormat pattern, got {}", all_type_counts[1]);
    assert!(all_type_counts[2] > 190, "Expected 190+ RegexBased patterns, got {}", all_type_counts[2]);
}
