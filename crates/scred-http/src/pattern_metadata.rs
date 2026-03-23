/// Pattern Tier Metadata Helper
/// Maps pattern names to their tiers for filtering detection and redaction
///
/// This is a temporary solution until we export full metadata from Zig.
/// Eventually this will be auto-generated from patterns.zig

use crate::PatternTier;
use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Map of pattern names to their tiers
/// Updated as we categorize more patterns in patterns.zig
static PATTERN_TIER_MAP: Lazy<HashMap<&'static str, PatternTier>> = Lazy::new(|| {
    let mut map = HashMap::new();
    
    // CRITICAL Tier (24 patterns)
    // AWS credentials
    map.insert("aws-akia", PatternTier::Critical);
    map.insert("aws-access-token", PatternTier::Critical);
    map.insert("aws-secret-access-key", PatternTier::Critical);
    map.insert("aws-session-token", PatternTier::Critical);
    map.insert("aws-mfa-serial", PatternTier::Critical);
    
    // GitHub tokens
    map.insert("github-token", PatternTier::Critical);
    map.insert("github-pat", PatternTier::Critical);
    map.insert("github-oauth", PatternTier::Critical);
    map.insert("github-user", PatternTier::Critical);
    map.insert("github-server", PatternTier::Critical);
    map.insert("github-refresh", PatternTier::Critical);
    
    // Stripe API keys (live keys = critical)
    map.insert("stripe-api-key", PatternTier::Critical);
    map.insert("stripe-restricted-key", PatternTier::Critical);
    map.insert("stripe-payment-intent", PatternTier::Critical);
    map.insert("stripepaymentintent-2", PatternTier::Critical);
    
    // Shopify
    map.insert("shopify-app-password", PatternTier::Critical);
    
    // OpenAI admin
    map.insert("openaiadmin", PatternTier::Critical);
    map.insert("sk-admin-", PatternTier::Critical);
    
    // Context7 (critical secrets)
    map.insert("context7-api-key", PatternTier::Critical);
    map.insert("context7-secret", PatternTier::Critical);
    
    // Database connections
    map.insert("mongodb", PatternTier::Critical);
    map.insert("postgres", PatternTier::Critical);
    
    // API_KEYS Tier (60+ patterns)
    // OpenAI
    map.insert("openai-api-key", PatternTier::ApiKeys);
    map.insert("openai", PatternTier::ApiKeys);
    
    // Anthropic
    map.insert("anthropic", PatternTier::ApiKeys);
    
    // Google
    map.insert("google-gemini", PatternTier::ApiKeys);
    map.insert("google-cloud-api-key", PatternTier::ApiKeys);
    
    // Communication APIs
    map.insert("slack-token", PatternTier::ApiKeys);
    map.insert("twilio-api-key", PatternTier::ApiKeys);
    map.insert("sendgrid-api-key", PatternTier::ApiKeys);
    map.insert("mailchimp-api-key", PatternTier::ApiKeys);
    map.insert("mailgun-api-key", PatternTier::ApiKeys);
    map.insert("telegram-bot-token", PatternTier::ApiKeys);
    map.insert("discord-webhook", PatternTier::ApiKeys);
    
    // Cloud providers
    map.insert("azure-api-key", PatternTier::ApiKeys);
    map.insert("azure-ad-client-secret", PatternTier::ApiKeys);
    
    // Monitoring/observability
    map.insert("datadog-api-key", PatternTier::ApiKeys);
    map.insert("sentry-access-token", PatternTier::ApiKeys);
    map.insert("sentryorgtoken", PatternTier::ApiKeys);
    map.insert("pagerduty-api-key", PatternTier::ApiKeys);
    map.insert("new-relic-api-key", PatternTier::ApiKeys);
    
    // Other APIs
    map.insert("heroku-api-key", PatternTier::ApiKeys);
    map.insert("npm-token", PatternTier::ApiKeys);
    map.insert("npmtokenv2", PatternTier::ApiKeys);
    map.insert("notion-api-key", PatternTier::ApiKeys);
    map.insert("huggingface-token", PatternTier::ApiKeys);
    map.insert("hubspot-api-key", PatternTier::ApiKeys);
    map.insert("mapbox-token", PatternTier::ApiKeys);
    map.insert("gitlab-token", PatternTier::ApiKeys);
    map.insert("postman-api-key", PatternTier::ApiKeys);
    map.insert("snyk-api-token", PatternTier::ApiKeys);
    map.insert("twitch-oauth-token", PatternTier::ApiKeys);
    map.insert("rubygems", PatternTier::ApiKeys);
    map.insert("linear-api-key", PatternTier::ApiKeys);
    map.insert("linearapi", PatternTier::ApiKeys);
    map.insert("vercel-token", PatternTier::ApiKeys);
    map.insert("travisoauth", PatternTier::ApiKeys);
    map.insert("supabase-api-key", PatternTier::ApiKeys);
    map.insert("atlas ian", PatternTier::ApiKeys);
    map.insert("contentful-personal-access-token", PatternTier::ApiKeys);
    map.insert("circleci-personal-access-token", PatternTier::ApiKeys);
    map.insert("gitee-access-token", PatternTier::ApiKeys);
    map.insert("gandi-api-key", PatternTier::ApiKeys);
    map.insert("apideck", PatternTier::ApiKeys);
    map.insert("okta-api-token", PatternTier::ApiKeys);
    
    // INFRASTRUCTURE Tier (40+ patterns)
    // Kubernetes & container orchestration
    map.insert("k8s-bearer-token", PatternTier::Infrastructure);
    map.insert("k8s-service-account-token", PatternTier::Infrastructure);
    map.insert("kubelet-token", PatternTier::Infrastructure);
    
    // Container registries
    map.insert("docker-registry-token", PatternTier::Infrastructure);
    map.insert("docker-login-token", PatternTier::Infrastructure);
    map.insert("ecr-registry-token", PatternTier::Infrastructure);
    map.insert("ghcr-token", PatternTier::Infrastructure);
    
    // Secrets management
    map.insert("vault-token", PatternTier::Infrastructure);
    map.insert("vault-unseal-key", PatternTier::Infrastructure);
    map.insert("consul-token", PatternTier::Infrastructure);
    map.insert("etcd-password", PatternTier::Infrastructure);
    map.insert("minio-access-key", PatternTier::Infrastructure);
    
    // Infrastructure APIs
    map.insert("databricks-token", PatternTier::Infrastructure);
    map.insert("dynatrace-api-token", PatternTier::Infrastructure);
    map.insert("grafana-api-key", PatternTier::Infrastructure);
    map.insert("grafana", PatternTier::Infrastructure);
    map.insert("prometheus-bearer-token", PatternTier::Infrastructure);
    map.insert("splunk-bearer-token", PatternTier::Infrastructure);
    map.insert("okta", PatternTier::Infrastructure);
    map.insert("1password-svc-token", PatternTier::Infrastructure);
    map.insert("artifactory-api-key", PatternTier::Infrastructure);
    map.insert("artifactory-reference-token", PatternTier::Infrastructure);
    map.insert("artifactoryreferencetoken", PatternTier::Infrastructure);
    map.insert("planetscale-1", PatternTier::Infrastructure);
    map.insert("planetscaledb-1", PatternTier::Infrastructure);
    map.insert("planetscale-password", PatternTier::Infrastructure);
    map.insert("upstash-redis", PatternTier::Infrastructure);
    map.insert("salad-cloud-api-key", PatternTier::Infrastructure);
    map.insert("azure-storage", PatternTier::Infrastructure);
    map.insert("azure-app-config", PatternTier::Infrastructure);
    map.insert("azure-batch", PatternTier::Infrastructure);
    map.insert("azure-cosmosdb", PatternTier::Infrastructure);
    
    // SERVICES Tier (100+ patterns)
    // Payment processors
    map.insert("razorpay-api-key", PatternTier::Services);
    map.insert("square-api-key", PatternTier::Services);
    map.insert("braintree-api-key", PatternTier::Services);
    map.insert("paystack", PatternTier::Services);
    map.insert("pagarme", PatternTier::Services);
    map.insert("ramp", PatternTier::Services);
    map.insert("flutterwave", PatternTier::Services);
    map.insert("flutterwave-public-key", PatternTier::Services);
    
    // Communication services
    map.insert("coinbase", PatternTier::Services);
    map.insert("checkr-personal-access-token", PatternTier::Services);
    map.insert("digicert-api-key", PatternTier::Services);
    map.insert("duffel-api-token", PatternTier::Services);
    map.insert("easypost-api-token", PatternTier::Services);
    map.insert("expo-access-token", PatternTier::Services);
    map.insert("figma-token", PatternTier::Services);
    map.insert("fleetbase", PatternTier::Services);
    map.insert("getdns", PatternTier::Services);
    map.insert("assertible", PatternTier::Services);
    map.insert("pagertree-api-token", PatternTier::Services);
    map.insert("pypi-upload-token", PatternTier::Services);
    map.insert("tumblr-api-key", PatternTier::Services);
    map.insert("generic-api-key", PatternTier::Services);
    
    // PATTERNS Tier (50+ patterns - regex-based)
    // Generic patterns
    map.insert("jwt-generic", PatternTier::Patterns);
    map.insert("jwt", PatternTier::Patterns);
    map.insert("bearer-token", PatternTier::Patterns);
    map.insert("basic-auth", PatternTier::Patterns);
    map.insert("authorization-header", PatternTier::Patterns);
    map.insert("api-key-header", PatternTier::Patterns);
    map.insert("private-key", PatternTier::Patterns);
    map.insert("privatekey", PatternTier::Patterns);
    
    map
});

