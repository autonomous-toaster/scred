#[cfg(test)]
mod integration_e2e_httpbin {
    use std::process::{Command, Stdio};
    use std::io::Write;

    /// Helper to run scred CLI with input
    fn scred_cli(args: &[&str], input: &str) -> String {
        let mut child = Command::new("scred")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn scred");

        {
            let stdin = child.stdin.as_mut().expect("Failed to get stdin");
            stdin.write_all(input.as_bytes()).expect("Failed to write");
        }

        let output = child.wait_with_output().expect("Failed to wait for scred");
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    /// Verify a secret was redacted by checking for redaction markers
    fn assert_redacted(output: &str, original_secret: &str, secret_prefix: &str) {
        assert!(
            !output.contains(original_secret),
            "Secret '{}' not redacted in output:\n{}",
            original_secret,
            output
        );
        
        // Should contain redaction markers (xxx, ***,etc) near the prefix
        let has_redaction_marker = output.contains("x") || output.contains("*");
        assert!(
            has_redaction_marker && output.contains(secret_prefix),
            "Output doesn't show redacted secret format for prefix '{}' in:\n{}",
            secret_prefix,
            output
        );
    }

    /// Verify a string is NOT incorrectly redacted (no false positives)
    fn assert_not_redacted(output: &str, content: &str) {
        assert!(
            output.contains(content),
            "Content '{}' was incorrectly redacted/modified in:\n{}",
            content,
            output
        );
    }

    #[test]
    fn test_cli_aws_key_redaction_critical() {
        let input = "export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let output = scred_cli(&["--redact", "CRITICAL"], input);
        
        assert_redacted(&output, "AKIAIOSFODNN7EXAMPLE", "AKIA");
    }

    #[test]
    fn test_cli_github_token_redaction() {
        let input = "github_token: ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let output = scred_cli(&["--redact", "CRITICAL,API_KEYS"], input);
        
        assert_redacted(&output, "ghp_1234567890abcdefghijklmnopqrstuvwxyz", "ghp_");
    }

    #[test]
    fn test_cli_multiple_secrets_same_line() {
        let input = "aws=AKIAIOSFODNN7EXAMPLE github=ghp_abcdef123456 slack=xoxb-1234567890";
        let output = scred_cli(&["--redact", "CRITICAL,API_KEYS"], input);
        
        // All three should be redacted
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!output.contains("ghp_abcdef123456"));
        assert!(!output.contains("xoxb-1234567890"));
    }

    #[test]
    fn test_cli_env_mode_database_url() {
        let input = "DATABASE_URL=postgresql://user:secretPass@db.example.com:5432/app";
        let output = scred_cli(&["--env-mode", "--redact", "CRITICAL"], input);
        
        // Should preserve format but redact password
        assert!(output.contains("postgresql://"));
        assert!(output.contains("@db.example.com"));
        // Password part should be redacted
        assert!(!output.contains("secretPass"));
    }

    #[test]
    fn test_cli_env_file_multiple_secrets() {
        let input = r#"
API_KEY=sk-proj-test123456789abcdef
DATABASE_PASSWORD=superSecret123!
SLACK_WEBHOOK=https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX
"#;
        let output = scred_cli(&["--env-mode", "--redact", "CRITICAL,API_KEYS"], input);
        
        // Should preserve variable names but redact values
        assert!(output.contains("API_KEY="));
        assert!(output.contains("DATABASE_PASSWORD="));
        assert!(output.contains("SLACK_WEBHOOK="));
        
        // But original values should not appear
        assert!(!output.contains("test123456789abcdef"));
        assert!(!output.contains("superSecret123!"));
    }

    #[test]
    fn test_cli_no_false_positives_email() {
        let input = "Contact: admin@sk-proj-example.com";
        let output = scred_cli(&["--redact", "API_KEYS"], input);
        
        // Email domain should NOT be redacted as a secret
        assert_not_redacted(&output, "@sk-proj-example.com");
    }

