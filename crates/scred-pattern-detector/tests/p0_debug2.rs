use regex::Regex;

#[test]
fn test_header_simple() {
    // Test each part individually
    let input = "X-Auth-Token: sk_live_abcdef123456789012345678901234";
    
    // Part 1: Just the header name
    let re1 = Regex::new(r"X-Auth-Token").unwrap();
    println!("\nPart 1 (header name): {}", re1.is_match(input));
    
    // Part 2: Header name with colon
    let re2 = Regex::new(r"X-Auth-Token:").unwrap();
    println!("Part 2 (with colon): {}", re2.is_match(input));
    
    // Part 3: With space
    let re3 = Regex::new(r"X-Auth-Token: ").unwrap();
    println!("Part 3 (with space): {}", re3.is_match(input));
    
    // Part 4: With token (no underscore)
    let re4 = Regex::new(r"X-Auth-Token: [a-zA-Z0-9]{20,}").unwrap();
    println!("Part 4 (token no underscore): {}", re4.is_match(input));
    
    // Part 5: With underscore
    let re5 = Regex::new(r"X-Auth-Token: [a-zA-Z0-9_]{20,}").unwrap();
    println!("Part 5 (with underscore): {}", re5.is_match(input));
    
    // Part 6: Full pattern
    let re6 = Regex::new(r"X-Auth-Token: [a-zA-Z0-9_./+\-]{20,}").unwrap();
    println!("Part 6 (full): {}", re6.is_match(input));
    
    // Part 7: Case insensitive manually
    let re7 = Regex::new(r"[Xx]-[Aa]uth-[Tt]oken: [a-zA-Z0-9_./+\-]{20,}").unwrap();
    println!("Part 7 (manual case insensitive): {}", re7.is_match(input));
}
