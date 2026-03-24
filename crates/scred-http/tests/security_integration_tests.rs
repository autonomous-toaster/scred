//! Security Integration Tests
//! Real-world testing of redaction functionality

#[cfg(test)]
mod security_tests {
    use std::process::{Command, Stdio};
    use std::io::Write;
    use std::env;
    use std::path::PathBuf;

    fn get_scred_binary() -> PathBuf {
        let mut path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        path.pop(); // scred-http
        path.pop(); // crates
        path.push("target/release/scred");
        path
    }

    fn run_scred_cli(input: &str) -> String {
        let binary = get_scred_binary();
        
        let mut child = Command::new(&binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("Failed to spawn scred at {:?}: {}", binary, e));

        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
        }

        let output = child.wait_with_output().expect("Failed to wait on child");
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    #[test]
    fn test_aws_key_redaction() {
        let input = "AWS Key: AKIAIOSFODNN7EXAMPLE";
        let output = run_scred_cli(input);
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"), "AWS key leaked: {}", output);
        assert!(output.contains("AKIAxxxxxxxxxxxxxxxx"), "AWS key not properly redacted: {}", output);
    }

    #[test]
    fn test_character_preservation() {
        let input = "Secret: AKIAIOSFODNN7EXAMPLE is here";
        let output = run_scred_cli(input);
        let output_line = output.trim();
        assert_eq!(input.len(), output_line.len(),
                   "Character count not preserved!");
    }

    #[test]
    fn test_json_with_embedded_secret() {
        let input = r#"{"password":"AKIAIOSFODNN7EXAMPLE","user":"admin"}"#;
        let output = run_scred_cli(input);
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(output.contains("password"));
        assert!(output.contains("user"));
    }

    #[test]
    fn test_no_false_positives_on_random_data() {
        let input = "Random data: not_a_real_key_12345";
        let output = run_scred_cli(input);
        assert_eq!(input, output.trim(), "False positive redaction detected");
    }

    #[test]
    fn test_partial_key_not_redacted() {
        let input = "AKIA (partial AWS key format)";
        let output = run_scred_cli(input);
        assert_eq!(input, output.trim(), "Partial key should not be redacted");
    }

    #[test]
    fn test_large_input_with_multiple_secrets() {
        // Create input with small number of large secrets (20 char AWS keys)
        let input = "start AKIAIOSFODNN7EXAMPLE middle AKIAIOSFODNN7EXAMPLE end";
        let output = run_scred_cli(input);
        
        // Both keys should be redacted
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"),
                "AWS keys not redacted in large input");
        
        // Character count preserved
        assert_eq!(input.len(), output.trim().len());
    }

    #[test]
    fn test_pattern_at_line_boundaries() {
        let input = "Line1\nAKIAIOSFODNN7EXAMPLE\nLine3";
        let output = run_scred_cli(input);
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_authorization_bearer_token() {
        let input = "Authorization: Bearer sk_test_4eC39HqLyjWDarhtT1ZdV7dn";
        let output = run_scred_cli(input);
        assert!(output.contains("Authorization"));
        assert!(output.contains("Bearer"));
        assert_eq!(input.len(), output.trim().len());
    }

    #[test]
    fn test_mongodb_connection_string() {
        let input = "mongodb://user:AKIAIOSFODNN7EXAMPLE@host:27017/db";
        let output = run_scred_cli(input);
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(output.contains("mongodb://"));
        assert!(output.contains("@host"));
    }

    #[test]
    fn test_nested_secrets() {
        let input = r#"{"url":"https://api.example.com?key=AKIAIOSFODNN7EXAMPLE"}"#;
        let output = run_scred_cli(input);
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(output.contains("api.example.com"));
    }

    #[test]
    fn test_whitespace_preservation() {
        let input = "Secret:    AKIAIOSFODNN7EXAMPLE    with_spaces";
        let output = run_scred_cli(input);
        assert_eq!(input.len(), output.trim().len());
    }
}