/// Get the tier for a pattern name
pub fn get_pattern_tier(pattern_name: &str) -> PatternTier {
    // Try exact match first
    if let Some(tier) = PATTERN_TIER_MAP.get(pattern_name) {
        return *tier;
    }
    
    // Try case-insensitive match
    let lower = pattern_name.to_lowercase();
    for (key, tier) in PATTERN_TIER_MAP.iter() {
        if key.to_lowercase() == lower {
            return *tier;
        }
    }
    
    // Default to PATTERNS (generic/unknown patterns)
    // This is conservative - unknown patterns are treated as low-risk
    PatternTier::Patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critical_patterns() {
        assert_eq!(get_pattern_tier("aws-akia"), PatternTier::Critical);
        assert_eq!(get_pattern_tier("github-token"), PatternTier::Critical);
        assert_eq!(get_pattern_tier("stripe-api-key"), PatternTier::Critical);
    }

    #[test]
    fn test_api_key_patterns() {
        assert_eq!(get_pattern_tier("openai-api-key"), PatternTier::ApiKeys);
        assert_eq!(get_pattern_tier("slack-token"), PatternTier::ApiKeys);
        assert_eq!(get_pattern_tier("sendgrid-api-key"), PatternTier::ApiKeys);
    }

    #[test]
    fn test_infrastructure_patterns() {
        assert_eq!(get_pattern_tier("k8s-bearer-token"), PatternTier::Infrastructure);
        assert_eq!(get_pattern_tier("vault-token"), PatternTier::Infrastructure);
        assert_eq!(get_pattern_tier("grafana-api-key"), PatternTier::Infrastructure);
    }

    #[test]
    fn test_services_patterns() {
        assert_eq!(get_pattern_tier("razorpay-api-key"), PatternTier::Services);
        assert_eq!(get_pattern_tier("paystack"), PatternTier::Services);
    }

    #[test]
    fn test_generic_patterns() {
        assert_eq!(get_pattern_tier("jwt"), PatternTier::Patterns);
        assert_eq!(get_pattern_tier("bearer-token"), PatternTier::Patterns);
        assert_eq!(get_pattern_tier("private-key"), PatternTier::Patterns);
    }

    #[test]
    fn test_default_unknown() {
        // Unknown patterns should default to generic
        assert_eq!(get_pattern_tier("unknown-pattern"), PatternTier::Patterns);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(get_pattern_tier("AWS-AKIA"), PatternTier::Critical);
        assert_eq!(get_pattern_tier("GitHub-Token"), PatternTier::Critical);
    }
}
