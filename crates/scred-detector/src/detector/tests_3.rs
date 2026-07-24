    #[test]
    fn test_redact_env_token() {
        let text = b"AUTH_TOKEN=eyJhbGciOiJIUzI1NiJ9abcd1234567890";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Keep "AUTH_TOKEN=", first 4 of value "eyJa", redact rest
        assert_eq!(redacted, b"AUTH_TOKEN=eyJhxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_password() {
        let text = b"DB_PASSWORD=MySecurePassword123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Keep "DB_PASSWORD=", first 4 of value "MySe", redact rest
        assert_eq!(redacted, b"DB_PASSWORD=MySexxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_short_value() {
        // Test with value shorter than 4 characters
        let text = b"API_KEY=abc";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Keep "API_KEY=", value is only 3 chars, preserve all, nothing to redact
        assert_eq!(redacted, b"API_KEY=abc");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_exactly_4_chars() {
        // Test with value exactly 4 characters
        let text = b"KEY=abcd";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Keep "KEY=", preserve all 4 chars of value, nothing to redact
        assert_eq!(redacted, b"KEY=abcd");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_env_with_special_chars_in_value() {
        // Test environment variable with special characters in value
        let text = b"MONGODB_URI=mongodb+srv://user:pass@cluster.mongodb.net/db?param=value";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Keep "MONGODB_URI=", first 4 of value "mong", redact rest
        assert_eq!(
            redacted,
            b"MONGODB_URI=mongxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        );
        assert_eq!(text.len(), redacted.len());
    }

