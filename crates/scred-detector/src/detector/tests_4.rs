    #[test]
    fn test_redact_env_multiple_in_text() {
        // Test multiple environment variables in same text
        let text = b"PGPASS=6577abc123 and SERVICE_API_KEY=sk_test_123456";
        let matches = vec![
            Match::new(0, 17, 0),  // PGPASS=6577abc123
            Match::new(22, 52, 0), // SERVICE_API_KEY=sk_test_123456
        ];
        let redacted = redact_text(text, &matches);

        // First: "PGPASS=6577xxxxxxxxx", second: "SERVICE_API_KEY=sk_txxxxxxxxxxxxxxx"
        // Middle " and " should be unchanged
        assert_eq!(
            redacted,
            b"PGPASS=6577xxxxxx and SERVICE_API_KEY=sk_txxxxxxxxxx"
        );
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_non_env_still_works() {
        // Ensure non-environment patterns still work as before
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // No '=' in match, so use old behavior: keep first 4, redact rest
        assert_eq!(redacted, b"AKIAxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_github_token_no_equals() {
        let text = b"ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // No '=' sign, use old behavior: keep first 4 chars
        assert_eq!(redacted, b"ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_langsmith_deployment_key() {
        let text = b"lsv2_sk_abcdef1234567890abcdef1234567890abc";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Should keep first 4 chars "lsv2" and redact rest
        assert_eq!(redacted, b"lsv2xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_pgpassword() {
        let text = b"PGPASSWORD=mypassword123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Should keep "PGPASSWORD=" and first 4 of value "mypa", redact rest
        assert_eq!(redacted, b"PGPASSWORD=mypaxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

