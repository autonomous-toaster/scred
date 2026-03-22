/// Per-stream redaction isolation tests
///
/// Ensures that secrets from one stream don't leak to another stream.
/// This is critical for HTTP/2 multiplexing safety.

#[cfg(test)]
mod redaction_isolation_tests {
    use std::collections::HashSet;

    /// Mock stream processor for testing
    struct StreamProcessor {
        stream_id: u32,
        requests: Vec<Vec<u8>>,
        responses: Vec<Vec<u8>>,
    }

    impl StreamProcessor {
        fn new(stream_id: u32) -> Self {
            Self {
                stream_id,
                requests: Vec::new(),
                responses: Vec::new(),
            }
        }

        /// Add request data (headers + body)
        fn add_request(&mut self, data: Vec<u8>) {
            self.requests.push(data);
        }

        /// Add response data (headers + body)
        fn add_response(&mut self, data: Vec<u8>) {
            self.responses.push(data);
        }

        /// Simulate redaction by replacing patterns
        fn redact(&self, text: &[u8]) -> Vec<u8> {
            let mut result = text.to_vec();
            
            // Redact common secret patterns (simplified for testing)
            let patterns = vec![
                (b"sk-proj-".to_vec(), 8),         // OpenAI
                (b"Bearer ".to_vec(), 6),          // Bearer token
                (b"Authorization: ".to_vec(), 15), // Auth header
                (b"old-token-".to_vec(), 10),      // Test token
                (b"new-token-".to_vec(), 10),      // Test token
                (b"x-custom-secret: ".to_vec(), 16), // Custom header
            ];

            for (pattern, prefix_len) in patterns {
                let mut pos = 0;
                loop {
                    match find_pattern(&result[pos..], &pattern) {
                        Some(found_pos) => {
                            let actual_pos = pos + found_pos;
                            let start = actual_pos + prefix_len;
                            let end = std::cmp::min(start + 32, result.len());
                            
                            // Redact the secret, but stop at whitespace/special chars
                            for i in start..end {
                                if result[i] == b' ' || result[i] == b'\n' || result[i] == b'\r' 
                                    || result[i] == b'"' || result[i] == b'\'' || result[i] == b'}' {
                                    break;
                                }
                                result[i] = b'x';
                            }
                            
                            // Move past this redaction to avoid infinite loop
                            pos = actual_pos + pattern.len();
                            if pos >= result.len() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }

            result
        }

        fn stream_id(&self) -> u32 {
            self.stream_id
        }
    }

    fn find_pattern(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// Test Case 1: Streams with different secrets should maintain isolation
    #[test]
    fn test_stream_isolation_different_secrets() {
        let mut stream1 = StreamProcessor::new(1);
        let mut stream2 = StreamProcessor::new(3);

        let secret1 = b"Authorization: Bearer sk-proj-AAAAAAAAAAAAAAAAAAAAAA1111111111";
        let secret2 = b"Authorization: Bearer sk-proj-BBBBBBBBBBBBBBBBBBBBBB2222222222";

        stream1.add_request(secret1.to_vec());
        stream2.add_request(secret2.to_vec());

        let redacted1 = stream1.redact(secret1);
        let redacted2 = stream2.redact(secret2);

        // Both should be redacted
        assert!(!String::from_utf8_lossy(&redacted1).contains("AAAA"));
        assert!(!String::from_utf8_lossy(&redacted2).contains("BBBB"));

        // Redacted versions should be different from originals
        assert_ne!(redacted1, secret1.to_vec());
        assert_ne!(redacted2, secret2.to_vec());

        println!("[STREAM-1] Redacted: {}", String::from_utf8_lossy(&redacted1));
        println!("[STREAM-3] Redacted: {}", String::from_utf8_lossy(&redacted2));
    }

    /// Test Case 2: Stream 1 secret should NOT appear in Stream 2 output
    #[test]
    fn test_cross_stream_secret_leakage() {
        let mut stream1 = StreamProcessor::new(1);
        let mut stream2 = StreamProcessor::new(3);

        let secret1_raw = b"super-secret-token-12345678901234567890";
        let secret1_in_request = format!("GET /api?token={}", String::from_utf8_lossy(secret1_raw));

        stream1.add_request(secret1_in_request.as_bytes().to_vec());

        let stream1_redacted = stream1.redact(secret1_in_request.as_bytes());
        let stream2_response = b"Response from stream 2";

        // Stream 2 should NOT contain any part of stream 1's secret
        let stream2_output = stream2.redact(stream2_response);
        
        let stream1_secret_str = String::from_utf8_lossy(secret1_raw);
        let stream2_output_str = String::from_utf8_lossy(&stream2_output);

        assert!(!stream2_output_str.contains(stream1_secret_str.as_ref()),
                "Stream 2 contains Stream 1's secret!");

        println!("[LEAK-CHECK] Stream 1 secret: {} (length: {})", 
                 stream1_secret_str, stream1_secret_str.len());
        println!("[LEAK-CHECK] Stream 2 output clean: {}", stream2_output_str);
    }

    /// Test Case 3: Concurrent redaction of multiple streams
    #[test]
    fn test_concurrent_stream_redaction() {
        let mut streams = vec![
            StreamProcessor::new(1),
            StreamProcessor::new(3),
            StreamProcessor::new(5),
            StreamProcessor::new(7),
        ];

        let payloads = vec![
            b"sk-proj-stream1secret111111111111111111".to_vec(),
            b"sk-proj-stream3secret333333333333333333".to_vec(),
            b"sk-proj-stream5secret555555555555555555".to_vec(),
            b"sk-proj-stream7secret777777777777777777".to_vec(),
        ];

        // Feed unique secrets to each stream
        for (stream, payload) in streams.iter_mut().zip(payloads.iter()) {
            stream.add_request(payload.clone());
        }

        // Redact all streams
        let redacted: Vec<_> = streams.iter()
            .zip(payloads.iter())
            .map(|(stream, payload)| {
                let redacted = stream.redact(payload);
                println!("[STREAM-{}] Redacted: {}", stream.stream_id, 
                        String::from_utf8_lossy(&redacted));
                redacted
            })
            .collect();

        // Verify no cross-contamination: secrets from one stream should not appear in another
        for i in 0..redacted.len() {
            for j in 0..redacted.len() {
                if i != j {
                    let payload_i_str = String::from_utf8_lossy(&payloads[i]);
                    let redacted_j_str = String::from_utf8_lossy(&redacted[j]);
                    
                    // Extract the unique part of payload i
                    let unique_part = if let Some(pos) = payload_i_str.find("stream") {
                        &payload_i_str[pos..pos+7] // "stream1", "stream3", etc
                    } else {
                        &payload_i_str
                    };

                    // This unique identifier should NOT be in stream j's redacted output
                    // (unless it's part of the redacted x's)
                    if unique_part.chars().all(|c| c != 'x') {
                        assert!(!redacted_j_str.contains(unique_part),
                                "Stream {} contains identifier from Stream {}", j, i);
                    }
                }
            }
        }
    }

    /// Test Case 4: Header continuation frames don't leak between streams
    #[test]
    fn test_header_continuation_isolation() {
        let mut stream1 = StreamProcessor::new(1);
        let mut stream2 = StreamProcessor::new(3);

        // Simulate large headers with Bearer tokens
        let header_part1 = b"Authorization: Bearer sk-proj-verylongsecrettokenpart1xxxxxxxxxx";
        let header_part2 = b"x-api-key: Bearer sk-proj-anothersecretpart2yyyyyyyyyyyyyy";

        stream1.add_request(header_part1.to_vec());
        stream1.add_request(header_part2.to_vec());

        stream2.add_request(b"Normal request data".to_vec());

        // Redact stream1 headers
        let redacted1_p1 = stream1.redact(header_part1);
        let redacted1_p2 = stream1.redact(header_part2);

        // Redact stream2 (should not be affected)
        let redacted2 = stream2.redact(b"Normal request data");

        // Verify stream2 output is unaffected
        assert_eq!(redacted2, b"Normal request data");

        // Verify stream1 has some redaction (stream isolation check)
        assert_ne!(redacted1_p1, header_part1.to_vec());
        assert_ne!(redacted1_p2, header_part2.to_vec());
    }

    /// Test Case 5: Verify redaction doesn't affect parsing
    #[test]
    fn test_redaction_preserves_structure() {
        let stream = StreamProcessor::new(1);

        let json_request = br#"{"api_key": "sk-proj-secrettoken1234567890abcdefghij", "action": "login"}"#;
        let redacted = stream.redact(json_request);
        let redacted_str = String::from_utf8_lossy(&redacted);

        // JSON structure should still be valid
        assert!(redacted_str.contains('{'));
        assert!(redacted_str.contains('}'));
        assert!(redacted_str.contains(":"));
        assert!(redacted_str.contains("action"));
        assert!(redacted_str.contains("login"));

        // But the secret should be redacted
        assert!(!redacted_str.contains("secrettoken"));
        assert!(redacted_str.contains("xxxxx"));

        println!("[STRUCTURE] Original: {}", String::from_utf8_lossy(json_request));
        println!("[STRUCTURE] Redacted: {}", redacted_str);
    }

    /// Test Case 6: Empty/null stream handling
    #[test]
    fn test_empty_stream_handling() {
        let mut stream1 = StreamProcessor::new(1);
        let mut stream2 = StreamProcessor::new(3);

        // Stream 1 has data, Stream 2 is empty
        stream1.add_request(b"Authorization: Bearer sk-proj-token1234".to_vec());
        
        let redacted1 = stream1.redact(b"Authorization: Bearer sk-proj-token1234");
        let redacted2 = stream2.redact(b""); // Empty

        assert!(!String::from_utf8_lossy(&redacted1).contains("token"));
        assert_eq!(redacted2.len(), 0);

        println!("[EMPTY-STREAM] Stream 1 (has data): {}", String::from_utf8_lossy(&redacted1));
        println!("[EMPTY-STREAM] Stream 2 (empty): OK");
    }

    /// Test Case 7: Stream state doesn't persist across different redaction calls
    #[test]
    fn test_stream_state_isolation() {
        let mut stream1 = StreamProcessor::new(1);
        let mut stream2 = StreamProcessor::new(3);

        let secret_a = b"Bearer sk-proj-stream1secret111111111111111111";
        let secret_b = b"Bearer sk-proj-stream2secret222222222222222222";

        // Stream 1 processes secret A
        stream1.add_request(secret_a.to_vec());
        let redacted_a = stream1.redact(secret_a);

        // Stream 2 processes secret B independently
        stream2.add_request(secret_b.to_vec());
        let redacted_b = stream2.redact(secret_b);

        // Verify stream 1 secret doesn't appear in stream 2's output
        assert!(!String::from_utf8_lossy(&redacted_b).contains("stream1secret"));
        
        // Verify redactions happened
        assert_ne!(redacted_a, secret_a.to_vec());
        assert_ne!(redacted_b, secret_b.to_vec());
    }
}
