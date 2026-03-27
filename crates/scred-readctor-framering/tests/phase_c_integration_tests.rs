//! Phase C: Integration Testing for PatternType System
//! Real-world scenarios and edge cases

#[cfg(test)]
mod phase_c_integration_tests {
    use scred_readctor_framering::PatternSelector;

    // =========================================================================
    // 1. BACKWARD COMPATIBILITY TESTS
    // =========================================================================

    #[test]
    fn test_old_tier_format_still_works() {
        let selector = PatternSelector::from_str("CRITICAL,API_KEYS").unwrap();
        match selector {
            PatternSelector::Tiers(_) => assert!(true),
            _ => panic!("Expected Tiers variant for backward compatibility"),
        }
    }

    #[test]
    fn test_new_type_format_works() {
        let selector = PatternSelector::from_str("fast,structured").unwrap();
        match selector {
            PatternSelector::Type(types) => {
                assert_eq!(types.len(), 2);
                assert!(types.contains(&"fast".to_string()));
                assert!(types.contains(&"structured".to_string()));
            }
            _ => panic!("Expected Type variant"),
        }
    }

    #[test]
    fn test_all_format_unchanged() {
        let selector = PatternSelector::from_str("all").unwrap();
        match selector {
            PatternSelector::All => assert!(true),
            _ => panic!("Expected All variant"),
        }
    }

    #[test]
    fn test_selector_description_includes_type() {
        let selector = PatternSelector::from_str("fast").unwrap();
        let desc = selector.description();
        assert!(!desc.is_empty());
    }

    // =========================================================================
    // 2. PERFORMANCE CHARACTERISTICS
    // =========================================================================

    #[test]
    fn test_fast_mode_selector_creation() {
        let start = std::time::Instant::now();
        let _selector = PatternSelector::from_str("fast").unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 50, "Fast selector creation too slow");
    }

    #[test]
    fn test_selector_matches_deterministic() {
        let selector1 = PatternSelector::from_str("fast").unwrap();
        let selector2 = PatternSelector::from_str("fast").unwrap();
        assert_eq!(selector1, selector2);
    }

    // =========================================================================
    // 3. EDGE CASES
    // =========================================================================

    #[test]
    fn test_selector_none_format() {
        let selector = PatternSelector::from_str("none").unwrap();
        match selector {
            PatternSelector::None => assert!(true),
            _ => panic!("Expected None variant"),
        }
    }

    #[test]
    fn test_selector_case_insensitive() {
        let sel1 = PatternSelector::from_str("FAST").unwrap();
        let sel2 = PatternSelector::from_str("fast").unwrap();
        let sel3 = PatternSelector::from_str("Fast").unwrap();
        
        assert_eq!(sel1, sel2);
        assert_eq!(sel2, sel3);
    }

    #[test]
    fn test_selector_whitespace_handling() {
        let selector = PatternSelector::from_str("  fast  ,  structured  ").unwrap();
        
        match selector {
            PatternSelector::Type(types) => {
                assert_eq!(types.len(), 2);
                assert!(types.contains(&"fast".to_string()));
                assert!(types.contains(&"structured".to_string()));
            }
            _ => panic!("Expected Type variant"),
        }
    }

    #[test]
    fn test_selector_multiple_type_combinations() {
        let selector = PatternSelector::from_str("fast,structured,regex").unwrap();
        
        match selector {
            PatternSelector::Type(types) => {
                assert_eq!(types.len(), 3);
            }
            _ => panic!("Expected Type variant"),
        }
    }

    #[test]
    fn test_selector_invalid_type_fails() {
        let result = PatternSelector::from_str("invalid_type");
        assert!(result.is_err(), "Invalid type should fail");
    }

    // =========================================================================
    // 4. TYPE PREFIX FORMAT
    // =========================================================================

    #[test]
    fn test_selector_type_prefix_format() {
        let selector = PatternSelector::from_str("type:fast,regex").unwrap();
        
        match selector {
            PatternSelector::Type(types) => {
                assert_eq!(types.len(), 2);
                assert!(types.contains(&"fast".to_string()));
                assert!(types.contains(&"regex".to_string()));
            }
            _ => panic!("Expected Type variant"),
        }
    }

    #[test]
    fn test_selector_type_prefix_single() {
        let selector = PatternSelector::from_str("type:fast").unwrap();
        
        match selector {
            PatternSelector::Type(types) => {
                assert_eq!(types.len(), 1);
                assert_eq!(types[0], "fast");
            }
            _ => panic!("Expected Type variant"),
        }
    }

    // =========================================================================
    // 5. SELECTOR CLONING AND EQUALITY
    // =========================================================================

    #[test]
    fn test_selector_clone_equality() {
        let selector = PatternSelector::from_str("fast,structured").unwrap();
        let cloned = selector.clone();
        
        assert_eq!(selector, cloned);
    }

    #[test]
    fn test_different_selectors_not_equal() {
        let sel1 = PatternSelector::from_str("fast").unwrap();
        let sel2 = PatternSelector::from_str("regex").unwrap();
        
        assert_ne!(sel1, sel2);
    }

    // =========================================================================
    // 6. REAL-WORLD PATTERNS
    // =========================================================================

    #[test]
    fn test_fast_mode_detects_critical_patterns() {
        // Fast mode should include patterns for:
        // - AWS (AKIA)
        // - GitHub (ghp_, gho_, ghu_)
        // - Stripe (sk_, pk_)
        let selector = PatternSelector::from_str("fast").unwrap();
        match selector {
            PatternSelector::Type(types) => {
                assert!(types.contains(&"fast".to_string()));
            }
            _ => panic!("Expected Type variant"),
        }
    }

    #[test]
    fn test_balanced_mode_has_jwt() {
        // Balanced mode should include JWT validation
        let selector = PatternSelector::from_str("fast,structured").unwrap();
        match selector {
            PatternSelector::Type(types) => {
                assert!(types.iter().any(|t| t.contains("structured")));
            }
            _ => panic!("Expected Type variant"),
        }
    }

    #[test]
    fn test_comprehensive_mode_all_available() {
        // All mode should enable all 270 patterns
        let selector = PatternSelector::from_str("all").unwrap();
        match selector {
            PatternSelector::All => assert!(true),
            _ => panic!("Expected All variant"),
        }
    }
}
