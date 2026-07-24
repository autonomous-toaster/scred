#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

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
}