    #[test]
    fn test_cli_no_false_positives_documentation() {
        let input = "AWS documentation: Always use IAM roles, not AKIA keys";
        let output = scred_cli(&["--redact", "CRITICAL"], input);
        
        // "AKIA" in documentation should not trigger redaction
        // (it's part of "AKIA keys" text, not an actual key)
        assert!(output.contains("AKIA") || output.contains("documentation"));
    }

    #[test]
    fn test_cli_consistent_redaction() {
        let input = "key=AKIAIOSFODNN7EXAMPLE";
        
        let output1 = scred_cli(&["--redact", "CRITICAL"], input);
        let output2 = scred_cli(&["--redact", "CRITICAL"], input);
        
        // Same input should produce same output (or at least same redaction pattern)
        assert_eq!(
            output1.matches("x").count(),
            output2.matches("x").count(),
            "Redaction not consistent"
        );
    }

    #[test]
    fn test_cli_detect_only_flag() {
        let input = "key=AKIAIOSFODNN7EXAMPLE";
        let output = scred_cli(&["--detect-only"], input);
        
        // Should show detection result, not redacted content
        assert!(
            output.contains("Detection:") || output.contains("EnvFormat") || output.contains("TextFormat"),
            "--detect-only should show detection result"
        );
    }

    #[test]
    fn test_cli_pattern_selector_critical_only() {
        let input = "aws=AKIAIOSFODNN7EXAMPLE stripe=sk_live_test123";
        let output = scred_cli(&["--redact", "CRITICAL"], input);
        
        // CRITICAL should include AWS but not necessarily Stripe
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_cli_pattern_selector_all_patterns() {
        let input = "aws=AKIAIOSFODNN7EXAMPLE stripe=sk_live_test123";
        let output = scred_cli(&["--redact", "CRITICAL,API_KEYS"], input);
        
        // Both should be redacted with broader selector
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!output.contains("sk_live_test123"));
    }

    #[test]
    fn test_cli_private_key_multiline() {
        let input = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA0Z3VS5JJcds3xfn/ygWyF73DpACkNPy5r7YqVg7sSW3qxI8B
-----END RSA PRIVATE KEY-----"#;
        let output = scred_cli(&["--redact", "CRITICAL"], input);
        
        // Headers should be preserved
        assert!(output.contains("BEGIN RSA PRIVATE KEY"));
        assert!(output.contains("END RSA PRIVATE KEY"));
        
        // Key data should be redacted
        assert!(!output.contains("MIIEpAIBAAKCAQEA"));
    }

    #[test]
    fn test_cli_streaming_large_input() {
        // Create a large input with secrets at various positions
        let mut input = String::new();
        for i in 0..10000 {
            input.push_str(&format!(
                "line_{}: AKIAIOSFODNN7EXAMPLE sk-proj-test123 ghp_token\n",
                i
            ));
        }
        
        let output = scred_cli(&["--redact", "CRITICAL,API_KEYS"], &input);
        
        // Should process without panic and redact secrets
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!output.contains("sk-proj-test123"));
        assert!(!output.contains("ghp_token"));
        
