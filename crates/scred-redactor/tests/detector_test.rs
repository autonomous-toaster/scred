
#[test]
fn test_uri_character_preservation() {
    use scred_detector::{detect_all, detector::redact_text};
    
    // Test MongoDB URI
    let input = b"mongodb://user:MyPassword123@localhost:27017/db";
    let matches = detect_all(input);
    let output = redact_text(input, &matches.matches);
    
    println!("URI Input:  {}", std::str::from_utf8(input).unwrap());
    println!("URI Output: {}", std::str::from_utf8(&output).unwrap());
    
    assert_eq!(input.len(), output.len(), 
        "Character preservation violated! Input: {}, Output: {}", 
        input.len(), output.len());
    
    // Test Slack webhook
    let input2 = b"https://hooks.slack.com/services/T123/B456/abcdef123456";
    let matches2 = detect_all(input2);
    let output2 = redact_text(input2, &matches2.matches);
    
    println!("Webhook Input:  {}", std::str::from_utf8(input2).unwrap());
    println!("Webhook Output: {}", std::str::from_utf8(&output2).unwrap());
    
    assert_eq!(input2.len(), output2.len(), 
        "Character preservation violated! Input: {}, Output: {}", 
        input2.len(), output2.len());
}

#[test]
fn test_uri_pattern_types() {
    use scred_detector::detect_all;
    
    let input = b"mongodb://user:MyPassword123@localhost:27017/db";
    let matches = detect_all(input);
    
    println!("Found {} matches", matches.count());
    for m in &matches.matches {
        println!("Match: [{}, {}], pattern_type: {}, text: {:?}", 
            m.start, m.end, m.pattern_type, 
            std::str::from_utf8(&input[m.start..m.end]).unwrap_or("<invalid>"));
    }
}
