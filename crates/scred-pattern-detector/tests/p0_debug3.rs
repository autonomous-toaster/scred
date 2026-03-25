use regex::Regex;

#[test]
fn test_jdbc_parts() {
    let input = "jdbc:oracle:thin:scott:tiger@//oracle.example.com:1521/orcl";
    
    // Part 1: Just scheme
    let re1 = Regex::new(r"[a-zA-Z][\w+:\.\-]*://").unwrap();
    println!("\nPart 1 (scheme with ://): {}", re1.is_match(input));
    if let Some(m) = re1.find(input) {
        println!("  Matched: {}", &input[m.start()..m.end()]);
    }
    
    // Part 2: username:password
    let re2 = Regex::new(r"[a-zA-Z0-9._%-]+:[^@]+").unwrap();
    println!("Part 2 (user:pass): {}", re2.is_match(input));
    if let Some(m) = re2.find(input) {
        println!("  Matched: {}", &input[m.start()..m.end()]);
    }
    
    // Part 3: Full pattern
    let re3 = Regex::new(r"[a-zA-Z][\w+:\.\-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.\-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    println!("Part 3 (full): {}", re3.is_match(input));
    if let Some(m) = re3.find(input) {
        println!("  Matched: {}", &input[m.start()..m.end()]);
    }
    
    // Simplified version - remove character class issues
    let re4 = Regex::new(r"[a-zA-Z][a-zA-Z0-9+:.\-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.\-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    println!("Part 4 (simplified): {}", re4.is_match(input));
    if let Some(m) = re4.find(input) {
        println!("  Matched: {}", &input[m.start()..m.end()]);
    }
}
