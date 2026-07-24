    #[test]
    fn test_detect_simple_prefix_aws() {
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let result = detect_simple_prefix(text);
        assert!(result.count() > 0);
        assert_eq!(result.matches[0].start, 0);
        assert!(result.matches[0].end > 4); // At least prefix length
    }

    #[test]
    fn test_detect_simple_prefix_github() {
        let text = b"token ghp_abcdefghijklmnopqrstuvwxyz";
        let result = detect_simple_prefix(text);
        assert!(result.count() > 0);
    }

    #[test]
    fn test_detect_validation_github_detailed() {
        let text = b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let result = detect_validation(text);
        assert!(result.count() > 0, "Should detect github-token-detailed");
    }

    #[test]
    fn test_detect_jwt() {
        let text = b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let result = detect_jwt(text);
        assert!(result.count() > 0, "Should detect JWT");
        assert_eq!(result.matches[0].start, 0);
    }

    #[test]
    fn test_detect_jwt_in_context() {
        let text =
            b"Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.X3XL0MU4p0Xz5W1Z6KvK";
        let result = detect_jwt(text);
        assert!(result.count() > 0);
    }

