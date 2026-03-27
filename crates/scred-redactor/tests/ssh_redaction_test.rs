#[test]
fn test_ssh_key_character_preservation() {
    use scred_detector::{detect_all, detector::redact_text};
    
    // Sample SSH private key (truncated for testing)
    let input = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA1234567890abcdef\n-----END RSA PRIVATE KEY-----";
    
    println!("SSH Input length:  {}", input.len());
    println!("SSH Input:\n{}\n", std::str::from_utf8(input).unwrap());
    
    let matches = detect_all(input);
    println!("SSH Matches found: {}", matches.count());
    
    for m in &matches.matches {
        println!("  Match [{}, {}] type={}", m.start, m.end, m.pattern_type);
    }
    
    let output = redact_text(input, &matches.matches);
    println!("SSH Output length: {}", output.len());
    println!("SSH Output:\n{}\n", std::str::from_utf8(&output).unwrap());
    
    // CHARACTER PRESERVATION CHECK
    assert_eq!(input.len(), output.len(), 
        "❌ CHARACTER PRESERVATION VIOLATED for SSH keys!\nInput: {}, Output: {}", 
        input.len(), output.len());
    
    println!("✅ SSH key redaction preserves character length!");
}

#[test]
fn test_simple_ssh_key() {
    use scred_detector::{detect_all, detector::redact_text};
    
    let input = b"BEGIN RSA PRIVATE KEY";
    let output = redact_text(input, &detect_all(input).matches);
    
    println!("\nSimple SSH Input:  '{}'", std::str::from_utf8(input).unwrap());
    println!("Simple SSH Output: '{}'", std::str::from_utf8(&output).unwrap());
    println!("Input len:  {}", input.len());
    println!("Output len: {}", output.len());
    
    assert_eq!(input.len(), output.len());
}
