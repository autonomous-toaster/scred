//! TDD Tests for Phase 4: Proxy Streaming Selector Integration
//!
//! Tests for integrating selector support into Proxy streaming.
//! Written BEFORE implementation (TDD approach).

#[cfg(test)]
mod phase4_proxy_streaming_selector_tests {
    use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor, StreamingConfig, PatternSelector, PatternTier};
    use std::sync::Arc;

    // =========================================================================
    // PROXY STREAMING WITH SELECTOR SUPPORT
    // =========================================================================

    #[test]
    fn test_proxy_streaming_redactor_without_selector() {
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();
        let redactor = StreamingRedactor::new(engine, config);

        // Default streaming redactor should not have selector
        assert!(!redactor.has_selector());
    }

    #[test]
    fn test_proxy_streaming_redactor_with_selector() {
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();
        let selector = PatternSelector::Tier(vec![PatternTier::Critical]);

        let redactor = StreamingRedactor::with_selector(engine, config, selector);

        assert!(redactor.has_selector());
    }

    #[test]
    fn test_proxy_streaming_redactor_critical_selector() {
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();
        let selector = PatternSelector::Tier(vec![PatternTier::Critical]);

        let redactor = StreamingRedactor::with_selector(engine, config, selector);

        // Should be able to retrieve selector
        match redactor.get_selector() {
            Some(PatternSelector::Tier(tiers)) => {
                assert_eq!(tiers.len(), 1);
                assert!(tiers.contains(&PatternTier::Critical));
            }
            _ => panic!("Expected Critical tier selector"),
        }
    }

    #[test]
    fn test_proxy_streaming_redactor_multiple_tiers() {
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();
        let selector = PatternSelector::Tier(vec![
            PatternTier::Critical,
            PatternTier::ApiKeys,
        ]);

        let redactor = StreamingRedactor::with_selector(engine, config, selector);

        match redactor.get_selector() {
            Some(PatternSelector::Tier(tiers)) => {
                assert_eq!(tiers.len(), 2);
                assert!(tiers.contains(&PatternTier::Critical));
                assert!(tiers.contains(&PatternTier::ApiKeys));
            }
            _ => panic!("Expected multiple tier selector"),
        }
    }

    #[test]
    fn test_proxy_streaming_redactor_all_selector() {
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();
        let selector = PatternSelector::All;

        let redactor = StreamingRedactor::with_selector(engine, config, selector);

        match redactor.get_selector() {
            Some(PatternSelector::All) => {}, // Success
            _ => panic!("Expected All selector"),
        }
    }

    #[test]
    fn test_proxy_streaming_redactor_none_selector() {
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();
        let selector = PatternSelector::None;

        let redactor = StreamingRedactor::with_selector(engine, config, selector);

        match redactor.get_selector() {
            Some(PatternSelector::None) => {}, // Success
            _ => panic!("Expected None selector"),
        }
    }

    #[test]
    fn test_proxy_streaming_redactor_selector_stored() {
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();
        let selector = PatternSelector::Tier(vec![PatternTier::ApiKeys]);

        let redactor = StreamingRedactor::with_selector(engine, config, selector.clone());

        // Verify selector is stored and accessible
        assert!(redactor.has_selector());
        let stored = redactor.get_selector().expect("Selector should exist");
        match stored {
            PatternSelector::Tier(tiers) => {
                assert_eq!(tiers.len(), 1);
                assert_eq!(tiers[0], PatternTier::ApiKeys);
            }
            _ => panic!("Expected Tier selector"),
        }
    }

    #[test]
    fn test_proxy_streaming_multiple_redactors_independent() {
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();

        let sel1 = PatternSelector::Tier(vec![PatternTier::Critical]);
        let sel2 = PatternSelector::Tier(vec![PatternTier::Infrastructure]);

        let redactor1 = StreamingRedactor::with_selector(engine.clone(), config.clone(), sel1);
        let redactor2 = StreamingRedactor::with_selector(engine.clone(), config, sel2);

        // Verify independent selectors
        match redactor1.get_selector() {
            Some(PatternSelector::Tier(tiers)) => {
                assert!(tiers.contains(&PatternTier::Critical));
            }
            _ => panic!("Redactor1 failed"),
        }

        match redactor2.get_selector() {
            Some(PatternSelector::Tier(tiers)) => {
                assert!(tiers.contains(&PatternTier::Infrastructure));
            }
            _ => panic!("Redactor2 failed"),
        }
    }

    // =========================================================================
    // BACKWARD COMPATIBILITY
    // =========================================================================

    #[test]
    fn test_proxy_streaming_new_unchanged() {
        // Existing StreamingRedactor::new() should still work
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();

        let _redactor = StreamingRedactor::new(engine, config);
        // Should compile and work as before
    }

    #[test]
    fn test_proxy_streaming_redact_buffer_works() {
        // Existing redact_buffer() should still work
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();
        let redactor = StreamingRedactor::new(engine, config);

        let data = b"test data";
        let (_output, _stats) = redactor.redact_buffer(data);
        // Should complete without error
    }

    #[test]
    fn test_proxy_streaming_default_selector_none() {
        // Default StreamingRedactor should have no selector
        let engine = Arc::new(RedactionEngine::new(Default::default()));
        let config = StreamingConfig::default();
        let redactor = StreamingRedactor::new(engine, config);

        assert!(!redactor.has_selector());
    }

    // =========================================================================
    // PROXY CONFIGURATION INTEGRATION
    // =========================================================================

    #[test]
    fn test_proxy_config_with_critical_selector() {
        // Simulate proxy configuration with CRITICAL selector
        let selector = PatternSelector::Tier(vec![PatternTier::Critical]);
        
        // Verify selector can be created from string (as proxy does)
        let parsed = PatternSelector::from_str("CRITICAL");
        assert!(parsed.is_ok(), "Should parse CRITICAL selector");
        
        let parsed_selector = parsed.unwrap();
        match parsed_selector {
            PatternSelector::Tier(tiers) => {
                assert_eq!(tiers.len(), 1);
                assert!(tiers.contains(&PatternTier::Critical));
            }
            _ => panic!("Expected Tier selector"),
        }
    }

    #[test]
    fn test_proxy_config_with_comma_separated_selectors() {
        // Simulate proxy configuration with multiple selectors
        let parsed = PatternSelector::from_str("CRITICAL,API_KEYS");
        assert!(parsed.is_ok(), "Should parse multiple selectors");
        
        let parsed_selector = parsed.unwrap();
        match parsed_selector {
            PatternSelector::Tier(tiers) => {
                assert_eq!(tiers.len(), 2);
                assert!(tiers.contains(&PatternTier::Critical));
                assert!(tiers.contains(&PatternTier::ApiKeys));
            }
            _ => panic!("Expected Tier selector"),
        }
    }

    // =========================================================================
    // METHOD SIGNATURES (documented for Phase 4)
    // =========================================================================

    #[test]
    #[ignore = "Implementation note: Phase 4 updates Proxy main.rs"]
    fn phase4_todo_proxy_streaming_integration() {
        // After Phase 4, Proxy main.rs will:
        // 1. Remove _config_engine (line 424-428)
        // 2. Create StreamingRedactor with selector:
        //    let redactor = Arc::new(
        //        StreamingRedactor::with_selector(
        //            redaction_engine.clone(),
        //            streaming_config,
        //            config.redact_selector.clone(),
        //        )
        //    );
        // 3. Use redactor for streaming redaction with selector support
    }

    #[test]
    #[ignore = "Implementation note: Phase 4 removes dead code"]
    fn phase4_todo_remove_config_engine() {
        // After Phase 4, the unused _config_engine will be removed from:
        // - crates/scred-proxy/src/main.rs line 424-428
        // - Related comments about broken selector support
    }
}
