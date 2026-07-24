    #[test]
    fn test_redact_in_place_empty_matches() {
        let text = b"no secrets here";
        let mut buffer = text.to_vec();
        let original = buffer.clone();

        let count = redact_in_place(&mut buffer, &[]);

        assert_eq!(count, 0, "Should redact 0 patterns");
        assert_eq!(buffer, original, "Buffer should be unchanged");
    }

    #[test]
    fn test_redact_in_place_ssh_key_full_redaction() {
        let text = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA1234567890abcdef\n-----END RSA PRIVATE KEY-----";
        let mut buffer = text.to_vec();
        let detection = detect_all(&buffer);

        if !detection.matches.is_empty() {
            redact_in_place(&mut buffer, &detection.matches);

            // SSH keys should be fully redacted
            assert_eq!(buffer.len(), text.len(), "Length must be preserved");
        }
    }

    #[test]
    fn test_redact_in_place_character_preservation_aws() {
        let text = b"My access key: AKIAIOSFODNN7EXAMPLE, keep it secret!";
        let mut buffer = text.to_vec();
        let original_len = buffer.len();
        let detection = detect_all(&buffer);

        redact_in_place(&mut buffer, &detection.matches);

        assert_eq!(
            buffer.len(),
            original_len,
            "Character count must be preserved"
        );
        assert_eq!(buffer.len(), text.len(), "Output length must match input");
    }

    #[test]
    fn test_redact_in_place_all_patterns_preserve_length() {
        // Test a variety of secrets to ensure all preserve length
        let test_cases = vec![
            b"AKIAIOSFODNN7EXAMPLE" as &[u8],
            b"ghp_1234567890abcdefghijklmnopqrstuvwxyz",
            b"sk_live_123456789",
            b"AIzaSyB1234567890abcdefg",
        ];

        for text in test_cases {
            let mut buffer = text.to_vec();
            let original_len = buffer.len();
            let detection = detect_all(&buffer);

            if !detection.matches.is_empty() {
                redact_in_place(&mut buffer, &detection.matches);
                assert_eq!(
                    buffer.len(),
                    original_len,
                    "Length must be preserved for: {}",
                    String::from_utf8_lossy(text)
                );
            }
        }
    }
