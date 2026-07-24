    #[test]
    fn test_pgpass_vs_pgpassword() {
        // Test that both patterns work correctly in different contexts
        let text1 = b"PGPASS=abc123456";
    }

    // ============================================================================
    // SSH KEY DETECTION TESTS
    // ============================================================================

    #[test]
    fn test_detect_ssh_rsa_key() {
        let input = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA1234567890\n-----END RSA PRIVATE KEY-----\n";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "SSH RSA key should be detected");
        assert_eq!(result.matches[0].start, 0);
        assert!(result.matches[0].end > 30, "Should cover full key");
    }

    #[test]
    fn test_detect_ssh_openssh_key() {
        let input = b"-----BEGIN OPENSSH PRIVATE KEY-----\nAAAAB3NzaC1yc2EAAA\n-----END OPENSSH PRIVATE KEY-----";
        let result = detect_ssh_keys(input);
        assert!(
            !result.matches.is_empty(),
            "SSH OpenSSH key should be detected"
        );
    }

    #[test]
    fn test_ssh_ec_private_key() {
        let input =
            b"-----BEGIN EC PRIVATE KEY-----\nMHcCAQEEIIGlVdZfvfg\n-----END EC PRIVATE KEY-----";
        let result = detect_ssh_keys(input);
        assert!(
            !result.matches.is_empty(),
            "EC private key should be detected"
        );
    }

    #[test]
    fn test_incomplete_ssh_key_no_match() {
        let input = b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpQIBAAKCAQEA";
        let result = detect_ssh_keys(input);
        assert!(
            result.matches.is_empty(),
            "Incomplete key (no END marker) should not match"
        );
    }

