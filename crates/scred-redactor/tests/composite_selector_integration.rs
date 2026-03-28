//! Phase 4: Integration Tests for CompositePatternSelector
//! Tests real-world pattern selection scenarios

use scred_redactor::{CompositePatternSelector, PatternTier};

#[test]
fn test_database_provider_selection() {
    // Real-world: Select only database-related patterns
    let selector = CompositePatternSelector::from_string(
        "mysql*,postgresql*,mongodb*,redis*,mariadb*,cassandra*,couchdb*",
    )
    .unwrap();

    // Should match various DB patterns
    assert!(selector.matches("mysql-password", PatternTier::ApiKeys));
    assert!(selector.matches("postgresql-dsn", PatternTier::ApiKeys));
    assert!(selector.matches("mongodb-uri", PatternTier::ApiKeys));
    assert!(selector.matches("redis-password", PatternTier::ApiKeys));
    assert!(selector.matches("mariadb-password", PatternTier::ApiKeys));
    assert!(selector.matches("cassandra-password", PatternTier::ApiKeys));
    assert!(selector.matches("couchdb-password", PatternTier::ApiKeys));

    // Should NOT match other providers
    assert!(!selector.matches("aws-akia", PatternTier::Critical));
    assert!(!selector.matches("github-pat", PatternTier::Critical));
}

#[test]
fn test_cloud_provider_selection() {
    // Real-world: Select all major cloud providers
    let selector =
        CompositePatternSelector::from_string("aws-*,gcp-*,azure-*,digitalocean-*,linode-*")
            .unwrap();

    assert!(selector.matches("aws-akia", PatternTier::Critical));
    assert!(selector.matches("aws-secret-access-key", PatternTier::Critical));
    assert!(selector.matches("gcp-service-account", PatternTier::Critical));
    assert!(selector.matches("azure-connection-string", PatternTier::Infrastructure));
    assert!(selector.matches("digitalocean-token", PatternTier::ApiKeys));
    assert!(selector.matches("linode-api-token", PatternTier::ApiKeys));

    assert!(!selector.matches("github-pat", PatternTier::Critical));
}

#[test]
fn test_ai_api_provider_selection() {
    // Real-world: Select all AI/ML API keys
    let selector = CompositePatternSelector::from_string(
        "openai*,anthropic*,huggingface*,cohere*,replicate*,together*",
    )
    .unwrap();

    assert!(selector.matches("openai-api-key", PatternTier::ApiKeys));
    assert!(selector.matches("openai-sk-proj", PatternTier::ApiKeys));
    assert!(selector.matches("anthropic-api-key", PatternTier::ApiKeys));
    assert!(selector.matches("huggingface-token", PatternTier::ApiKeys));
    assert!(selector.matches("cohere-api-key", PatternTier::ApiKeys));
    assert!(selector.matches("replicate-token", PatternTier::ApiKeys));
    assert!(selector.matches("together-api-key", PatternTier::ApiKeys));

    assert!(!selector.matches("stripe-api-key", PatternTier::Critical));
}

#[test]
fn test_payment_processor_selection() {
    // Real-world: Select payment processor secrets
    let selector = CompositePatternSelector::from_string(
        "stripe*,paypal*,square*,braintree*,twilio*,sendgrid*",
    )
    .unwrap();

    assert!(selector.matches("stripe-api-key", PatternTier::Critical));
    assert!(selector.matches("stripe-sk-live", PatternTier::Critical));
    assert!(selector.matches("paypal-client-id", PatternTier::Critical));
    assert!(selector.matches("square-access-token", PatternTier::ApiKeys));
    assert!(selector.matches("braintree-private-key", PatternTier::Critical));
    assert!(selector.matches("twilio-auth-token", PatternTier::ApiKeys));
    assert!(selector.matches("sendgrid-api-key", PatternTier::ApiKeys));

    assert!(!selector.matches("github-pat", PatternTier::Critical));
}

#[test]
fn test_critical_plus_database_selection() {
    // Real-world: CRITICAL tier + specific DB patterns
    let selector = CompositePatternSelector::from_string("CRITICAL,mysql*,postgresql*").unwrap();

    // CRITICAL tier matches
    assert!(selector.matches("aws-akia", PatternTier::Critical));
    assert!(selector.matches("github-pat", PatternTier::Critical));
    assert!(selector.matches("stripe-sk-live", PatternTier::Critical));

    // DB patterns match regardless of tier
    assert!(selector.matches("mysql-password", PatternTier::ApiKeys));
    assert!(selector.matches("postgresql-dsn", PatternTier::Patterns));

    // Other patterns don't match
    assert!(!selector.matches("openai-api-key", PatternTier::ApiKeys));
}

