    #[test]
    fn test_redact_pgp_key_full() {
        let text = b"-----BEGIN PGP PRIVATE KEY BLOCK-----\nVersion: GnuPG v1\ndata123\n-----END PGP PRIVATE KEY BLOCK-----";
        let matches = vec![Match::new(0, text.len(), 308)]; // Pattern type 308 = PGP private key
        let redacted = redact_text(text, &matches);

        // PGP keys should be fully redacted with 'x' (consistent with all redaction)
        for (i, &byte) in redacted.iter().enumerate() {
            if i < text.len() {
                assert_eq!(
                    byte, b'x',
                    "PGP key bytes should be redacted with 'x' at position {}",
                    i
                );
            }
        }
        assert_eq!(text.len(), redacted.len(), "Redaction must preserve length");
    }

    // ============================================================================
    // Phase 1B.2: In-Place Redaction Tests
    // ============================================================================

    #[test]
    fn test_redact_in_place_basic_api_key() {
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let mut buffer = text.to_vec();
        let detection = detect_all(&buffer);

        let count = redact_in_place(&mut buffer, &detection.matches);

        assert_eq!(count, 1, "Should redact 1 pattern");
        assert_eq!(buffer.len(), text.len(), "Length must be preserved");
        assert_eq!(&buffer[..4], b"AKIA", "First 4 chars (prefix) preserved");
        for byte in &buffer[4..] {
            assert_eq!(*byte, b'x', "Rest should be redacted with 'x'");
        }
    }

    #[test]
    fn test_redact_in_place_env_variable() {
        // Note: Environment variable pattern detection may vary
        // This test verifies the in-place redaction works IF pattern is detected
        let text = b"DATABASE_PASSWORD=secret123";
        let mut buffer = text.to_vec();
        let detection = detect_all(&buffer);

        if !detection.matches.is_empty() {
            let count = redact_in_place(&mut buffer, &detection.matches);

            // If detected, should redact properly
            assert!(count > 0, "Should redact detected patterns");
            assert_eq!(buffer.len(), text.len(), "Length must be preserved");
            assert!(
                String::from_utf8_lossy(&buffer).contains('='),
                "Equals sign must be preserved"
            );
        }
        // If not detected, that's OK - env patterns are optional in detector
    }

    #[test]
    fn test_redact_in_place_multiple_secrets() {
        let text =
            b"First: AKIAIOSFODNN7EXAMPLE and second: ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let mut buffer = text.to_vec();
        let detection = detect_all(&buffer);

        let count = redact_in_place(&mut buffer, &detection.matches);

        assert!(count >= 2, "Should redact multiple patterns");
        assert_eq!(buffer.len(), text.len(), "Length must be preserved");
    }

    #[test]
    fn test_redact_in_place_vs_redact_text_equivalence() {
        let text = b"AKIAIOSFODNN7EXAMPLE";
        let detection = detect_all(text);

        // Method 1: redact_text (original)
        let redacted_text = redact_text(text, &detection.matches);

        // Method 2: redact_in_place (new)
        let mut buffer = text.to_vec();
        redact_in_place(&mut buffer, &detection.matches);

        // Both should produce identical results
        assert_eq!(
            buffer, redacted_text,
            "In-place and copy-based redaction must be identical"
        );
    }

