// Test PatternType selector parsing and filtering

#[test]
fn test_pattern_type_selector_fast() {
    use scred_http::PatternSelector;

    let selector = PatternSelector::from_str("fast").unwrap();
    match selector {
        PatternSelector::Type(types) => {
            assert_eq!(types.len(), 1);
            assert_eq!(types[0], "fast");
        }
        _ => panic!("Expected Type variant"),
    }
}

#[test]
fn test_pattern_type_selector_structured() {
    use scred_http::PatternSelector;

    let selector = PatternSelector::from_str("structured").unwrap();
    match selector {
        PatternSelector::Type(types) => {
            assert_eq!(types.len(), 1);
            assert_eq!(types[0], "structured");
        }
        _ => panic!("Expected Type variant"),
    }
}

#[test]
fn test_pattern_type_selector_regex() {
    use scred_http::PatternSelector;

    let selector = PatternSelector::from_str("regex").unwrap();
    match selector {
        PatternSelector::Type(types) => {
            assert_eq!(types.len(), 1);
            assert_eq!(types[0], "regex");
        }
        _ => panic!("Expected Type variant"),
    }
}

#[test]
fn test_pattern_type_selector_combined() {
    use scred_http::PatternSelector;

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
fn test_pattern_type_selector_with_prefix() {
    use scred_http::PatternSelector;

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
fn test_pattern_type_selector_all() {
    use scred_http::PatternSelector;

    let selector = PatternSelector::from_str("all").unwrap();
    match selector {
        PatternSelector::All => {},
        _ => panic!("Expected All variant"),
    }
}

#[test]
fn test_pattern_type_selector_backward_compat_tier() {
    use scred_http::{PatternSelector, PatternTier};

    let selector = PatternSelector::from_str("CRITICAL,API_KEYS").unwrap();
    match selector {
        PatternSelector::Tiers(tiers) => {
            assert_eq!(tiers.len(), 2);
            assert!(tiers.iter().any(|t| matches!(t, PatternTier::Critical)));
            assert!(tiers.iter().any(|t| matches!(t, PatternTier::ApiKeys)));
        }
        _ => panic!("Expected Tiers variant for backward compatibility"),
    }
}
