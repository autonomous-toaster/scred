use regex::Regex;

#[test]
fn test_bcrypt_simple() {
    let input = "$2a$10$R9h/cIPz0gi.URNNX3kh2OPST9/PgBkqquzi.Ss7KIUgO2t0jKm2";
    
    // Extremely simple test first
    let re1 = Regex::new(r"\$2a\$").unwrap();
    assert!(re1.is_match(input), "Can't match $2a$");
    
    // Add more
    let re2 = Regex::new(r"\$2a\$10\$").unwrap();
    assert!(re2.is_match(input), "Can't match $2a$10$");
    
    // Character class test
    let re3 = Regex::new(r"\$2a\$10\$[./A-Za-z0-9]{1}").unwrap();
    assert!(re3.is_match(input), "Can't match character class");
    
    // Full pattern
    let re4 = Regex::new(r"\$2a\$10\$[./A-Za-z0-9]{53}").unwrap();
    assert!(re4.is_match(input), "Full pattern doesn't match");
    
    // Now test the bracket pattern from patterns.zig
    let re5 = Regex::new(r"\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}").unwrap();
    assert!(re5.is_match(input), "Flexible pattern doesn't match");
}
