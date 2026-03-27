//! Comprehensive test to verify ALL redaction types preserve character length

#[test]
fn test_all_pattern_types_character_preservation() {
    use scred_detector::{detect_all, detector::redact_text};
    
    let test_cases = vec![
        // API Keys (Simple Prefix - pattern type 0-22)
        ("sk_live_abcd1234efgh5678", "Stripe live key"),
        ("rk_live_xyz123abc456def", "Rekey live key"),
        
        // Validation Patterns (pattern type 100-347)
        ("AKIA1234567890ABCDEF", "AWS Access Key"),
        ("dapi1234567890abcdefghijklmnopqr", "Databricks token"),
        
        // JWT (pattern type 200)
        ("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c", "JWT token"),
        
        // SSH Keys (pattern type 300+)
        ("-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA1234567890\n-----END RSA PRIVATE KEY-----", "SSH RSA key"),
        ("-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmU\n-----END OPENSSH PRIVATE KEY-----", "SSH OpenSSL key"),
        
        // Environment Variables
        ("API_KEY=sk_live_1234567890abcdef", "Env var with API key"),
        ("PASSWORD=MySecretPassword123", "Env var with password"),
        
        // URI Patterns (pattern type 400+)
        ("mongodb://user:MyPassword123@localhost:27017/db", "MongoDB URI"),
        ("redis://user:secretpass@redis.example.com:6379/0", "Redis URI"),
    ];
    
    println!("=== CHARACTER PRESERVATION TEST ===\n");
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (input, description) in test_cases {
        let input_bytes = input.as_bytes();
        let matches = detect_all(input_bytes);
        let output = redact_text(input_bytes, &matches.matches);
        
        let status = if input_bytes.len() == output.len() {
            passed += 1;
            "✅"
        } else {
            failed += 1;
            "❌"
        };
        
        println!("{} {} (len: {} → {})", 
            status, 
            description,
            input_bytes.len(),
            output.len());
        
        if input_bytes.len() != output.len() {
            println!("   Input:  {}", input);
            println!("   Output: {}", std::str::from_utf8(&output).unwrap_or("<invalid>"));
        }
        
        assert_eq!(input_bytes.len(), output.len(), 
            "Character preservation failed for: {}", description);
    }
    
    println!("\n{} passed, {} failed", passed, failed);
    assert_eq!(failed, 0, "Some tests failed character preservation!");
}
