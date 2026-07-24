    #[test]
    fn test_detect_pgp_public_key_block() {
        let input = b"-----BEGIN PGP PUBLIC KEY BLOCK-----\nVersion: GnuPG v1\nmQGiBDoxrZ0RBADZ\n-----END PGP PUBLIC KEY BLOCK-----";
        let result = detect_ssh_keys(input);
        assert!(
            !result.matches.is_empty(),
            "PGP public key block should be detected"
        );
    }

    #[test]
    fn test_detect_pgp_message() {
        let input = b"-----BEGIN PGP MESSAGE-----\nVersion: GnuPG v1\nwcDMA5qETJX5s6SUAQf+MQsometestencrypted\n-----END PGP MESSAGE-----";
        let result = detect_ssh_keys(input);
        assert!(
            !result.matches.is_empty(),
            "PGP encrypted message should be detected"
        );
    }

    #[test]
    fn test_incomplete_pgp_key_no_match() {
        let input =
            b"-----BEGIN PGP PRIVATE KEY BLOCK-----\nVersion: GnuPG v1\nhQEMA5qETJX5s6SUAQf";
        let result = detect_ssh_keys(input);
        assert!(
            result.matches.is_empty(),
            "Incomplete PGP key (no END marker) should not match"
        );
    }

    #[test]
    fn test_multiple_pgp_keys() {
        let input = b"key1:\n-----BEGIN PGP PUBLIC KEY BLOCK-----\ndata1\n-----END PGP PUBLIC KEY BLOCK-----\nkey2:\n-----BEGIN PGP PRIVATE KEY BLOCK-----\ndata2\n-----END PGP PRIVATE KEY BLOCK-----";
        let result = detect_ssh_keys(input);
        assert!(result.count() >= 2, "Should detect multiple PGP keys");
    }

    #[test]
    fn test_pgp_in_mixed_content() {
        let input = "# PGP Key Storage\nPrivate Key:\n-----BEGIN PGP PRIVATE KEY BLOCK-----\nVersion: GnuPG\ndata123data456\n-----END PGP PRIVATE KEY BLOCK-----\n# End storage";
        let result = detect_ssh_keys(input.as_bytes());
        assert!(
            !result.matches.is_empty(),
            "PGP key in mixed content should be detected"
        );
    }