#[test]
fn test_exclude_test_patterns() {
    // Real-world: Select everything EXCEPT test/mock/dummy patterns
    let selector =
        CompositePatternSelector::from_string("CRITICAL,API_KEYS,!test-*,!mock-*,!dummy-*")
            .unwrap();

    assert!(selector.matches("aws-akia", PatternTier::Critical));
    assert!(selector.matches("github-pat", PatternTier::Critical));
    assert!(selector.matches("openai-api-key", PatternTier::ApiKeys));

    assert!(!selector.matches("test-secret", PatternTier::Critical));
    assert!(!selector.matches("mock-password", PatternTier::Critical));
    assert!(!selector.matches("dummy-key", PatternTier::Critical));
    assert!(!selector.matches("test-api-token", PatternTier::ApiKeys));
}

#[test]
fn test_include_specific_exclude_pattern_family() {
    // Real-world: Include specific patterns but exclude one family
    let selector = CompositePatternSelector::from_string(
        "mysql*,postgresql*,mongodb*,redis*,!redis-dev*",
    )
    .unwrap();

    assert!(selector.matches("mysql-password", PatternTier::ApiKeys));
    assert!(selector.matches("postgresql-dsn", PatternTier::ApiKeys));
    assert!(selector.matches("mongodb-uri", PatternTier::ApiKeys));
    assert!(selector.matches("redis-password", PatternTier::ApiKeys));

    // Excluded
    assert!(!selector.matches("redis-dev-password", PatternTier::ApiKeys));
}

#[test]
fn test_complex_production_scenario() {
    // Real-world: Production log filtering for security team
    // Include: CRITICAL + API_KEYS + major cloud/DB providers
    // Exclude: test, mock, dummy, example patterns
    let selector = CompositePatternSelector::from_string(
        "CRITICAL,API_KEYS,aws-*,gcp-*,github-*,mysql*,postgresql*,mongodb*,!test-*,!mock-*,!example-*",
    ).unwrap();

    // Should match - high-value patterns
    assert!(selector.matches("aws-akia", PatternTier::Critical));
    assert!(selector.matches("aws-secret-access-key", PatternTier::Critical));
    assert!(selector.matches("github-pat", PatternTier::Critical));
    assert!(selector.matches("gcp-service-account", PatternTier::Critical));
    assert!(selector.matches("heroku-api-key", PatternTier::ApiKeys));
    assert!(selector.matches("mysql-password", PatternTier::ApiKeys));
    assert!(selector.matches("postgresql-dsn", PatternTier::ApiKeys));
    assert!(selector.matches("mongodb-uri", PatternTier::ApiKeys));

    // Should NOT match - excluded or not included
    assert!(!selector.matches("test-secret", PatternTier::Critical));
    assert!(!selector.matches("mock-password", PatternTier::Critical));
    assert!(!selector.matches("example-token", PatternTier::Critical));
    // Other patterns not in selection (not cloud/db and not in CRITICAL/API_KEYS)
    assert!(!selector.matches("datadog-api-key", PatternTier::Services));
}

#[test]
fn test_microservices_architecture_scenario() {
    // Real-world: Microservices with multiple databases and services
    // Database tier: mysql, postgres, mongo, redis
    // Service tier: stripe, sendgrid, twilio, datadog
    let db_selector = CompositePatternSelector::from_string("mysql*,postgresql*,mongodb*,redis*")
        .unwrap();
    let service_selector =
        CompositePatternSelector::from_string("stripe*,sendgrid*,twilio*,datadog*").unwrap();

    // Database patterns
    assert!(db_selector.matches("mysql-password", PatternTier::ApiKeys));
    assert!(!service_selector.matches("mysql-password", PatternTier::ApiKeys));

    // Service patterns
    assert!(service_selector.matches("stripe-api-key", PatternTier::Critical));
    assert!(!db_selector.matches("stripe-api-key", PatternTier::Critical));
}

