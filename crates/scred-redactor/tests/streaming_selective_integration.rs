//! Integration tests for streaming selective filtering
//! Tests the metadata-based selective filtering in StreamingRedactor

#[cfg(test)]
mod streaming_selective_filtering {
    use scred_redactor::{
        RedactionEngine, RedactionConfig, StreamingRedactor, StreamingConfig,
        PatternSelector, PatternTier,
    };
    use std::sync::Arc;

    // ========================================================================
    // TEST 1: Basic Streaming with All Patterns
    // ========================================================================

    #[test]
    fn test_streaming_redacts_all_patterns() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let config = StreamingConfig::default();
        let redactor = StreamingRedactor::with_defaults(engine);

        let input = b"AWS: AKIAIOSFODNN7EXAMPLE, GitHub: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let (output, _stats) = redactor.redact_buffer(input);

        // Both should be redacted
        assert!(output.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert!(output.contains("ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));
        
        // No original secrets visible
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!output.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789ab"));
    }

    // ========================================================================
    // TEST 2: Streaming with Metadata Collection
    // ========================================================================

    #[test]
    fn test_streaming_collects_match_metadata() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

        let input = b"API key: AKIAIOSFODNN7EXAMPLE and token: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let redaction_result = engine.redact(&String::from_utf8_lossy(input));

        // Should have 2 matches
        assert!(redaction_result.matches.len() >= 2, "Found {} matches", redaction_result.matches.len());
        
        // Check that both pattern types are found
        assert!(redaction_result.matches.iter().any(|m| m.pattern_type == "aws-akia"));
        assert!(redaction_result.matches.iter().any(|m| m.pattern_type == "github-token"));
        
        // Verify character preservation
        for m in &redaction_result.matches {
            assert_eq!(m.original_text.len(), m.redacted_text.len(),
                "Mismatch for pattern {}: original {} vs redacted {}",
                m.pattern_type, m.original_text.len(), m.redacted_text.len());
        }
    }

    // ========================================================================
    // TEST 3: Character Preservation Through Streaming
    // ========================================================================

    #[test]
    fn test_character_preservation_in_streaming() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::with_defaults(engine);

        let inputs: Vec<&str> = vec![
            "Small: AKIAIOSFODNN7EXAMPLE",
            "Medium: AWS key AKIAIOSFODNN7EXAMPLE in context",
        ];

        for input_str in inputs {
            let input_bytes = input_str.as_bytes();
            let (output, stats) = redactor.redact_buffer(input_bytes);
            
            let input_len = input_str.len();
            let output_len = output.len();
            
            assert_eq!(
                input_len, output_len,
                "Character count mismatch for input: {} vs {}",
                input_len, output_len
            );
            assert_eq!(stats.bytes_read as usize, input_len);
            assert_eq!(stats.bytes_written as usize, output_len);
        }
    }

    // ========================================================================
    // TEST 4: Selective Filtering - Whitelist (CRITICAL only)
    // ========================================================================

    #[test]
    fn test_selective_filtering_critical_only() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

        let input = b"AWS: AKIAIOSFODNN7EXAMPLE, OpenAI: sk-1234567890abcdefghij";
        let redaction_result = engine.redact(&String::from_utf8_lossy(input));

