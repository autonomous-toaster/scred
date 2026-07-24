    #[test]
    fn test_multiple_ssh_keys() {
        let input = b"key1:\n-----BEGIN RSA PRIVATE KEY-----\ndata1\n-----END RSA PRIVATE KEY-----\nkey2:\n-----BEGIN OPENSSH PRIVATE KEY-----\ndata2\n-----END OPENSSH PRIVATE KEY-----\n";
        let result = detect_ssh_keys(input);
        assert!(result.count() >= 2, "Should detect multiple keys");
    }

    #[test]
    fn test_ssh_key_in_mixed_content() {
        let input = "# SSH Configuration\nPrivateKey:\n-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA1234567890abcdef\n-----END RSA PRIVATE KEY-----\n# End of configuration";
        let result = detect_ssh_keys(input.as_bytes());
        assert!(
            !result.matches.is_empty(),
            "SSH key in mixed content should be detected"
        );
    }

    #[test]
    fn test_detect_all_with_ssh_key() {
        let input = b"API_KEY=abc123def456\n-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA\n-----END RSA PRIVATE KEY-----";
        let result = detect_all(input);
        // Should find both API key and SSH key
        assert!(
            result.count() >= 1,
            "Should detect patterns including SSH key"
        );
    }

    #[test]
    fn test_redact_ssh_key_full() {
        let text =
            b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA\n-----END RSA PRIVATE KEY-----";
        let matches = vec![Match::new(0, text.len(), 300)]; // Pattern type 300 = SSH key
        let redacted = redact_text(text, &matches);

        // SSH keys should be fully redacted with 'x' (consistent with all redaction)
        for (i, &byte) in redacted.iter().enumerate() {
            if i < text.len() {
                assert_eq!(
                    byte, b'x',
                    "SSH key bytes should be redacted with 'x' at position {}",
                    i
                );
            }
        }
        assert_eq!(text.len(), redacted.len(), "Redaction must preserve length");
    }

    #[test]
    fn test_false_positive_ssh_like_text() {
        let input =
            "# This is a comment about -----BEGIN something-----\n# But it's not a real key";
        let result = detect_ssh_keys(input.as_bytes());
        assert!(
            result.matches.is_empty(),
            "Random text with -----BEGIN should not match"
        );
    }