#[test]
fn test_tiered_deployment_scenario() {
    // Real-world: Different pattern selection per environment
    // Dev: All 5 tiers
    // Staging: CRITICAL + API_KEYS + common databases
    // Prod: CRITICAL only + exclude test patterns

    let dev_selector = CompositePatternSelector::from_string(
        "CRITICAL,API_KEYS,INFRASTRUCTURE,SERVICES,PATTERNS",
    )
    .unwrap();
    let staging_selector =
        CompositePatternSelector::from_string("CRITICAL,API_KEYS,mysql*,postgresql*,mongodb*")
            .unwrap();
    let prod_selector =
        CompositePatternSelector::from_string("CRITICAL,!test-*,!dummy-*,!mock-*").unwrap();

    let test_patterns = vec![
        ("aws-akia", PatternTier::Critical),
        ("github-pat", PatternTier::Critical),
        ("openai-api-key", PatternTier::ApiKeys),
        ("mysql-password", PatternTier::ApiKeys),
        ("mytest-secret", PatternTier::Infrastructure),
    ];

    for (pattern_name, tier) in test_patterns {
        let should_dev = true; // Dev matches all tiers

        let should_staging = match pattern_name {
            "aws-akia" => true,           // CRITICAL
            "github-pat" => true,         // CRITICAL
            "openai-api-key" => true,     // API_KEYS
            "mysql-password" => true,     // mysql*
            "mytest-secret" => false,     // Not CRITICAL, API_KEYS, mysql*, postgresql*, mongodb*
            _ => false,
        };

        let should_prod = match pattern_name {
            "aws-akia" => true,       // CRITICAL
            "github-pat" => true,     // CRITICAL
            "openai-api-key" => false, // Not CRITICAL
            "mysql-password" => false, // Not CRITICAL
            "mytest-secret" => false,  // Not CRITICAL
            _ => false,
        };

        assert_eq!(
            dev_selector.matches(pattern_name, tier),
            should_dev,
            "Dev selector mismatch for {}",
            pattern_name
        );
        assert_eq!(
            staging_selector.matches(pattern_name, tier),
            should_staging,
            "Staging selector mismatch for {}",
            pattern_name
        );
        assert_eq!(
            prod_selector.matches(pattern_name, tier),
            should_prod,
            "Prod selector mismatch for {}",
            pattern_name
        );
    }
}

#[test]
fn test_security_team_audit_scenario() {
    // Real-world: Security team wants to audit all CRITICAL patterns
    // but exclude internal/test patterns
    let selector = CompositePatternSelector::from_string(
        "CRITICAL,!internal-*,!test-*,!sandbox-*,!dev-*",
    )
    .unwrap();

    assert!(selector.matches("aws-akia", PatternTier::Critical));
    assert!(selector.matches("github-pat", PatternTier::Critical));
    assert!(selector.matches("stripe-sk-live", PatternTier::Critical));

    assert!(!selector.matches("internal-key", PatternTier::Critical));
    assert!(!selector.matches("test-token", PatternTier::Critical));
    assert!(!selector.matches("sandbox-secret", PatternTier::Critical));
    assert!(!selector.matches("dev-password", PatternTier::Critical));
}

#[test]
fn test_performance_with_complex_selectors() {
    // Verify complex selectors remain performant
    let selector = CompositePatternSelector::from_string(
        "CRITICAL,API_KEYS,aws-*,gcp-*,azure-*,mysql*,postgresql*,mongodb*,redis*,stripe*,github-*,\
         openai*,anthropic*,huggingface*,!test-*,!mock-*,!dummy-*,!example-*,!sandbox-*",
    )
    .unwrap();

    // Benchmark: 10,000 matches should be < 100ms
    let start = std::time::Instant::now();
    for _ in 0..10000 {
        let _ = selector.matches("aws-akia", PatternTier::Critical);
        let _ = selector.matches("mysql-password", PatternTier::ApiKeys);
        let _ = selector.matches("test-secret", PatternTier::Critical);
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 100,
        "Complex selector performance issue: {}ms for 30k matches",
        elapsed.as_millis()
    );
}

#[test]
fn test_all_wildcard_patterns() {
    // Verify all major pattern wildcards work
    let patterns = vec![
        ("aws-*", "aws-akia", true),
        ("aws-*", "aws-secret-access-key", true),
        ("aws-*", "gcp-key", false),
        ("github-*", "github-pat", true),
        ("github-*", "github-oauth", true),
        ("github-*", "gitlab-token", false),
        ("*-password", "mysql-password", true),
        ("*-password", "postgres-password", true),
        ("*-password", "postgres-user", false),
        ("stripe-*", "stripe-api-key", true),
        ("stripe-*", "stripe-webhook", true),
        ("stripe-*", "square-token", false),
    ];

    for (glob, pattern_name, should_match) in patterns {
        let selector = CompositePatternSelector::from_string(glob).unwrap();
        assert_eq!(
            selector.matches(pattern_name, PatternTier::Critical),
            should_match,
            "Pattern {} against glob {} should be {}",
            pattern_name,
            glob,
            should_match
        );
    }
}
