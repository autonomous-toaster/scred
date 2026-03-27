//! Test for OpenSSH private key detection
//! Issue: -----BEGIN OPENSSH PRIVATE KEY----- on multiline is not detected and redacted

#[cfg(test)]
mod openssh_key_tests {
    use scred_readctor_framering::RedactionEngine;

    #[test]
    fn test_openssh_private_key_simple() {
        // Single-line format (should be easier to detect)
        let test = r#"-----BEGIN OPENSSH PRIVATE KEY-----"#;
        
        let config = Default::default();
        let engine = RedactionEngine::new(config);
        
        let result = engine.redact(test).redacted;
        
        // Check if any redaction happened
        if result == test {
            eprintln!("WARNING: OpenSSH key marker NOT redacted");
            eprintln!("Input:  {}", test);
            eprintln!("Output: {}", result);
        }
        
        println!("Simple marker test: {}", if result != test { "PASS" } else { "FAIL" });
    }

    #[test]
    fn test_openssh_private_key_multiline_short() {
        // Minimal multiline format
        let test = r#"-----BEGIN OPENSSH PRIVATE KEY-----
abc123xyz
-----END OPENSSH PRIVATE KEY-----"#;
        
        let config = Default::default();
        let engine = RedactionEngine::new(config);
        
        let result = engine.redact(test).redacted;
        
        // Check if any redaction happened
        if result == test {
            eprintln!("WARNING: Multiline OpenSSH key NOT redacted");
            eprintln!("Input:\n{}", test);
            eprintln!("\nOutput:\n{}", result);
        }
        
        println!("Multiline test: {}", if result != test { "PASS" } else { "FAIL" });
    }

    #[test]
    fn test_openssh_private_key_full() {
        // Full realistic format
        let test = r#"-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUtbm9uZS1ub25lAAAAiQAAAAdzc2gtcnNhAAAA
awAAAIEAu0A34sFb3y8cEhc7tKl9VLgXZv/PoqGgP7WF4C1S0fIlbZy5cOWzJEhHScnV
Fs1H6vGlXvHLK6PgfLJX/8VEb+JrPvhakM3mBJt3K0kcS6vgPyb3OoV8Vt0Wq5j6/KqN
AAAI0HmUHCR5lBwkQQAAAIEAu0A34sFb3y8cEhc7tKl9VLgXZv/PoqGgP7WF4C1S0fIl
-----END OPENSSH PRIVATE KEY-----"#;
        
        let config = Default::default();
        let engine = RedactionEngine::new(config);
        
        let result = engine.redact(test).redacted;
        
        // Check if any redaction happened
        if result == test {
            eprintln!("WARNING: Full OpenSSH key NOT redacted");
            eprintln!("Expected redaction but got no changes");
        } else {
            eprintln!("SUCCESS: OpenSSH key redacted");
            eprintln!("Output:\n{}", result);
        }
        
        assert_ne!(result, test, "OpenSSH private key should be redacted!");
    }

    #[test]
    fn test_rsa_private_key() {
        // Standard RSA format (should also be caught)
        let test = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA0Z3VS5JJcds3xfn/
-----END RSA PRIVATE KEY-----"#;
        
        let config = Default::default();
        let engine = RedactionEngine::new(config);
        
        let result = engine.redact(test).redacted;
        
        assert_ne!(result, test, "RSA private key should be redacted!");
    }

    #[test]
    fn test_ec_private_key() {
        // EC key format
        let test = r#"-----BEGIN EC PRIVATE KEY-----
MHcCAQEEIGV8/LxjyM+IJIJ2pNCPwA
-----END EC PRIVATE KEY-----"#;
        
        let config = Default::default();
        let engine = RedactionEngine::new(config);
        
        let result = engine.redact(test).redacted;
        
        assert_ne!(result, test, "EC private key should be redacted!");
    }
}
