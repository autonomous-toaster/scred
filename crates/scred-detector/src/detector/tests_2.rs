    #[test]
    fn test_redact_text_preserves_length() {
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let matches = vec![Match::new(0, 20, 0)];
        let redacted = redact_text(text, &matches);
        assert_eq!(text.len(), redacted.len());
        // First 4 chars (prefix) preserved, rest redacted
        assert_eq!(redacted, b"AKIAxxxxxxxxxxxxxxxx");
    }

    #[test]
    fn test_redact_text_mixed() {
        let text = b"key: AKIAIOSFODNN7 value";
        let matches = vec![Match::new(5, 21, 0)]; // Positions 5-21: "AKIAIOSFODNN7 va" (16 bytes)
        let redacted = redact_text(text, &matches);
        assert_eq!(text.len(), redacted.len());
        // Keep "AKIA" (4 chars), replace "IOSFODNN7 va" (12 chars) with x's
        assert_eq!(redacted, b"key: AKIAxxxxxxxxxxxxlue");
    }

    // Environment variable redaction tests
    #[test]
    #[test]
    fn test_redact_env_client_secret() {
        let text = b"SERVICE_CLIENT_SECRET=abcdef123456";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Keep "SERVICE_CLIENT_SECRET=", first 4 of value "abcd", redact rest
        assert_eq!(redacted, b"SERVICE_CLIENT_SECRET=abcdxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_api_key() {
        let text = b"STRIPE_API_KEY=sk_test_abcd1234567890ef";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Keep "STRIPE_API_KEY=", first 4 of value "sk_t", redact rest
        assert_eq!(redacted, b"STRIPE_API_KEY=sk_txxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