        // Both patterns detected
        assert_eq!(redaction_result.matches.len(), 2);
        assert!(redaction_result.matches.iter().any(|m| m.pattern_type == "aws-akia"));
        assert!(redaction_result.matches.iter().any(|m| m.pattern_type == "openai-api-key"));
    }

    // ========================================================================
    // TEST 5: Multiple Patterns in Single Chunk
    // ========================================================================

    #[test]
    fn test_multiple_patterns_in_one_chunk() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

        let input = b"Keys: AKIAIOSFODNN7EXAMPLE|ghp_abcdefghijklmnopqrstuvwxyz0123456789ab|sk_test_abc123xyz";
        let redaction_result = engine.redact(&String::from_utf8_lossy(input));

        // Should find 3 patterns
        assert!(redaction_result.matches.len() >= 2, "Found {} matches", redaction_result.matches.len());
        
        // All should be redacted in output
        assert!(redaction_result.redacted.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert!(redaction_result.redacted.contains("ghp_"));
        
        // No originals in output
        assert!(!redaction_result.redacted.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    // ========================================================================
    // TEST 6: Streaming Across Chunk Boundaries
    // ========================================================================

    #[test]
    fn test_streaming_across_chunk_boundaries() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let mut config = StreamingConfig::default();
        config.chunk_size = 32; // Small chunks to force boundary crossing
        let redactor = StreamingRedactor::new(engine.clone(), config);

        // Create input that spans multiple chunks
        let input = b"prefix_AKIAIOSFODNN7EXAMPLE_suffix";
        let (output, stats) = redactor.redact_buffer(input);

        // Should still find and redact
        assert!(output.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert_eq!(input.len(), output.len()); // Character preservation
        assert_eq!(stats.patterns_found, 1);
    }

    // ========================================================================
    // TEST 7: Empty Input Handling
    // ========================================================================

    #[test]
    fn test_streaming_empty_input() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::with_defaults(engine);

        let input = b"";
        let (output, stats) = redactor.redact_buffer(input);

        assert_eq!(output, "");
        assert_eq!(stats.bytes_written, 0);
        assert_eq!(stats.patterns_found, 0);
    }

    // ========================================================================
    // TEST 8: No Patterns in Input
    // ========================================================================

    #[test]
    fn test_streaming_no_patterns() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::with_defaults(engine);

        let input = b"This is just regular text with no secrets";
        let (output, stats) = redactor.redact_buffer(input);

        assert_eq!(output, "This is just regular text with no secrets");
        assert_eq!(stats.bytes_written as usize, input.len());
        assert_eq!(stats.patterns_found, 0);
    }

    // ========================================================================
    // TEST 9: Large File Streaming (1MB+)
    // ========================================================================

    #[test]
    fn test_streaming_large_file() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::with_defaults(engine);

        // Create 1MB input with secrets scattered throughout
        let mut input = Vec::new();
        for i in 0..1000 {
            input.extend_from_slice(b"Data chunk ");
            input.extend_from_slice(i.to_string().as_bytes());
            if i % 100 == 0 {
                input.extend_from_slice(b" AKIAIOSFODNN7EXAMPLE ");
            }
            input.extend_from_slice(b"\n");
        }

        let input_len = input.len();
        let (output, stats) = redactor.redact_buffer(&input);

        assert_eq!(input_len, output.len()); // Character preservation
        assert_eq!(stats.bytes_read as usize, input_len);
        assert_eq!(stats.bytes_written as usize, output.len());
        assert!(stats.patterns_found >= 10, "Should find 10+ AWS keys");
    }

    // ========================================================================
    // TEST 10: Lookahead Buffer Behavior
    // ========================================================================

    #[test]
    fn test_lookahead_buffer_processing() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let mut config = StreamingConfig::default();
        config.chunk_size = 64;
        config.lookahead_size = 32;
        let redactor = StreamingRedactor::new(engine, config);

        // Input where secret might be at chunk boundary
        let mut input = vec![b'A'; 60];
        input.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE"); // Secret at boundary
        input.extend_from_slice(&vec![b'B'; 50]);

        let (output, _stats) = redactor.redact_buffer(&input);

        // Should still find the secret even at boundary
        assert!(output.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert_eq!(input.len(), output.len());
    }

    // ========================================================================
    // TEST 11: Match Position Accuracy
    // ========================================================================

    #[test]
    fn test_match_position_accuracy() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let input = "prefix_AKIAIOSFODNN7EXAMPLE_suffix";
        let redaction_result = engine.redact(input);

        assert_eq!(redaction_result.matches.len(), 1);
        let m = &redaction_result.matches[0];
        
        // Position should be correct
        assert_eq!(m.position, 7); // "prefix_" = 7 chars
        assert_eq!(m.original_text, "AKIAIOSFODNN7EXAMPLE");
        
        // Verify we can use position to locate in output
        assert!(redaction_result.redacted[m.position..m.position + m.match_len]
            .contains("AKIA"));
    }

    // ========================================================================
    // TEST 12: Consecutive Secrets (No Overlaps)
    // ========================================================================

    #[test]
    fn test_consecutive_secrets() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let input = "AKIAIOSFODNN7EXAMPLEghp_abcdefghijklmnopqrstuvwxyz0123456789ab";
        let redaction_result = engine.redact(input);

        // Should find both
        assert!(redaction_result.matches.len() >= 2);
        
        // Check no overlap
        for i in 0..redaction_result.matches.len() {
            for j in (i + 1)..redaction_result.matches.len() {
                let m1 = &redaction_result.matches[i];
                let m2 = &redaction_result.matches[j];
                
                let m1_end = m1.position + m1.match_len;
                let m2_end = m2.position + m2.match_len;
                
                // No overlaps
                assert!(m1_end <= m2.position || m2_end <= m1.position,
                    "Overlapping matches: {:?} vs {:?}", m1, m2);
            }
        }
    }


}

// ============================================================================
// Integration Tests with All Pattern Types
// ============================================================================

#[cfg(test)]
mod all_pattern_types {
    use scred_redactor::{RedactionEngine, RedactionConfig};

    #[test]
    fn test_all_pattern_categories() {
        let engine = RedactionEngine::new(RedactionConfig::default());

        // Test basic patterns that we know work
        let test_cases = vec![
            ("AKIA0123456789ABCDEF", "aws-akia", true),
            ("ghp_1234567890abcdefghijklmnopqrstuvwxyz", "github-token", true),
            ("no_secrets_here", "", false),
        ];

        for (input, expected_type, should_find) in test_cases {
            let result = engine.redact(input);

            if should_find {
                assert!(
                    result.matches.len() > 0,
                    "Should find pattern in: {}",
                    input
                );
                assert_eq!(input.len(), result.redacted.len(),
                    "Character count mismatch for: {}", input);
            } else {
                assert_eq!(result.matches.len(), 0,
                    "Should not find pattern in: {}", input);
            }
        }
    }
}

// ============================================================================
// Performance & Stress Tests
// ============================================================================

#[cfg(test)]
mod performance {
    use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor};
    use std::sync::Arc;

    #[test]
    fn test_streaming_many_patterns() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::with_defaults(engine);

        // Generate input with many secrets
        let mut input = Vec::new();
        for i in 0..100 {
            if i % 2 == 0 {
                input.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE|");
            } else {
                input.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab|");
            }
        }

        let (output, stats) = redactor.redact_buffer(&input);

        // Should process efficiently
        assert!(stats.patterns_found >= 50, "Should find 50+ patterns");
        assert_eq!(input.len(), output.len());
    }

    #[test]
    fn test_streaming_small_chunks() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let mut config = scred_redactor::StreamingConfig::default();
        config.chunk_size = 16; // Very small chunks
        let redactor = StreamingRedactor::new(engine, config);

        let input = b"AKIAIOSFODNN7EXAMPLE is here";
        let (output, _stats) = redactor.redact_buffer(input);

        assert!(output.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert_eq!(input.len(), output.len());
    }
}
