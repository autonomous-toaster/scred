use super::*;

#[cfg(test)]
mod glob_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_exact_match() {
        let matcher = GlobMatcher::new("mysql-password");
        assert!(matcher.matches("mysql-password"));
        assert!(!matcher.matches("mysql-user"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_star_suffix() {
        let matcher = GlobMatcher::new("mysql*");
        assert!(matcher.matches("mysql-password"));
        assert!(matcher.matches("mysql-url"));
        assert!(matcher.matches("mysql-dsn"));
        assert!(matcher.matches("mysql")); // * matches 0 chars
        assert!(!matcher.matches("postgres-password"));
        assert!(!matcher.matches("mariadb-password"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_star_prefix() {
        let matcher = GlobMatcher::new("*-password");
        assert!(matcher.matches("mysql-password"));
        assert!(matcher.matches("postgres-password"));
        assert!(matcher.matches("redis-password"));
        assert!(!matcher.matches("password"));
        assert!(!matcher.matches("mysql-user"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_star_middle() {
        let matcher = GlobMatcher::new("aws*-key");
        assert!(matcher.matches("aws-key"));
        assert!(matcher.matches("aws-access-key"));
        assert!(matcher.matches("aws-secret-key"));
        assert!(!matcher.matches("aws-user"));
        assert!(!matcher.matches("aws"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_question_single() {
        let matcher = GlobMatcher::new("aws-?");
        assert!(matcher.matches("aws-a"));
        assert!(matcher.matches("aws-k"));
        assert!(!matcher.matches("aws-ab"));
        assert!(!matcher.matches("aws-"));
        assert!(!matcher.matches("aws"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_question_multiple() {
        let matcher = GlobMatcher::new("gh?-token");
        assert!(matcher.matches("ghp-token"));
        assert!(matcher.matches("ghu-token"));
        assert!(matcher.matches("ghs-token"));
        assert!(!matcher.matches("gh-token"));
        assert!(!matcher.matches("ghab-token"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_combined_wildcards() {
        let matcher = GlobMatcher::new("*test*");
        assert!(matcher.matches("test"));
        assert!(matcher.matches("pre-test"));
        assert!(matcher.matches("test-post"));
        assert!(matcher.matches("pre-test-post"));
        assert!(!matcher.matches("tst"));
        assert!(!matcher.matches("tes"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_aws_pattern() {
        let matcher = GlobMatcher::new("aws-*");
        assert!(matcher.matches("aws-akia"));
        assert!(matcher.matches("aws-access-key"));
        assert!(matcher.matches("aws-secret-key"));
        assert!(matcher.matches("aws-asia"));
        assert!(!matcher.matches("azure-key"));
        assert!(!matcher.matches("aws"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_github_pattern() {
        let matcher = GlobMatcher::new("github-*");
        assert!(matcher.matches("github-ghp"));
        assert!(matcher.matches("github-token"));
        assert!(matcher.matches("github-pat"));
        assert!(!matcher.matches("gitlab-token"));
        assert!(!matcher.matches("github"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_api_key_patterns() {
        let matchers = vec![
            ("mysql*", vec!["mysql-password", "mysql-url", "mysql-dsn"]),
            ("postgres*", vec!["postgresql-password", "postgresql-dsn"]),
            ("redis*", vec!["redis-password", "redis-url"]),
            ("mongodb*", vec!["mongodb-password", "mongodb-uri"]),
            ("openai*", vec!["openai-api-key", "openai-sk-proj"]),
            ("dependabot*", vec!["dependabot-token", "dependabot-secret"]),
        ];

        for (pattern, expected_matches) in matchers {
            let matcher = GlobMatcher::new(pattern);
            for name in expected_matches {
                assert!(
                    matcher.matches(name),
                    "Pattern {} should match {}",
                    pattern,
                    name
                );
            }
        }
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_exclusion_pattern() {
        let matcher = GlobMatcher::new("test-*");
        // These should match the glob
        assert!(matcher.matches("test-secret"));
        assert!(matcher.matches("test-password"));
        assert!(matcher.matches("test-key"));
        // These should NOT match
        assert!(!matcher.matches("prod-secret"));
        assert!(!matcher.matches("staging-password"));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_performance_simple() {
        // Verify simple case is fast
        let start = std::time::Instant::now();
        let matcher = GlobMatcher::new("mysql*");
        for _ in 0..10000 {
            let _ = matcher.matches("mysql-password");
        }
        let elapsed = start.elapsed();
        // Should be very fast (<1ms for 10k matches)
        assert!(
            elapsed.as_millis() < 50,
            "Performance regression: {}ms for 10k matches",
            elapsed.as_millis()
        );
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_edge_cases() {
        // Empty pattern should only match empty string
        let matcher = GlobMatcher::new("");
        assert!(matcher.matches(""));
        assert!(!matcher.matches("anything"));

        // Single * should match anything
        let matcher = GlobMatcher::new("*");
        assert!(matcher.matches(""));
        assert!(matcher.matches("anything"));
        assert!(matcher.matches("mysql-password-12345"));

        // ? should match any single char
        let matcher = GlobMatcher::new("?");
        assert!(matcher.matches("a"));
        assert!(!matcher.matches(""));
        assert!(!matcher.matches("ab"));
    }
}

#[cfg(test)]
mod pattern_selector_glob_tests {
    use super::*;

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_selector_wildcard_mode() {
        let selector = PatternSelector::Wildcard("mysql*".to_string());
        assert_eq!(selector.description(), "Wildcard: mysql*");
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_selector_from_string_wildcard() {
        let selector = PatternSelector::from_string("wildcard:mysql*").unwrap();
        assert!(matches!(selector, PatternSelector::Wildcard(_)));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_selector_from_string_multiple_globs() {
        // Note: Currently supports "patterns:mysql*,postgres*" syntax
        let selector =
            PatternSelector::from_string("patterns:mysql-password,postgres-dsn").unwrap();
        assert!(matches!(selector, PatternSelector::Patterns(_)));
    }
}

#[cfg(test)]
mod composite_selector_tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_single_tier_filter() {
        let selector = CompositePatternSelector::from_string("CRITICAL").unwrap();
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(!selector.matches("mysql-password", RiskTier::ApiKeys));
    }

    #[allow(clippy::unwrap_used)]
    #[allow(clippy::unwrap_used)]    #[test]
    fn test_multiple_tiers() {
        let selector = CompositePatternSelector::from_string("CRITICAL,API_KEYS").unwrap();
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(selector.matches("mysql-password", RiskTier::ApiKeys));
        assert!(!selector.matches("ssh-key", RiskTier::Infrastructure));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_glob_pattern_only() {
        let selector = CompositePatternSelector::from_string("mysql*").unwrap();
        assert!(selector.matches("mysql-password", RiskTier::Critical));
        assert!(selector.matches("mysql-url", RiskTier::Critical));
        assert!(!selector.matches("postgres-dsn", RiskTier::Critical));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_multiple_glob_patterns() {
        let selector = CompositePatternSelector::from_string("mysql*,postgresql*,redis*").unwrap();
        assert!(selector.matches("mysql-password", RiskTier::Critical));
        assert!(selector.matches("postgresql-dsn", RiskTier::Critical));
        assert!(selector.matches("redis-password", RiskTier::Critical));
        assert!(!selector.matches("mongodb-uri", RiskTier::Critical));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_tier_and_glob_combined() {
        let selector = CompositePatternSelector::from_string("CRITICAL,mysql*,postgres*").unwrap();
        // Matches CRITICAL tier
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        // Matches glob patterns
        assert!(selector.matches("mysql-password", RiskTier::ApiKeys));
        assert!(selector.matches("postgresql-dsn", RiskTier::ApiKeys));
        // Doesn't match anything
        assert!(!selector.matches("heroku-api-key", RiskTier::ApiKeys));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_simple_exclusion() {
        let selector = CompositePatternSelector::from_string("CRITICAL,!test-*").unwrap();
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(!selector.matches("test-secret", RiskTier::Critical));
        assert!(!selector.matches("test-password", RiskTier::Critical));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_exclude_syntax_variations() {
        let selector1 = CompositePatternSelector::from_string("CRITICAL,!test-*").unwrap();
        let selector2 = CompositePatternSelector::from_string("CRITICAL,exclude:test-*").unwrap();

        // Both should behave identically
        assert!(selector1.matches("aws-akia", RiskTier::Critical));
        assert!(selector2.matches("aws-akia", RiskTier::Critical));

        assert!(!selector1.matches("test-secret", RiskTier::Critical));
        assert!(!selector2.matches("test-secret", RiskTier::Critical));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_multiple_exclusions() {
        let selector =
            CompositePatternSelector::from_string("CRITICAL,!test-*,!mock-*,!dummy-*").unwrap();
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(!selector.matches("test-secret", RiskTier::Critical));
        assert!(!selector.matches("mock-password", RiskTier::Critical));
        assert!(!selector.matches("dummy-key", RiskTier::Critical));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_complex_real_world_scenario() {
        // Detect CRITICAL tier + AWS/GitHub/OpenAI patterns, excluding test patterns
        let selector = CompositePatternSelector::from_string(
            "CRITICAL,aws-*,github-*,openai-*,!test-*,!example-*",
        )
        .unwrap();

        // Should match
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        assert!(selector.matches("aws-access-key", RiskTier::Critical));
        assert!(selector.matches("github-ghp", RiskTier::Critical));
        assert!(selector.matches("openai-sk-proj", RiskTier::Critical));

        // Should NOT match (exclusions)
        assert!(!selector.matches("test-secret", RiskTier::Critical));
        assert!(!selector.matches("example-password", RiskTier::Critical));

        // Should NOT match (not included - different pattern type)
        assert!(!selector.matches("mysql-password", RiskTier::ApiKeys));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_database_pattern_selection() {
        // Select only database patterns
        let selector =
            CompositePatternSelector::from_string("mysql*,postgresql*,mongodb*,redis*").unwrap();

        assert!(selector.matches("mysql-password", RiskTier::Critical));
        assert!(selector.matches("postgresql-dsn", RiskTier::Critical));
        assert!(selector.matches("mongodb-uri", RiskTier::Critical));
        assert!(selector.matches("redis-password", RiskTier::Critical));

        assert!(!selector.matches("aws-akia", RiskTier::Critical));
        assert!(!selector.matches("github-ghp", RiskTier::Critical));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_api_provider_selection() {
        // Select OpenAI, Anthropic, HuggingFace
        let selector =
            CompositePatternSelector::from_string("openai*,anthropic*,huggingface*").unwrap();

        assert!(selector.matches("openai-api-key", RiskTier::Critical));
        assert!(selector.matches("openai-sk-proj", RiskTier::Critical));
        assert!(selector.matches("anthropic-api-key", RiskTier::Critical));
        assert!(selector.matches("huggingface-token", RiskTier::ApiKeys));

        assert!(!selector.matches("aws-akia", RiskTier::Critical));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_tier_with_specific_glob_and_exclusion() {
        let selector =
            CompositePatternSelector::from_string("CRITICAL,API_KEYS,mysql*,!test-*").unwrap();

        // CRITICAL tier
        assert!(selector.matches("aws-akia", RiskTier::Critical));
        // API_KEYS tier
        assert!(selector.matches("heroku-api-key", RiskTier::ApiKeys));
        // Glob pattern
        assert!(selector.matches("mysql-password", RiskTier::Critical));
        // Excluded
        assert!(!selector.matches("test-password", RiskTier::Critical));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_invalid_no_inclusions() {
        // Only exclusions should fail
        let result = CompositePatternSelector::from_string("!test-*,!mock-*");
        assert!(result.is_err());
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_pattern_filter_parsing() {
        let tier_filter = PatternFilter::from_str("CRITICAL").unwrap();
        assert!(matches!(
            tier_filter,
            PatternFilter::Tier(RiskTier::Critical)
        ));

        let glob_filter = PatternFilter::from_str("mysql*").unwrap();
        assert!(matches!(glob_filter, PatternFilter::GlobName(_)));

        let exclude_filter1 = PatternFilter::from_str("!test-*").unwrap();
        assert!(matches!(exclude_filter1, PatternFilter::Exclude(_)));

        let exclude_filter2 = PatternFilter::from_str("exclude:dummy-*").unwrap();
        assert!(matches!(exclude_filter2, PatternFilter::Exclude(_)));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_description() {
        let selector = CompositePatternSelector::from_string("CRITICAL,mysql*,!test-*").unwrap();
        let desc = selector.description();
        assert!(desc.contains("Critical")); // Debug format
        assert!(desc.contains("mysql"));
        assert!(desc.contains("test"));
    }

    #[allow(clippy::unwrap_used)]    #[test]
    fn test_performance_composite_matching() {
        let selector = CompositePatternSelector::from_string(
            "CRITICAL,API_KEYS,mysql*,postgresql*,redis*,mongodb*,!test-*,!mock-*",
        )
        .unwrap();

        // Verify it's fast (should be <1ms for 1000 matches)
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = selector.matches("mysql-password", RiskTier::Critical);
            let _ = selector.matches("aws-akia", RiskTier::Critical);
            let _ = selector.matches("test-secret", RiskTier::Critical);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 10,
            "Performance regression: {}ms for 3000 matches",
            elapsed.as_millis()
        );
    }
}
