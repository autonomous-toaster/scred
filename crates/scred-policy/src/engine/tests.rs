use super::*;

mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*_API_KEY", "OPENAI_API_KEY"));
        assert!(glob_match("APP_*", "APP_SECRET"));
        assert!(!glob_match("APP_*", "OTHER_SECRET"));
    }

    #[test]
    #[test]
    fn test_process_headers_passthrough() {
        let config = PolicyConfig {
            enabled: false,
            providers: vec![],
            ..Default::default()
        };
        let engine = PolicyEngine::new(config).unwrap();

        let mut headers = http::HeaderMap::new();
        headers.insert("Content-Type", http::HeaderValue::from_static("application/json"));

        // Default policy should process headers
        let result = engine.process_headers(&mut headers, "example.com").unwrap();
        assert!(result.headers_processed > 0);
    }

    #[test]
    fn test_process_body_passthrough() {
        let config = PolicyConfig {
            enabled: false,
            providers: vec![],
            ..Default::default()
        };
        let engine = PolicyEngine::new(config).unwrap();

        let mut body = b"{\"message\": \"hello\"}".to_vec();
        let result = engine.process_body(&mut body, "example.com", Direction::Request).unwrap();
        assert!(result.bytes_processed > 0);
    }

    #[test]
    fn test_resolve_policy_default() {
        let config = PolicyConfig {
            enabled: false,
            providers: vec![],
            ..Default::default()
        };
        let engine = PolicyEngine::new(config).unwrap();

        // Should resolve to default for unknown host
        let resolved = engine.resolve_for_host("unknown.example.com");
        assert!(matches!(resolved.source, ConfigSource::Default));
    }

    #[test]
    fn test_header_action_resolution() {
        use scred_config::{HeaderRules, HostPolicy};

        // Create config with specific header rules
        let defaults = HostPolicy {
            merge: scred_config::MergeStrategy::Merge,
            headers: HeaderRules::default(),
            body: scred_config::BodyRules::default(),
            patterns: PatternFilter::default(),
        };

        let config = PolicyConfig {
            enabled: false,
            providers: vec![],
            defaults,
            ..Default::default()
        };

        let engine = PolicyEngine::new(config).unwrap();

        // Check Authorization gets Replace action by default
        let resolved = engine.resolve_for_host("example.com");
        assert_eq!(resolved.header_action("Authorization"), HeaderAction::Replace);
        assert_eq!(resolved.header_action("Content-Type"), HeaderAction::Redact);
    }

    #[test]
    fn test_value_collision_detection() {
        std::env::set_var("SCRED_UNIFIED_TEST_KEY_A", "same-collision-value");
        std::env::set_var("SCRED_UNIFIED_TEST_KEY_B", "same-collision-value");

        let config = PolicyConfig {
            enabled: true,
            providers: vec![scred_config::ProviderConfig::Env {
                keys: vec![
                    "SCRED_UNIFIED_TEST_KEY_A".to_string(),
                    "SCRED_UNIFIED_TEST_KEY_B".to_string(),
                ],
            }],
            ..Default::default()
        };

        let result = PolicyEngine::new(config);
        assert!(result.is_err(), "Expected error for value collision");

        if let Err(e) = result {
            let err_str = e.to_string();
            assert!(
                err_str.contains("SCRED_UNIFIED_TEST_KEY_A") || err_str.contains("same-collision-value"),
                "Error should mention the colliding keys: {}",
                err_str
            );
        }

        std::env::remove_var("SCRED_UNIFIED_TEST_KEY_A");
        std::env::remove_var("SCRED_UNIFIED_TEST_KEY_B");
    }
}
