/// Integration test for pattern selector filtering
/// Verifies that pattern selectors correctly filter detected secrets

use scred_http::{PatternSelector, PatternTier};

#[test]
fn test_pattern_selector_default_detect() {
    let selector = PatternSelector::default_detect();
    
    // Should include CRITICAL tier patterns
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.contains(&PatternTier::Critical)));
    
    // Should include API_KEYS tier patterns
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.contains(&PatternTier::ApiKeys)));
    
    // Should include INFRASTRUCTURE tier patterns
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.contains(&PatternTier::Infrastructure)));
}

#[test]
fn test_pattern_selector_default_redact() {
    let selector = PatternSelector::default_redact();
    
    // Should include only CRITICAL and API_KEYS tiers (conservative)
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.contains(&PatternTier::Critical)));
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.contains(&PatternTier::ApiKeys)));
    
    // Should NOT include SERVICES or PATTERNS
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if !tiers.contains(&PatternTier::Services)));
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if !tiers.contains(&PatternTier::Patterns)));
}

#[test]
fn test_pattern_selector_parse_critical_only() {
    let selector = PatternSelector::from_str("CRITICAL").expect("Should parse");
    
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.len() == 1));
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.contains(&PatternTier::Critical)));
}

#[test]
fn test_pattern_selector_parse_multiple_tiers() {
    let selector = PatternSelector::from_str("CRITICAL,API_KEYS,INFRASTRUCTURE")
        .expect("Should parse multiple tiers");
    
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.len() == 3));
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.contains(&PatternTier::Critical)));
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.contains(&PatternTier::ApiKeys)));
    assert!(matches!(selector, PatternSelector::Tier(ref tiers) if tiers.contains(&PatternTier::Infrastructure)));
}

#[test]
fn test_pattern_selector_description() {
    let selector = PatternSelector::default_detect();
    let desc = selector.description();
    
    // Should contain meaningful description
    assert!(!desc.is_empty());
    assert!(desc.contains("Tier"));
}

#[test]
fn test_pattern_tier_names() {
    assert_eq!(PatternTier::Critical.name(), "CRITICAL");
    assert_eq!(PatternTier::ApiKeys.name(), "API_KEYS");
    assert_eq!(PatternTier::Infrastructure.name(), "INFRASTRUCTURE");
    assert_eq!(PatternTier::Services.name(), "SERVICES");
    assert_eq!(PatternTier::Patterns.name(), "PATTERNS");
}

#[test]
fn test_pattern_tier_risk_scores() {
    // Critical should be highest risk
    assert!(PatternTier::Critical.risk_score() > PatternTier::ApiKeys.risk_score());
    assert!(PatternTier::ApiKeys.risk_score() > PatternTier::Infrastructure.risk_score());
    assert!(PatternTier::Infrastructure.risk_score() > PatternTier::Services.risk_score());
    assert!(PatternTier::Services.risk_score() > PatternTier::Patterns.risk_score());
    
    // Sanity check
    assert!(PatternTier::Critical.risk_score() >= 90);
    assert!(PatternTier::Patterns.risk_score() <= 40);
}
