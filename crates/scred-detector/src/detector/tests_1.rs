    #[test]
    fn test_detect_jwt_large_payload_entraid() {
        // EntraID token with large payload: [84, 1063, 342] segment lengths
        // Total: ~1491 bytes (well within 10000 byte limit)
        let header = b"eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9"; // 36 bytes
        let payload = b"eyJkYXRhIjoieHh4eHh4eHg"; // Start with eyJ to simulate large payload
        let signature = vec![b'A'; 342]; // 342 bytes of valid base64url

        // Build token: header.payload.signature
        let mut token = Vec::new();
        token.extend_from_slice(header);
        token.push(b'.');
        token.extend_from_slice(payload);
        // Pad payload to ~1063 bytes with valid base64url chars
        token.extend_from_slice(&vec![b'x'; 1063 - payload.len()]);
        token.push(b'.');
        token.extend_from_slice(&signature);

        let result = detect_jwt(&token);
        assert!(
            result.count() > 0,
            "Should detect large JWT token with payload ~1063 bytes"
        );
        assert_eq!(result.matches[0].start, 0, "JWT should start at position 0");
        assert_eq!(
            result.matches[0].end,
            token.len(),
            "JWT should span entire token"
        );
    }

    #[test]
    fn test_detect_jwt_exact_entraid_structure() {
        // Test with exact segment lengths: [84, 1063, 342]
        // Construct with all valid base64url characters
        let mut token = Vec::new();

        // Header: 84 bytes starting with "eyJ"
        token.extend_from_slice(b"eyJ");
        token.extend_from_slice(&vec![b'A'; 81]); // 81 + 3 = 84 bytes
        token.push(b'.');

        // Payload: 1063 bytes of valid base64url
        token.extend_from_slice(&vec![b'B'; 1063]);
        token.push(b'.');

        // Signature: 342 bytes of valid base64url
        token.extend_from_slice(&vec![b'C'; 342]);

        let result = detect_jwt(&token);
        assert!(
            result.count() > 0,
            "Should detect JWT with exact [84, 1063, 342] structure"
        );
        assert_eq!(result.matches[0].start, 0);
        assert_eq!(result.matches[0].end, token.len());
    }

    #[test]
    fn test_detect_jwt_with_context_boundaries() {
        // Test JWT embedded in text with potential boundary issues
        let mut text = Vec::new();
        text.extend_from_slice(b"token: ");

        // Add JWT
        text.extend_from_slice(b"eyJ");
        text.extend_from_slice(&vec![b'A'; 81]);
        text.push(b'.');
        text.extend_from_slice(&vec![b'B'; 1063]);
        text.push(b'.');
        text.extend_from_slice(&vec![b'C'; 342]);

        text.extend_from_slice(b" end");

        let result = detect_jwt(&text);
        assert!(
            result.count() > 0,
            "Should detect JWT even with surrounding text"
        );
        // JWT should start after "token: "
        assert_eq!(result.matches[0].start, 7);
    }

    #[test]
    fn test_detect_all_mixed() {
        let text =
            b"AWS Key: AKIAIOSFODNN7EXAMPLE GitHub: ghp_abcd1234 JWT: eyJhbGc.eyJzdWI.SflKxw";
        let result = detect_all(text);
        assert!(result.count() >= 2, "Should detect multiple patterns");
    }

    #[test]
    fn test_detect_no_false_positives() {
        let text = b"This is normal text with numbers 12345 and letters abcde";
        let result = detect_all(text);
        // Should not detect random text
        assert_eq!(result.count(), 0);
    }

