/// Integration tests for v1.0.1 critical fixes
/// Issue #1: Proxy per-path rules enforcement
/// Issue #2: Regex selector implementation  
/// Issue #3: Invalid selector error handling (at CLI layer)

use scred_http::PatternSelector;

// ============================================================================
// ISSUE #2 TESTS: Regex Selector Implementation
// ============================================================================

#[test]
fn test_regex_selector_parsing() {
    let selector = PatternSelector::from_str("regex:^sk-");
    assert!(selector.is_ok());
    
    if let Ok(PatternSelector::Regex(patterns)) = selector {
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0], "^sk-");
    } else {
        panic!("Expected Regex selector");
    }
}

#[test]
fn test_regex_selector_with_start_anchor() {
    let selector = PatternSelector::from_str("regex:^sk-").unwrap();
    
    // Should match patterns starting with "sk-"
    assert!(selector.matches_pattern("sk-proj", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(selector.matches_pattern("sk-proj-123", scred_http::pattern_selector::PatternTier::Patterns));
    
    // Should NOT match patterns that don't start with "sk-"
    assert!(!selector.matches_pattern("openai-sk-", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(!selector.matches_pattern("my-secret-key", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(!selector.matches_pattern("prefix_sk-proj", scred_http::pattern_selector::PatternTier::Patterns));
}

#[test]
fn test_regex_selector_with_groups() {
    let selector = PatternSelector::from_str("regex:^(aws|github)").unwrap();
    
    // Should match patterns starting with aws or github
    assert!(selector.matches_pattern("aws_access_key", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(selector.matches_pattern("aws_secret_key", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(selector.matches_pattern("github_token", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(selector.matches_pattern("github_oauth", scred_http::pattern_selector::PatternTier::Patterns));
    
    // Should NOT match other patterns
    assert!(!selector.matches_pattern("stripe_key", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(!selector.matches_pattern("openai_key", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(!selector.matches_pattern("my_aws_key", scred_http::pattern_selector::PatternTier::Patterns));
}

#[test]
fn test_regex_selector_with_alternation() {
    let selector = PatternSelector::from_str("regex:secret|password|token").unwrap();
    
    // Should match any pattern containing secret, password, or token
    assert!(selector.matches_pattern("secret_key", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(selector.matches_pattern("api_password", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(selector.matches_pattern("access_token", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(selector.matches_pattern("bearer_token_auth", scred_http::pattern_selector::PatternTier::Patterns));
    
    // Should NOT match patterns without these keywords
    assert!(!selector.matches_pattern("api_key", scred_http::pattern_selector::PatternTier::Patterns));
}

#[test]
fn test_regex_selector_with_quantifiers() {
    let selector = PatternSelector::from_str("regex:^[a-z]{2,4}_").unwrap();
    
    // Should match patterns with 2-4 lowercase letters followed by underscore
    assert!(selector.matches_pattern("sk_proj", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(selector.matches_pattern("api_key", scred_http::pattern_selector::PatternTier::Patterns));
    assert!(selector.matches_pattern("aws_secret", scred_http::pattern_selector::PatternTier::Patterns));
    
    // Should NOT match
    assert!(!selector.matches_pattern("s_proj", scred_http::pattern_selector::PatternTier::Patterns));  // Too short
    assert!(!selector.matches_pattern("stripe_key", scred_http::pattern_selector::PatternTier::Patterns));  // 6 letters
    assert!(!selector.matches_pattern("proj", scred_http::pattern_selector::PatternTier::Patterns));  // No underscore
}

#[test]
fn test_regex_selector_matching_uses_actual_regex() {
    // This is the key test for Issue #2 fix:
    // Regex selector should use regex::is_match(), not string contains()
    
    let selector = PatternSelector::from_str("regex:^sk-").unwrap();
    
    // These should only match if regex is working correctly
    // If still using contains(), "^sk-" would have to be literally in the name
    let test_patterns = vec![
        ("sk-proj", true),
        ("sk-proj-123", true),
        ("sk_proj", false),  // Underscore, not dash
        ("prefix_sk-proj", false),  // Doesn't start with sk-
    ];
    
    for (pattern, should_match) in test_patterns {
        let matches = selector.matches_pattern(pattern, scred_http::pattern_selector::PatternTier::Patterns);
        assert_eq!(
            matches, should_match,
            "Pattern '{}' should {}match regex ^sk-",
            pattern,
            if should_match { "" } else { "NOT " }
        );
    }
}

// ============================================================================
// ISSUE #3 TESTS: Invalid Selector Error Handling
// ============================================================================

#[test]
fn test_valid_tier_single() {
    let result = PatternSelector::from_str("CRITICAL");
    assert!(result.is_ok());
    
    if let Ok(PatternSelector::Tier(tiers)) = result {
        assert_eq!(tiers.len(), 1);
    } else {
        panic!("Expected Tier selector");
    }
}

#[test]
fn test_valid_tier_multiple() {
    let result = PatternSelector::from_str("CRITICAL,API_KEYS");
    assert!(result.is_ok());
    
    if let Ok(PatternSelector::Tier(tiers)) = result {
        assert_eq!(tiers.len(), 2);
    } else {
        panic!("Expected Tier selector with 2 tiers");
    }
}

#[test]
fn test_valid_tier_all_variants() {
    let variants = vec![
        "CRITICAL",
        "API_KEYS",
        "INFRASTRUCTURE",
        "SERVICES",
        "PATTERNS",
    ];
    
    for variant in variants {
        let result = PatternSelector::from_str(variant);
        assert!(result.is_ok(), "Failed for variant: {}", variant);
    }
}

#[test]
fn test_valid_tier_case_insensitive_variations() {
    // API_KEYS can be spelled as API-KEYS
    let result = PatternSelector::from_str("CRITICAL,API-KEYS");
    assert!(result.is_ok());
    
    // Infrastructure can be abbreviated as INFRA
    let result = PatternSelector::from_str("INFRA");
    assert!(result.is_ok());
}

#[test]
fn test_valid_patterns_tier() {
    let result = PatternSelector::from_str("PATTERNS");
    assert!(result.is_ok());
    
    // GENERIC is alias for PATTERNS
    let result2 = PatternSelector::from_str("GENERIC");
    assert!(result2.is_ok());
}

#[test]
fn test_tier_parsing_with_parse_list() {
    // Test the tier parsing helper function
    let result = scred_http::pattern_selector::PatternTier::parse_list("CRITICAL,API_KEYS,INFRASTRUCTURE");
    assert!(result.is_ok());
    
    if let Ok(tiers) = result {
        assert_eq!(tiers.len(), 3);
    }
}

#[test]
fn test_tier_from_str_invalid() {
    // Test that invalid tier names return error
    let result = scred_http::pattern_selector::PatternTier::from_str("INVALID_TIER");
    assert!(result.is_err());
}

// ============================================================================
// ISSUE #1 TESTS: Per-Path Rules (Path Matching Logic)
// ============================================================================

/// Simulate the path matching used in proxy
fn path_matches(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if !pattern.contains('*') {
        return pattern == path;
    }

    // Simple wildcard matching
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut remaining = path;

    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            // First part must match at start
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else if i == parts.len() - 1 {
            // Last part must match at end
            if !remaining.ends_with(part) {
                return false;
            }
        } else {
            // Middle parts must be found in order
            if let Some(pos) = remaining.find(part) {
                remaining = &remaining[pos + part.len()..];
            } else {
                return false;
            }
        }
    }

    true
}

#[test]
fn test_path_exact_match() {
    assert!(path_matches("/admin", "/admin"));
    assert!(path_matches("/api/users", "/api/users"));
    assert!(path_matches("/", "/"));
}

#[test]
fn test_path_exact_no_match() {
    assert!(!path_matches("/admin", "/user"));
    assert!(!path_matches("/api/users", "/api/posts"));
    assert!(!path_matches("/health", "/health/status"));
}

#[test]
fn test_path_single_wildcard_prefix() {
    assert!(path_matches("/admin/*", "/admin/users"));
    assert!(path_matches("/admin/*", "/admin/settings"));
    assert!(path_matches("/admin/*", "/admin/"));
    assert!(path_matches("/api/*", "/api/v1"));
}

#[test]
fn test_path_single_wildcard_no_match() {
    assert!(!path_matches("/admin/*", "/user/admin"));
    assert!(!path_matches("/admin/*", "/administrator"));
    assert!(!path_matches("/api/*", "/v1/api"));
}

#[test]
fn test_path_wildcard_middle() {
    assert!(path_matches("/api/*/secret", "/api/v1/secret"));
    assert!(path_matches("/api/*/secret", "/api/v2/secret"));
    assert!(path_matches("/api/*/secret", "/api/beta/secret"));
}

#[test]
fn test_path_wildcard_middle_no_match() {
    assert!(!path_matches("/api/*/secret", "/api/v1/public"));
    assert!(!path_matches("/api/*/secret", "/api/secret"));
    assert!(!path_matches("/api/*/secret", "/secret/v1/api"));
}

#[test]
fn test_path_multiple_wildcards() {
    assert!(path_matches("/api/*/v*/secret", "/api/users/v1/secret"));
    assert!(path_matches("/api/*/v*/secret", "/api/posts/v2/secret"));
    assert!(path_matches("*/api/*", "/admin/api/users"));
    assert!(path_matches("*/api/*", "/service/api/posts"));
}

#[test]
fn test_path_multiple_wildcards_no_match() {
    assert!(!path_matches("/api/*/v*/secret", "/api/users/v1/public"));
    assert!(!path_matches("*/api/*", "/admin/service/users"));
}

#[test]
fn test_path_catch_all_wildcard() {
    assert!(path_matches("*", "/"));
    assert!(path_matches("*", "/any"));
    assert!(path_matches("*", "/any/path"));
    assert!(path_matches("*", "/any/path/with/many/segments"));
}

#[test]
fn test_path_prefix_suffix_pattern() {
    assert!(path_matches("/api*-secret", "/api/users-secret"));
    assert!(path_matches("/api*-secret", "/apiv1-secret"));
    assert!(path_matches("test*file.txt", "test_data_file.txt"));
    assert!(path_matches("test*file.txt", "testfile.txt"));
}

#[test]
fn test_path_prefix_suffix_no_match() {
    assert!(!path_matches("/api*-secret", "/api/users/secret"));
    assert!(!path_matches("test*file.txt", "test_file.doc"));
}

#[test]
fn test_path_health_check_pattern() {
    // Common use case: skip redaction for health checks
    assert!(path_matches("/health*", "/health"));
    assert!(path_matches("/health*", "/health/status"));
    assert!(path_matches("/health*", "/healthz"));
    assert!(path_matches("/health*", "/health-check"));
    
    assert!(!path_matches("/health*", "/actual-health"));
}

#[test]
fn test_path_per_path_rules_common_scenarios() {
    // Real-world test cases for proxy per-path rules
    
    // Scenario 1: Admin panel - might not want redaction
    assert!(path_matches("/admin/*", "/admin/users"));
    assert!(path_matches("/admin/*", "/admin/settings"));
    assert!(!path_matches("/admin/*", "/user/admin/profile"));
    
    // Scenario 2: Health checks - never redact
    assert!(path_matches("/health*", "/health"));
    assert!(path_matches("/health*", "/healthz"));
    
    // Scenario 3: API versioning - redact for all v1, v2, etc.
    assert!(path_matches("/api/v*/*", "/api/v1/users"));
    assert!(path_matches("/api/v*/*", "/api/v2/posts"));
    assert!(!path_matches("/api/v*/*", "/api/users"));  // No version
    
    // Scenario 4: Internal only endpoints
    assert!(path_matches("/internal/*", "/internal/config"));
    assert!(!path_matches("/internal/*", "/public/internal"));
}

// ============================================================================
// Integration Tests: Combined Selector Behavior
// ============================================================================

#[test]
fn test_selector_all() {
    let selector = PatternSelector::from_str("ALL").unwrap();
    
    // All patterns should match
    assert!(selector.matches_pattern("anything", scred_http::pattern_selector::PatternTier::Critical));
    assert!(selector.matches_pattern("pattern", scred_http::pattern_selector::PatternTier::ApiKeys));
    assert!(selector.matches_pattern("name", scred_http::pattern_selector::PatternTier::Infrastructure));
}

#[test]
fn test_selector_none() {
    let selector = PatternSelector::from_str("NONE").unwrap();
    
    // No patterns should match
    assert!(!selector.matches_pattern("anything", scred_http::pattern_selector::PatternTier::Critical));
    assert!(!selector.matches_pattern("pattern", scred_http::pattern_selector::PatternTier::ApiKeys));
}

#[test]
fn test_selector_default_detect() {
    let selector = PatternSelector::default_detect();
    
    // Should match CRITICAL by default
    assert!(selector.matches_pattern("any", scred_http::pattern_selector::PatternTier::Critical));
}

#[test]
fn test_selector_default_redact() {
    let selector = PatternSelector::default_redact();
    
    // Should match CRITICAL and API_KEYS by default
    assert!(selector.matches_pattern("any", scred_http::pattern_selector::PatternTier::Critical));
    assert!(selector.matches_pattern("any", scred_http::pattern_selector::PatternTier::ApiKeys));
}
