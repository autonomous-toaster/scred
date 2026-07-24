    #[test]
    fn test_incomplete_certificate_no_match() {
        let input = b"-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJAKy11CC";
        let result = detect_ssh_keys(input);
        assert!(
            result.matches.is_empty(),
            "Incomplete certificate (no END marker) should not match"
        );
    }

    #[test]
    fn test_multiple_certificates() {
        let input = b"cert1:\n-----BEGIN CERTIFICATE-----\ndata1\n-----END CERTIFICATE-----\n\ncert2:\n-----BEGIN CERTIFICATE-----\ndata2\n-----END CERTIFICATE-----";
        let result = detect_ssh_keys(input);
        assert!(result.count() >= 2, "Should detect multiple certificates");
    }

    #[test]
    fn test_certificate_in_mixed_content() {
        let input = "# TLS Configuration\nCertificate:\n-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJAKy11CCC\n-----END CERTIFICATE-----\n# End config";
        let result = detect_ssh_keys(input.as_bytes());
        assert!(
            !result.matches.is_empty(),
            "Certificate in mixed content should be detected"
        );
    }

    #[test]
    fn test_redact_certificates_full() {
        let text =
            b"-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJAKy11CCC\n-----END CERTIFICATE-----";
        let matches = vec![Match::new(0, text.len(), 304)]; // Pattern type 304 = certificate
        let redacted = redact_text(text, &matches);

        // Certificates should be fully redacted with 'x' (consistent with all redaction)
        for (i, &byte) in redacted.iter().enumerate() {
            if i < text.len() {
                assert_eq!(
                    byte, b'x',
                    "Certificate bytes should be redacted with 'x' at position {}",
                    i
                );
            }
        }
        assert_eq!(text.len(), redacted.len(), "Redaction must preserve length");
    }

    // ===== Phase 4c: PGP Key Pattern Tests =====

    #[test]
    fn test_detect_pgp_private_key_block() {
        let input = b"-----BEGIN PGP PRIVATE KEY BLOCK-----\nVersion: GnuPG v1\nhQEMA5qETJX5s6SUAQf+MQsometestdata\n-----END PGP PRIVATE KEY BLOCK-----";
        let result = detect_ssh_keys(input);
        assert!(
            !result.matches.is_empty(),
            "PGP private key block should be detected"
        );
    }

