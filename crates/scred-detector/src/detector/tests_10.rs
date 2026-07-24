    #[test]
    fn test_ssh_key_without_newline_after_end() {
        // SSH key at end of file without trailing newline
        let input = b"-----BEGIN PRIVATE KEY-----\ndata123\n-----END PRIVATE KEY-----";
        let result = detect_ssh_keys(input);
        assert!(
            !result.matches.is_empty(),
            "SSH key without trailing newline should still match"
        );
    }

    // ===== Phase 4b: Certificate Pattern Tests =====

    #[test]
    fn test_detect_x509_certificate() {
        let input = b"-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJAKy11CCCCBDMA0G\n-----END CERTIFICATE-----";
        let result = detect_ssh_keys(input);
        assert!(
            !result.matches.is_empty(),
            "X.509 certificate should be detected"
        );
        assert!(result.matches[0].end > 30, "Should cover full certificate");
    }

    #[test]
    fn test_detect_certificate_request() {
        let input = b"-----BEGIN CERTIFICATE REQUEST-----\nMIICljCCAX4CAQAwDQYJKoZIhvcNAQEBBQAw\n-----END CERTIFICATE REQUEST-----";
        let result = detect_ssh_keys(input);
        assert!(
            !result.matches.is_empty(),
            "Certificate request should be detected"
        );
    }

    #[test]
    fn test_detect_encrypted_private_key() {
        let input = b"-----BEGIN ENCRYPTED PRIVATE KEY-----\nMIIFHDBOBgkqhkiG9w0BBQ0wQTApBgkqhkiG9w0BBQwwHAYIKwYBBQUHAwIECJ+C\n-----END ENCRYPTED PRIVATE KEY-----";
        let result = detect_ssh_keys(input);
        assert!(
            !result.matches.is_empty(),
            "Encrypted private key should be detected"
        );
    }

    #[test]
    fn test_detect_public_key() {
        let input = b"-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA1234567890\n-----END PUBLIC KEY-----";
        let result = detect_ssh_keys(input);
        assert!(!result.matches.is_empty(), "Public key should be detected");
    }

