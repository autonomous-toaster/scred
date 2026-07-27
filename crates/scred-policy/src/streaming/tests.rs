#![allow(clippy::unwrap_used)]
use super::*;
use std::collections::HashMap;
use crate::placeholder::PlaceholderGenerator;

fn test_generator() -> PlaceholderGenerator {
        PlaceholderGenerator::new("test-seed")
    }

    #[test]
    fn test_automaton_basic() {
        let mut secrets = HashMap::new();
        secrets.insert("API_KEY".to_string(), "sk-secret-123".to_string());

        let mut generator = test_generator();
        let automaton = PlaceholderAutomaton::build(&secrets, &mut generator).unwrap();

        let placeholder = generator.generate("API_KEY", "sk-secret-123");
        let mut buffer = format!("Authorization: Bearer {}", placeholder.value)
            .as_bytes()
            .to_vec();

        let (tracker, count) =
            automaton.replace_placeholders(&mut buffer, "api.example.com", |_, _| true);

        assert_eq!(count, 1);
        let result = String::from_utf8_lossy(&buffer);
        assert!(result.contains("sk-secret-123"));
        assert!(!result.contains(&placeholder.value));

        // Response path
        let response = format!("Key: {}", "sk-secret-123");
        let mut resp_buf = response.as_bytes().to_vec();
        let count = automaton.replace_secrets(&mut resp_buf, &tracker);

        assert_eq!(count, 1);
        let result = String::from_utf8_lossy(&resp_buf);
        assert!(result.contains(&placeholder.value));
        assert!(!result.contains("sk-secret-123"));
    }

    #[test]
    fn test_streaming_chunks() {
        let mut secrets = HashMap::new();
        secrets.insert("KEY".to_string(), "secret-value".to_string());

        let mut generator = test_generator();
        let automaton = PlaceholderAutomaton::build(&secrets, &mut generator).unwrap();

        let placeholder = generator.generate("KEY", "secret-value");
        let placeholder_value = placeholder.value.clone();

        // Split placeholder across chunks
        let full_text = format!("data: {}", placeholder_value);
        let split_point = full_text.len() / 2;

        let chunk1 = full_text[..split_point].as_bytes();
        let chunk2 = full_text[split_point..].as_bytes();

        let mut lookahead = Vec::new();

        // Process chunk 1
        let (output1, _) = automaton.process_chunk_request(chunk1, &mut lookahead, false);

        // Process chunk 2
        let (output2, _) = automaton.process_chunk_request(chunk2, &mut lookahead, true);

        let combined = [output1, output2].concat();
        let result = String::from_utf8_lossy(&combined);
        assert!(result.contains("secret-value"));
    }

    #[test]
    fn test_replace_placeholders_empty_values() {
        let automaton = PlaceholderAutomaton {
            ac: AhoCorasick::new(&[""]).unwrap(),
            placeholder_values: vec![],
            replacements: vec![],
        };
        let mut buffer = b"hello".to_vec();
        let (tracker, count) = automaton.replace_placeholders(&mut buffer, "example.com", |_, _| true);
        assert_eq!(count, 0);
        assert!(tracker.replacements().is_empty());
    }

    #[test]
    fn test_replace_placeholders_non_utf8() {
        let mut secrets = HashMap::new();
        secrets.insert("KEY".to_string(), "secret".to_string());
        let mut generator = test_generator();
        let automaton = PlaceholderAutomaton::build(&secrets, &mut generator).unwrap();
        
        // Non-UTF8 data should return 0 replacements
        let mut buffer = vec![0xFF, 0xFE, 0x00];
        let (tracker, count) = automaton.replace_placeholders(&mut buffer, "example.com", |_, _| true);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_replace_placeholders_no_matches() {
        let mut secrets = HashMap::new();
        secrets.insert("KEY".to_string(), "secret".to_string());
        let mut generator = test_generator();
        let automaton = PlaceholderAutomaton::build(&secrets, &mut generator).unwrap();
        
        let mut buffer = b"no placeholders here".to_vec();
        let (tracker, count) = automaton.replace_placeholders(&mut buffer, "example.com", |_, _| true);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_replace_placeholders_domain_restricted() {
        let mut secrets = HashMap::new();
        secrets.insert("KEY".to_string(), "secret".to_string());
        let mut generator = test_generator();
        let automaton = PlaceholderAutomaton::build(&secrets, &mut generator).unwrap();
        
        let placeholder = generator.generate("KEY", "secret");
        let mut buffer = format!("value: {}", placeholder.value).as_bytes().to_vec();
        
        // Domain checker rejects all
        let (tracker, count) = automaton.replace_placeholders(&mut buffer, "other.com", |_, _| false);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_multiple_placeholders() {
        // Use realistic-length secrets (placeholder format: prefix + "scrd-" + hex)
        // sk-scrd-XXXXXXXX needs at least 11 chars to have hex variation
        let mut secrets = HashMap::new();
        secrets.insert("KEY_A".to_string(), "sk-apikey_a1234567890".to_string());
        secrets.insert("KEY_B".to_string(), "sk-apikey_b1234567890".to_string());

        let mut generator = test_generator();
        let automaton = PlaceholderAutomaton::build(&secrets, &mut generator).unwrap();

        let p_a = generator.generate("KEY_A", "sk-apikey_a1234567890").value.clone();
        let p_b = generator.generate("KEY_B", "sk-apikey_b1234567890").value.clone();

        // Verify placeholders contain marker
        assert!(p_a.contains("scrd-"));
        assert!(p_b.contains("scrd-"));

        let mut buffer = format!("{} and {}", p_a, p_b).as_bytes().to_vec();
        let (tracker, count) = automaton.replace_placeholders(&mut buffer, "api.example.com", |_, _| true);
        assert_eq!(count, 2);
        let result = String::from_utf8_lossy(&buffer);
        // Should contain real secrets after replacement
        assert!(result.contains("sk-apikey_a1234567890"));
        assert!(result.contains("sk-apikey_b1234567890"));

        // Response path - redact secrets back to placeholders
        let response = "Got: sk-apikey_a1234567890 and sk-apikey_b1234567890";
        let mut resp_buf = response.as_bytes().to_vec();
        let count = automaton.replace_secrets(&mut resp_buf, &tracker);
        assert_eq!(count, 2);
        let result = String::from_utf8_lossy(&resp_buf);
        assert!(result.contains(&p_a));
        assert!(result.contains(&p_b));
    }