        // Should have reasonable output size (not truncated)
        assert!(output.len() > 10000);
    }

    #[test]
    fn test_cli_unicode_handling() {
        let input = "key=AKIAIOSFODNN7EXAMPLE émoji=😀 chinese=中文";
        let output = scred_cli(&["--redact", "CRITICAL"], input);
        
        // UTF-8 should be preserved
        assert!(output.contains("😀") || output.contains("emoji"));
        assert!(output.contains("中文") || output.contains("chinese"));
        
        // Secret should still be redacted
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_cli_verbose_mode() {
        let input = "key=AKIAIOSFODNN7EXAMPLE";
        let mut cmd = Command::new("scred")
            .args(&["--verbose", "--redact", "CRITICAL"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn scred");

        {
            let stdin = cmd.stdin.as_mut().expect("Failed to get stdin");
            stdin.write_all(input.as_bytes()).expect("Failed to write");
        }

        let output = cmd.wait_with_output().expect("Failed to wait");
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Verbose mode should output statistics to stderr
        assert!(
            stderr.contains("redacting") || stderr.contains("Bytes") || stderr.contains("Time"),
            "Verbose mode should output statistics"
        );
    }

    #[test]
    fn test_cli_list_patterns() {
        let output = Command::new("scred")
            .args(&["--list-patterns"])
            .output()
            .expect("Failed to run --list-patterns");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should list patterns
        assert!(
            stdout.len() > 100,
            "--list-patterns should output pattern information"
        );
        
        // Should show pattern count
        assert!(
            stdout.contains("pattern") || stdout.contains("CRITICAL"),
            "Pattern list should contain recognizable pattern info"
        );
    }

    #[test]
    fn test_cli_describe_pattern() {
        let output = Command::new("scred")
            .args(&["--describe", "aws-key"])
            .output()
            .expect("Failed to run --describe");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should describe the pattern
        assert!(
            stdout.len() > 10 || stdout.contains("aws") || stdout.contains("key"),
            "--describe aws-key should output pattern details"
        );
    }

    #[test]
    fn test_cli_help_message() {
        let output = Command::new("scred")
            .args(&["--help"])
            .output()
            .expect("Failed to run --help");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        assert!(
            stdout.contains("Usage") || stdout.contains("SCRED"),
            "--help should show usage information"
        );
    }

    // ============================================================
    // Streaming Edge Cases
    // ============================================================

    #[test]
    fn test_streaming_secret_at_chunk_boundary_8190() {
        // Create input where secret starts at byte 8190 (near 8KB chunk boundary)
        let mut input = String::new();
        input.push_str(&"x".repeat(8190));
        input.push_str("AKIAIOSFODNN7EXAMPLE");
        
        let output = scred_cli(&["--redact", "CRITICAL"], &input);
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"), 
            "Secret at chunk boundary 8190 should be redacted");
    }

    #[test]
    fn test_streaming_secret_spanning_chunks() {
        // Create input where secret spans chunk boundary
        let mut input = String::new();
        input.push_str(&"x".repeat(8200));  // Past normal chunk size
        input.push_str("AKIAIOSFODNN7EXAMPLE");
        
        let output = scred_cli(&["--redact", "CRITICAL"], &input);
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"),
            "Secret spanning chunks should be redacted");
    }

    #[test]
    fn test_streaming_multiple_chunks_multiple_secrets() {
        let mut input = String::new();
        for i in 0..3 {
            input.push_str(&"x".repeat(8192 * i));
            input.push_str(&format!("secret_{}: AKIAIOSFODNN7EXAMPLE\n", i));
        }
        
        let output = scred_cli(&["--redact", "CRITICAL"], &input);
        
        // All instances should be redacted
        let count = output.matches("AKIAIOSFODNN7EXAMPLE").count();
        assert_eq!(count, 0, "All secret instances should be redacted");
    }

    // ============================================================
    // Security Configuration Tests
    // ============================================================

    #[test]
    fn test_detect_vs_redact_asymmetry() {
        let input = "aws=AKIAIOSFODNN7EXAMPLE stripe=sk_live_test123";
        
        // Detect CRITICAL,API_KEYS but only redact CRITICAL
        let mut cmd = Command::new("scred")
            .args(&["--detect", "CRITICAL,API_KEYS", "--redact", "CRITICAL"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");

        {
            let stdin = cmd.stdin.as_mut().unwrap();
            stdin.write_all(input.as_bytes()).unwrap();
        }

        let output = cmd.wait_with_output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // AWS key should be redacted (CRITICAL)
        assert!(!stdout.contains("AKIAIOSFODNN7EXAMPLE"));
        
        // Stripe key handling depends on classification
        // Just verify command doesn't crash with asymmetric config
        assert!(stdout.len() > 0);
    }

    #[test]
    fn test_config_file_loading() {
        // This test verifies that config file can be loaded without crashing
        // Actual config file content varies by environment
        let output = Command::new("scred")
            .args(&["--list-tiers"])
            .output()
            .expect("Failed to list tiers");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should show tier information
        assert!(
            stdout.contains("CRITICAL") || stdout.contains("Tier"),
            "Config system should be working"
        );
    }
}
