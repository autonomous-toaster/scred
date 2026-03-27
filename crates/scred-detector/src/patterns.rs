//! Pattern definitions - extracted from Zig source of truth
//! All 275 patterns organized by detection type

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternTier {
    Critical,
    Infrastructure,
    Services,
    ApiKeys,
    Patterns,
}

// ============================================================================
// SIMPLE PREFIX PATTERNS (26 total)
// Fast path: just check if text starts with prefix, no validation
// ============================================================================

#[derive(Debug, Clone)]
pub struct SimplePrefixPattern {
    pub name: &'static str,
    pub prefix: &'static str,
    pub tier: PatternTier,
}

pub const SIMPLE_PREFIX_PATTERNS: &[SimplePrefixPattern] = &[
    SimplePrefixPattern { name: "artifactoryreferencetoken", prefix: "cmVmdGtu", tier: PatternTier::Infrastructure },
    SimplePrefixPattern { name: "azure-storage", prefix: "AccountName", tier: PatternTier::Infrastructure },
    SimplePrefixPattern { name: "azure-app-config", prefix: "Endpoint=https://", tier: PatternTier::Infrastructure },
    SimplePrefixPattern { name: "coinbase", prefix: "organizations/", tier: PatternTier::Services },
    SimplePrefixPattern { name: "context7-api-key", prefix: "ctx7sk_", tier: PatternTier::Critical },
    SimplePrefixPattern { name: "context7-secret", prefix: "ctx7sk-", tier: PatternTier::Critical },
    SimplePrefixPattern { name: "langsmith-deployment-key", prefix: "lsv2_sk_", tier: PatternTier::Critical },
    SimplePrefixPattern { name: "pypi-upload-token", prefix: "pypi-AgEIcHlwaS5vcmc", tier: PatternTier::Services },
    SimplePrefixPattern { name: "salad-cloud-api-key", prefix: "salad_cloud_", tier: PatternTier::Infrastructure },
    SimplePrefixPattern { name: "sentry-access-token", prefix: "bsntrys_", tier: PatternTier::ApiKeys },
    SimplePrefixPattern { name: "travisoauth", prefix: "travis_", tier: PatternTier::ApiKeys },
    SimplePrefixPattern { name: "tumblr-api-key", prefix: "tumblr_", tier: PatternTier::Services },
    SimplePrefixPattern { name: "upstash-redis", prefix: "redis_", tier: PatternTier::Infrastructure },
    SimplePrefixPattern { name: "vercel-token", prefix: "vercel_", tier: PatternTier::ApiKeys },
    // AWS patterns (all use simple prefix check)
    SimplePrefixPattern { name: "aws-akia", prefix: "AKIA", tier: PatternTier::Critical },
    SimplePrefixPattern { name: "aws-asia", prefix: "ASIA", tier: PatternTier::Critical },
    SimplePrefixPattern { name: "aws-abia", prefix: "ABIA", tier: PatternTier::Critical },
    SimplePrefixPattern { name: "aws-acca", prefix: "ACCA", tier: PatternTier::Critical },
    // GitHub patterns
    SimplePrefixPattern { name: "github-ghp", prefix: "ghp_", tier: PatternTier::Critical },
    SimplePrefixPattern { name: "github-ghu", prefix: "ghu_", tier: PatternTier::Critical },
    SimplePrefixPattern { name: "github-ghs", prefix: "ghs_", tier: PatternTier::Critical },
    // OpenAI patterns
    SimplePrefixPattern { name: "openai-sk-proj", prefix: "sk-proj-", tier: PatternTier::Critical },
    SimplePrefixPattern { name: "openai-sk", prefix: "sk-", tier: PatternTier::Critical },
    // NOTE: Removed 4 overly broad generic patterns (generic-password, generic-password-colon,
    // generic-password-lower, generic-secret) to eliminate false positives.
    // Real secrets are caught by more specific patterns:
    // - Database passwords: MYSQL_PASSWORD=, POSTGRES_PASSWORD=, REDIS_PASSWORD=, etc.
    // - Secrets/tokens: Specific env patterns (_SECRET=, _TOKEN=, _API_KEY=)
    // - Generic "PASSWORD=" was matching demo values (PASSWORD=demo123)
];

// ============================================================================
// PREFIX VALIDATION PATTERNS (45 total)
// Medium path: check prefix + validate token length and charset
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charset {
    Alphanumeric,
    Base64,
    Base64Url,
    Hex,
    Any,
}

#[derive(Debug, Clone)]
pub struct PrefixValidationPattern {
    pub name: &'static str,
    pub prefix: &'static str,
    pub tier: PatternTier,
    pub min_len: usize,
    pub max_len: usize,
    pub charset: Charset,
}

pub const PREFIX_VALIDATION_PATTERNS: &[PrefixValidationPattern] = &[
    PrefixValidationPattern { name: "1password-svc-token", prefix: "ops_eyJ", tier: PatternTier::Infrastructure, min_len: 250, max_len: 300, charset: Charset::Base64 },
    PrefixValidationPattern { name: "assertible", prefix: "assertible_", tier: PatternTier::Services, min_len: 20, max_len: 200, charset: Charset::Any },
    PrefixValidationPattern { name: "atlassian", prefix: "AAAAA", tier: PatternTier::ApiKeys, min_len: 20, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "checkr-personal-access-token", prefix: "chk_live_", tier: PatternTier::Services, min_len: 40, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "circleci-personal-access-token", prefix: "ccpat_", tier: PatternTier::ApiKeys, min_len: 40, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "contentful-personal-access-token", prefix: "CFPAT-", tier: PatternTier::ApiKeys, min_len: 43, max_len: 43, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "databricks-token", prefix: "dapi", tier: PatternTier::Infrastructure, min_len: 32, max_len: 200, charset: Charset::Hex },
    PrefixValidationPattern { name: "digicert-api-key", prefix: "d7dc", tier: PatternTier::Services, min_len: 20, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "expo-access-token", prefix: "ExponentPushToken[", tier: PatternTier::Services, min_len: 60, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "figma-token", prefix: "figd_", tier: PatternTier::Services, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "flutterwave", prefix: "FLWRSP_TEST_", tier: PatternTier::Services, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "gandi-api-key", prefix: "Ov23li", tier: PatternTier::Services, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "generic-api-key", prefix: "X-API-KEY:", tier: PatternTier::Patterns, min_len: 20, max_len: 500, charset: Charset::Any },
    PrefixValidationPattern { name: "gitee-access-token", prefix: "glpat-", tier: PatternTier::ApiKeys, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "github-token-detailed", prefix: "ghp_", tier: PatternTier::Critical, min_len: 36, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "gitlab-token-detailed", prefix: "glpat-", tier: PatternTier::ApiKeys, min_len: 40, max_len: 300, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "google-gemini", prefix: "AIzaSy", tier: PatternTier::ApiKeys, min_len: 33, max_len: 33, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "grafana-api-key", prefix: "eyJrIjoiZWQ", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Base64 },
    PrefixValidationPattern { name: "heroku-api-key", prefix: "heroku_", tier: PatternTier::ApiKeys, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "hubspot-api-key", prefix: "pat-", tier: PatternTier::ApiKeys, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "huggingface-token", prefix: "hf_", tier: PatternTier::ApiKeys, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "mailgun-api-key", prefix: "key-", tier: PatternTier::ApiKeys, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "mapbox-token", prefix: "pk.", tier: PatternTier::ApiKeys, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "mailchimp-api-key", prefix: "us", tier: PatternTier::ApiKeys, min_len: 32, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "minio-access-key", prefix: "minioadmin", tier: PatternTier::Infrastructure, min_len: 20, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "newrelic-api-key", prefix: "NRAK-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "npm-token", prefix: "npm_", tier: PatternTier::ApiKeys, min_len: 40, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "noco-auth-token", prefix: "eyJhbGciOiJIUzI1N", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Base64 },
    PrefixValidationPattern { name: "notion-api-key", prefix: "secret_", tier: PatternTier::Services, min_len: 40, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "okta-token", prefix: "okta_", tier: PatternTier::Critical, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "pagerduty-api-key", prefix: "u+", tier: PatternTier::ApiKeys, min_len: 32, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "paypal-signature", prefix: "sig=", tier: PatternTier::Services, min_len: 50, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "postman-api-key", prefix: "PMAK-", tier: PatternTier::ApiKeys, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "pulumi-api-token", prefix: "pul-", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "sendbird-api-token", prefix: "SendBird.", tier: PatternTier::Services, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "sendgrid-api-key", prefix: "SG.", tier: PatternTier::Services, min_len: 60, max_len: 200, charset: Charset::Base64 },
    PrefixValidationPattern { name: "sentry-auth-token", prefix: "sntrys_eyJ", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Base64 },
    PrefixValidationPattern { name: "slack-bot-token", prefix: "xoxb-", tier: PatternTier::Critical, min_len: 40, max_len: 300, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "slack-user-token", prefix: "xoxp-", tier: PatternTier::Critical, min_len: 40, max_len: 300, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "stripe-api-key-live", prefix: "sk_live_", tier: PatternTier::Critical, min_len: 32, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "stripe-api-key-test", prefix: "sk_test_", tier: PatternTier::Critical, min_len: 32, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "supabase-api-key", prefix: "eyJhbGciOiJIUzI1N", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Base64 },
    PrefixValidationPattern { name: "twilio-api-key", prefix: "AC", tier: PatternTier::Critical, min_len: 32, max_len: 34, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "telegram-bot-token", prefix: "bot", tier: PatternTier::Services, min_len: 24, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "vault-api-token", prefix: "hvs.", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Base64Url },
    
    // ============================================================================
    // BATCH 1: TDD Implementation - 25 Core Patterns from test_cases.csv
    // ============================================================================
    // AWS (3 patterns)
    PrefixValidationPattern { name: "aws-access-token", prefix: "AKIA", tier: PatternTier::Critical, min_len: 20, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "aws-session-token", prefix: "ASIA", tier: PatternTier::Critical, min_len: 20, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "aws-secret-access-key", prefix: "aws-secret-access-key_", tier: PatternTier::Critical, min_len: 40, max_len: 200, charset: Charset::Any },
    
    // GitHub (5 patterns)
    PrefixValidationPattern { name: "github-pat", prefix: "ghp_", tier: PatternTier::Critical, min_len: 36, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "github-oauth", prefix: "gho_", tier: PatternTier::Critical, min_len: 36, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "github-user", prefix: "ghu_", tier: PatternTier::Critical, min_len: 36, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "github-refresh", prefix: "ghr_", tier: PatternTier::ApiKeys, min_len: 36, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "github-app-token", prefix: "ghs_", tier: PatternTier::ApiKeys, min_len: 36, max_len: 200, charset: Charset::Alphanumeric },
    
    // Slack (3 patterns)
    PrefixValidationPattern { name: "slack-bot-token", prefix: "xoxb-", tier: PatternTier::Critical, min_len: 50, max_len: 300, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "slack-user-token", prefix: "xoxp-", tier: PatternTier::Critical, min_len: 50, max_len: 300, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "slackwebhook", prefix: "slackwebhook_", tier: PatternTier::Services, min_len: 40, max_len: 300, charset: Charset::Any },
    
    // Stripe (3 patterns)
    PrefixValidationPattern { name: "stripe", prefix: "stripe_", tier: PatternTier::Critical, min_len: 40, max_len: 200, charset: Charset::Any },
    PrefixValidationPattern { name: "stripepaymentintent", prefix: "stripepaymentintent_", tier: PatternTier::Services, min_len: 40, max_len: 300, charset: Charset::Any },
    PrefixValidationPattern { name: "stripepaymentintent-1", prefix: "stripepaymentintent-1_", tier: PatternTier::Services, min_len: 40, max_len: 300, charset: Charset::Any },
    
    // Google (2 patterns)
    PrefixValidationPattern { name: "googlegemini", prefix: "googlegemini_", tier: PatternTier::Services, min_len: 40, max_len: 300, charset: Charset::Any },
    PrefixValidationPattern { name: "googleoauth2", prefix: "googleoauth2_", tier: PatternTier::Services, min_len: 40, max_len: 300, charset: Charset::Any },
    
    // Azure (3 patterns)
    PrefixValidationPattern { name: "azure-ad-client-secret", prefix: "azure-ad-client-secret_", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Any },
    PrefixValidationPattern { name: "azure_storage", prefix: "azure_storage_", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Any },
    PrefixValidationPattern { name: "azurefunctionkey", prefix: "azurefunctionkey_", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Any },
    
    // OpenAI (1 pattern)
    PrefixValidationPattern { name: "anthropic", prefix: "anthropic_", tier: PatternTier::Services, min_len: 40, max_len: 200, charset: Charset::Any },
    
    // Miscellaneous (2 patterns)
    PrefixValidationPattern { name: "adafruitio", prefix: "adafruitio_", tier: PatternTier::Services, min_len: 40, max_len: 200, charset: Charset::Any },
    PrefixValidationPattern { name: "1password-service-account-token", prefix: "1password-service-account-token_", tier: PatternTier::Infrastructure, min_len: 40, max_len: 300, charset: Charset::Any },
    
    // ============================================================================
    // BATCH 2: Infrastructure & System Authentication (25 patterns)
    // ============================================================================
    // Password Hashes (3 patterns)
    PrefixValidationPattern { name: "bcrypt-password-hash", prefix: "$2", tier: PatternTier::Infrastructure, min_len: 60, max_len: 200, charset: Charset::Base64 },
    PrefixValidationPattern { name: "sha256-password-hash", prefix: "$5$", tier: PatternTier::Infrastructure, min_len: 50, max_len: 200, charset: Charset::Base64 },
    PrefixValidationPattern { name: "sha512-password-hash", prefix: "$6$", tier: PatternTier::Infrastructure, min_len: 100, max_len: 200, charset: Charset::Base64 },
    
    // Database URI Patterns: REMOVED - These patterns cause false positives
    // Reason: Match "scheme://host:port" without detecting credentials (no colon for password)
    // Example false positives: "redis://localhost:6379", "postgres://db.example.com"
    // Solution: Use environment variable patterns instead (DATABASE_URL=, REDIS_URL=, etc.)
    // These env patterns catch real secrets and don't false positive on example URLs
    
    // Environment Variables (5 patterns)
    // DATABASE_URL & REDIS_URL: Fixed false positives on localhost/example URLs
    // Increased min_len to 40+ to avoid matching localhost examples
    // Real production connection strings with credentials are 50-100+ chars
    PrefixValidationPattern { name: "database-url-env", prefix: "DATABASE_URL=", tier: PatternTier::Critical, min_len: 50, max_len: 200, charset: Charset::Any },
    PrefixValidationPattern { name: "redis-url-env", prefix: "REDIS_URL=", tier: PatternTier::Critical, min_len: 40, max_len: 150, charset: Charset::Any },
    PrefixValidationPattern { name: "github-token-env", prefix: "GITHUB_TOKEN=ghp_", tier: PatternTier::Critical, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "docker-config-auth-env", prefix: "DOCKER_CONFIG_AUTH=", tier: PatternTier::Infrastructure, min_len: 40, max_len: 300, charset: Charset::Base64 },
    PrefixValidationPattern { name: "ssh-key-path-env", prefix: "SSH_KEY_PATH=/", tier: PatternTier::Infrastructure, min_len: 20, max_len: 500, charset: Charset::Any },
    
    // Generic Environment Variable Suffixes: Fixed min_len too short (critical security issue)
    // TOKEN was min_len:10, PASSWORD was min_len:8 - both dangerously short!
    // Updated to min_len:20 for meaningful secrets + added max_len bounds
    PrefixValidationPattern { name: "env-client-secret-suffix", prefix: "_CLIENT_SECRET=", tier: PatternTier::Critical, min_len: 20, max_len: 500, charset: Charset::Any },
    PrefixValidationPattern { name: "env-api-key-suffix", prefix: "_API_KEY=", tier: PatternTier::Critical, min_len: 15, max_len: 500, charset: Charset::Any },
    PrefixValidationPattern { name: "env-token-suffix", prefix: "_TOKEN=", tier: PatternTier::Critical, min_len: 20, max_len: 500, charset: Charset::Any },
    PrefixValidationPattern { name: "env-secret-suffix", prefix: "_SECRET=", tier: PatternTier::Critical, min_len: 15, max_len: 500, charset: Charset::Any },
    PrefixValidationPattern { name: "env-password-suffix", prefix: "_PASSWORD=", tier: PatternTier::Critical, min_len: 20, max_len: 500, charset: Charset::Any },
    PrefixValidationPattern { name: "env-api-key-lowercase", prefix: "api_key=", tier: PatternTier::Critical, min_len: 15, max_len: 500, charset: Charset::Any },
    // PGPASSWORD & PASSPHRASE: Fixed min_len too short (was 8 - dangerously permissive)
    PrefixValidationPattern { name: "pgpassword-env", prefix: "PGPASSWORD=", tier: PatternTier::Critical, min_len: 20, max_len: 500, charset: Charset::Any },
    PrefixValidationPattern { name: "passphrase-env", prefix: "PASSPHRASE=", tier: PatternTier::Critical, min_len: 20, max_len: 500, charset: Charset::Any },
    PrefixValidationPattern { name: "env-client-secret-lowercase", prefix: "client_secret=", tier: PatternTier::Critical, min_len: 20, max_len: 500, charset: Charset::Any },
    
    // Vault / Hashicorp (3 patterns)
    PrefixValidationPattern { name: "vault-api-token", prefix: "hvs.", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "vault-service-token", prefix: "s.", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "vault-batch-token", prefix: "b.", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    
    // Container & Orchestration (4 patterns)
    PrefixValidationPattern { name: "docker-swarm-token", prefix: "SWMTKN-", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "docker-api-key", prefix: "dckr_pat_", tier: PatternTier::Critical, min_len: 40, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "kubernetes-bearer-token", prefix: "eyJhbGc", tier: PatternTier::Critical, min_len: 100, max_len: 300, charset: Charset::Base64 },
    PrefixValidationPattern { name: "docker-registry-token", prefix: "eyJ0eXAiOiJ", tier: PatternTier::Infrastructure, min_len: 100, max_len: 200, charset: Charset::Base64 },
    
    // Cloud Provider Config (3 patterns)
    PrefixValidationPattern { name: "aws-secret-env", prefix: "AWS_SECRET_ACCESS_KEY=", tier: PatternTier::Critical, min_len: 40, max_len: 200, charset: Charset::Any },
    PrefixValidationPattern { name: "gcp-private-key-id", prefix: "private_key_id=", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Any },
    PrefixValidationPattern { name: "azure-connection-string", prefix: "DefaultEndpointsProtocol=", tier: PatternTier::Critical, min_len: 50, max_len: 300, charset: Charset::Any },
    
    // ============================================================================
    // BATCH 3: API Keys & Services (25 patterns)
    // Additional Database/Cache Services: REMOVED for false positive elimination
    // These patterns (dynamodb-uri, memcached-uri, couchdb-uri, influxdb-uri) caused false positives
    // Use environment variable patterns instead (DYNAMODB_URL=, etc.)
    
    // Communication Platforms (4 patterns)
    PrefixValidationPattern { name: "discord-bot-token", prefix: "ODI", tier: PatternTier::Infrastructure, min_len: 60, max_len: 300, charset: Charset::Base64 },
    PrefixValidationPattern { name: "telegram-token-env", prefix: "TELEGRAM_BOT_TOKEN=", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "slack-webhook-url", prefix: "https://hooks.slack.com/services/", tier: PatternTier::Infrastructure, min_len: 60, max_len: 300, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "zulip-api-key", prefix: "Zulip", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Development Tools & CI/CD (5 patterns)
    PrefixValidationPattern { name: "sonarqube-token", prefix: "squ_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "codecov-token", prefix: "codecov-token=", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "buildkite-token", prefix: "bk_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "dependabot-token", prefix: "ghs_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "sourcegraph-token", prefix: "sgp_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    
    // Analytics & Monitoring (4 patterns)
    PrefixValidationPattern { name: "datadog-api-key", prefix: "dd_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "amplitude-api-key", prefix: "amplitude-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "mixpanel-api-token", prefix: "mixpanel_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Email & Message Services (3 patterns)
    PrefixValidationPattern { name: "sparkpost-api-key", prefix: "sparkpost-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "sendpulse-api-token", prefix: "sendpulse-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "mailerlite-api-key", prefix: "mailerlite-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Storage & CDN Services (3 patterns)
    PrefixValidationPattern { name: "digitalocean-spaces-key", prefix: "DO00", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64 },
    PrefixValidationPattern { name: "backblaze-b2-key", prefix: "b2_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "wasabi-s3-key", prefix: "wasabi-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // ============================================================================
    // BATCH 4: Specialized Services & Cloud Platforms (25 patterns)
    // ============================================================================
    // AI/ML Platforms (5 patterns)
    PrefixValidationPattern { name: "replicate-api-token", prefix: "r8_", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "anthropic-api-key", prefix: "sk-ant-", tier: PatternTier::Critical, min_len: 40, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "mistral-api-key", prefix: "sk-", tier: PatternTier::Critical, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "together-api-key", prefix: "together-", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "modal-token-env", prefix: "MODAL_TOKEN_ID=", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Blockchain & Crypto (4 patterns)
    PrefixValidationPattern { name: "ethereum-private-key", prefix: "0x", tier: PatternTier::Critical, min_len: 66, max_len: 66, charset: Charset::Hex },
    PrefixValidationPattern { name: "infura-api-key", prefix: "infura-", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "alchemy-api-key", prefix: "alchemy-", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "web3-storage-token", prefix: "eyJ", tier: PatternTier::Infrastructure, min_len: 100, max_len: 200, charset: Charset::Base64 },
    
    // Incident Management & Collaboration (4 patterns)
    PrefixValidationPattern { name: "victorops-api-key", prefix: "victorops-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "opsgenie-api-key", prefix: "opsgenie-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "jira-api-token", prefix: "jira-api-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "linear-api-key", prefix: "lin_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    
    // Data & Search Platforms (4 patterns)
    PrefixValidationPattern { name: "algolia-api-key", prefix: "algolia-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "typesense-api-key", prefix: "typesense-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "meilisearch-api-key", prefix: "meili_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "weaviate-api-key", prefix: "weaviate-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Identity & Auth Platforms (4 patterns)
    PrefixValidationPattern { name: "auth0-management-token", prefix: "mgmt_", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "auth0-client-secret", prefix: "client_secret=", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    // cognito-user-pool-id: REMOVED (false positives on cognito-idp:// URIs without credentials)
    PrefixValidationPattern { name: "keycloak-client-secret", prefix: "keycloak-secret=", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    
    // Enterprise & SaaS Platforms (4 patterns)
    PrefixValidationPattern { name: "servicenow-api-key", prefix: "servicenow-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "salesforce-api-key", prefix: "salesforce-", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "zendesk-api-token", prefix: "zendesk-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    // supabase-postgres-password: REMOVED (false positives on postgres:// URIs like localhost, example.com)
    
    // ============================================================================
    // BATCH 5: Additional Providers & Niche Services (25 patterns)
    // ============================================================================
    // Game Development Platforms (4 patterns)
    PrefixValidationPattern { name: "unity-api-key", prefix: "unity-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "unreal-api-key", prefix: "unreal-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "godot-export-key", prefix: "godot-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "steam-api-key", prefix: "steam-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Documentation & Publishing (4 patterns)
    PrefixValidationPattern { name: "readme-io-api-key", prefix: "rdme_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "gitbook-api-token", prefix: "gbook_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "contentful-cpa-token", prefix: "CFPAT-", tier: PatternTier::Infrastructure, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "strapi-api-token", prefix: "strapi-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Automation & Workflow Platforms (5 patterns)
    PrefixValidationPattern { name: "zapier-api-key", prefix: "zapier-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "make-integration-token", prefix: "make_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "n8n-api-key", prefix: "n8n_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "workflow-automation-token", prefix: "wf_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "ifttt-webhook-key", prefix: "ifttt-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Design & Collaboration Tools (4 patterns)
    PrefixValidationPattern { name: "miro-api-token", prefix: "miro_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "lucidchart-api-key", prefix: "lucidchart-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "craft-api-token", prefix: "craft_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "invision-api-key", prefix: "invision-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Payment & Financial Services (3 patterns)
    PrefixValidationPattern { name: "square-access-token", prefix: "sq0atp-", tier: PatternTier::Critical, min_len: 40, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "wise-api-token", prefix: "wise_", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "coinbase-api-key", prefix: "cb-", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    
    // Quality Assurance & Testing (3 patterns)
    PrefixValidationPattern { name: "browserstack-api-key", prefix: "browserstack-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "testproject-io-token", prefix: "testproject_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "lambdatest-api-key", prefix: "lambdatest-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // ============================================================================
    // BATCH 6: Enterprise Tools & Monitoring Platforms (25 patterns)
    // ============================================================================
    // Application Performance Monitoring (5 patterns)
    PrefixValidationPattern { name: "new-relic-license-key", prefix: "NRJS-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "dynatrace-api-token", prefix: "dt0c01.", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "datadog-api-key-v2", prefix: "dda89f2d8b94f66", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Hex },
    PrefixValidationPattern { name: "elastic-api-key", prefix: "elastic-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "splunk-hec-token", prefix: "splunk-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Logging & Log Aggregation (4 patterns)
    PrefixValidationPattern { name: "logz-io-api-token", prefix: "logz_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "papertrail-api-token", prefix: "papertrail-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "sumo-logic-access-token", prefix: "sumo_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "loggly-api-token", prefix: "loggly-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Security & Vulnerability Management (4 patterns)
    PrefixValidationPattern { name: "snyk-api-token", prefix: "snyk_", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "crowdstrike-api-token", prefix: "crowdstrike-", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "qualys-api-token", prefix: "qualys-", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "rapid7-api-key", prefix: "rapid7_", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Cloud-Specific Tools (4 patterns)
    PrefixValidationPattern { name: "terraform-cloud-token", prefix: "api-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "pulumi-access-token", prefix: "pul-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "cloudsmith-api-key", prefix: "cloudsmith_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "artifactory-cloud-key", prefix: "AKCp", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Issue Tracking & Project Management (3 patterns)
    PrefixValidationPattern { name: "youtrack-api-token", prefix: "youtrack-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "taiga-api-token", prefix: "taiga_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "redmine-api-key", prefix: "redmine-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Container & Registry Services (3 patterns)
    PrefixValidationPattern { name: "quay-io-token", prefix: "quay_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "artifactory-helm-token", prefix: "AKCpD", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "jfrog-xray-token", prefix: "xray-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Additional Enterprise Services (2 patterns)
    PrefixValidationPattern { name: "chef-api-token", prefix: "chef-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "puppet-api-token", prefix: "puppet-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // ============================================================================
    // BATCH 7: Final SaaS & Specialized Services (25 patterns)
    // ============================================================================
    // Productivity & Office Tools (4 patterns)
    PrefixValidationPattern { name: "airtable-api-token", prefix: "patU", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "notion-integration-token", prefix: "secret_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "confluence-api-token", prefix: "confluence_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "asana-api-token", prefix: "asana_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Real-Time Communication (4 patterns)
    PrefixValidationPattern { name: "twitch-oauth-token", prefix: "oauth-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "discord-webhook-url", prefix: "https://discord.com/api/webhooks/", tier: PatternTier::Infrastructure, min_len: 50, max_len: 500, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "matrix-homeserver-token", prefix: "syt_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    PrefixValidationPattern { name: "rocket-chat-token", prefix: "rocketchat-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Knowledge Management & Docs (3 patterns)
    PrefixValidationPattern { name: "confluence-cloud-token", prefix: "atc_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "jira-cloud-token", prefix: "atb_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "slite-api-token", prefix: "slite-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Marketplace & Integration Platforms (4 patterns)
    PrefixValidationPattern { name: "shopify-api-key", prefix: "shppa_", tier: PatternTier::Critical, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "wix-api-token", prefix: "wix_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "bigcommerce-api-token", prefix: "bigcommerce-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "intercom-access-token", prefix: "intercom-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Social Media & Content (3 patterns)
    PrefixValidationPattern { name: "instagram-business-token", prefix: "IGQWRN", tier: PatternTier::Infrastructure, min_len: 100, max_len: 200, charset: Charset::Base64 },
    PrefixValidationPattern { name: "tiktok-api-token", prefix: "tiktok-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "youtube-api-key", prefix: "AIza", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Base64Url },
    
    // Analytics & Data (2 patterns)
    PrefixValidationPattern { name: "segment-write-key", prefix: "segment-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "chartio-api-token", prefix: "chartio-", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // Additional Specialized Services (2 patterns)
    PrefixValidationPattern { name: "fastapi-key", prefix: "fk_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    PrefixValidationPattern { name: "graphql-api-token", prefix: "gql_", tier: PatternTier::Infrastructure, min_len: 30, max_len: 200, charset: Charset::Alphanumeric },
    
    // ===== TIER 1 DATABASE & INFRASTRUCTURE (6 patterns) =====
    // MySQL official password environment variable
    PrefixValidationPattern { name: "mysql-password-env", prefix: "MYSQL_PWD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // RabbitMQ default password environment variable (standard in Docker)
    PrefixValidationPattern { name: "rabbitmq-default-pass-env", prefix: "RABBITMQ_DEFAULT_PASS=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // Redis password environment variable
    PrefixValidationPattern { name: "redis-password-env", prefix: "REDIS_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // PostgreSQL root password (different from PGPASSWORD which is for client connections)
    PrefixValidationPattern { name: "postgres-root-password-env", prefix: "POSTGRES_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // Docker Registry password for authentication (CI/CD)
    PrefixValidationPattern { name: "docker-registry-password-env", prefix: "DOCKER_REGISTRY_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // HashiCorp Vault token (auth token enables full vault access)
    PrefixValidationPattern { name: "vault-token-env", prefix: "VAULT_TOKEN=", tier: PatternTier::Critical, min_len: 20, max_len: 200, charset: Charset::Alphanumeric },
    
    // ===== TIER 2 ENTERPRISE & SPECIALIZED SERVICES (9 patterns) =====
    // LDAP directory service password
    PrefixValidationPattern { name: "ldap-password-env", prefix: "LDAP_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // LDAP bind password (service account authentication)
    PrefixValidationPattern { name: "ldap-bind-password-env", prefix: "LDAP_BIND_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // Apache Cassandra NoSQL database password
    PrefixValidationPattern { name: "cassandra-password-env", prefix: "CASSANDRA_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // Elasticsearch search engine password
    PrefixValidationPattern { name: "elasticsearch-password-env", prefix: "ELASTICSEARCH_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // Apache CouchDB document database password
    PrefixValidationPattern { name: "couchdb-password-env", prefix: "COUCHDB_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // Apache Kafka SASL authentication password
    PrefixValidationPattern { name: "kafka-sasl-password-env", prefix: "KAFKA_SASL_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // Apache ActiveMQ message broker password
    PrefixValidationPattern { name: "activemq-password-env", prefix: "ACTIVEMQ_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // Bitbucket Git server password (for CI/CD authentication)
    PrefixValidationPattern { name: "bitbucket-password-env", prefix: "BITBUCKET_PASSWORD=", tier: PatternTier::Infrastructure, min_len: 15, max_len: 100, charset: Charset::Any },
    
    // SMTP mail server password (email authentication)
    PrefixValidationPattern { name: "smtp-password-env", prefix: "SMTP_PASSWORD=", tier: PatternTier::Critical, min_len: 15, max_len: 100, charset: Charset::Any },
    PrefixValidationPattern {
        name: "bcrypt-password-hash",
        prefix: "$2",  // $2a, $2y, $2x variants
        tier: PatternTier::Infrastructure,
        min_len: 60,   // Bcrypt hashes are always 60 chars
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "sha256-password-hash",
        prefix: "$5$",
        tier: PatternTier::Infrastructure,
        min_len: 50,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "sha512-password-hash",
        prefix: "$6$",
        tier: PatternTier::Infrastructure,
        min_len: 100,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "redis-url-env",
        prefix: "REDIS_URL=",
        tier: PatternTier::Critical,
        min_len: 20,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "github-token-env",
        prefix: "GITHUB_TOKEN=ghp_",
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "docker-config-auth-env",
        prefix: "DOCKER_CONFIG_AUTH=",
        tier: PatternTier::Infrastructure,
        min_len: 40,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "ssh-key-path-env",
        prefix: "SSH_KEY_PATH=/",
        tier: PatternTier::Infrastructure,
        min_len: 20,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "vault-api-token",
        prefix: "hvs.",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "vault-service-token",
        prefix: "s.",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "vault-batch-token",
        prefix: "b.",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "docker-swarm-token",
        prefix: "SWMTKN-",
        tier: PatternTier::Infrastructure,
        min_len: 40,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "docker-api-key",
        prefix: "dckr_pat_",
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "kubernetes-bearer-token",
        prefix: "eyJhbGc",  // Base64 for JWT header {"alg"
        tier: PatternTier::Critical,
        min_len: 100,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "docker-registry-token",
        prefix: "eyJ0eXAiOiJ",  // Base64 for JWT header {"typ":"JWT"
        tier: PatternTier::Infrastructure,
        min_len: 100,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "aws-secret-env",
        prefix: "AWS_SECRET_ACCESS_KEY=",
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "gcp-private-key-id",
        prefix: "private_key_id=",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "azure-connection-string",
        prefix: "DefaultEndpointsProtocol=",
        tier: PatternTier::Critical,
        min_len: 50,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "discord-bot-token",
        prefix: "ODI",  // Base64 for bot token format
        tier: PatternTier::Infrastructure,
        min_len: 60,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "telegram-token-env",
        prefix: "TELEGRAM_BOT_TOKEN=",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "zulip-api-key",
        prefix: "Zulip",  // Zulip tokens often start with "Zulip"
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sonarqube-token",
        prefix: "squ_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "codecov-token",
        prefix: "codecov-token=",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "buildkite-token",
        prefix: "bk_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "dependabot-token",
        prefix: "ghs_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sourcegraph-token",
        prefix: "sgp_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "datadog-api-key",
        prefix: "dd_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sentry-dsn-url",
        prefix: "https://",
        tier: PatternTier::Infrastructure,
        min_len: 50,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "amplitude-api-key",
        prefix: "amplitude-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "mixpanel-api-token",
        prefix: "mixpanel_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sparkpost-api-key",
        prefix: "sparkpost-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sendpulse-api-token",
        prefix: "sendpulse-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "mailerlite-api-key",
        prefix: "mailerlite-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "digitalocean-spaces-key",
        prefix: "DO00",  // Base64 encoding of DO token
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "backblaze-b2-key",
        prefix: "b2_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "wasabi-s3-key",
        prefix: "wasabi-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "replicate-api-token",
        prefix: "r8_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "anthropic-api-key",
        prefix: "sk-ant-",
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "mistral-api-key",
        prefix: "sk-",  // Mistral keys use sk- prefix
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "together-api-key",
        prefix: "together-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "modal-token-env",
        prefix: "MODAL_TOKEN_ID=",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "ethereum-private-key",
        prefix: "0x",
        tier: PatternTier::Critical,
        min_len: 66,  // 0x + 64 hex chars
        max_len: 66,
        charset: Charset::Hex,
    },
    PrefixValidationPattern {
        name: "infura-api-key",
        prefix: "infura-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "alchemy-api-key",
        prefix: "alchemy-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "web3-storage-token",
        prefix: "eyJ",  // Base64 for JWT header
        tier: PatternTier::Infrastructure,
        min_len: 100,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "victorops-api-key",
        prefix: "victorops-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "opsgenie-api-key",
        prefix: "opsgenie-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "jira-api-token",
        prefix: "jira-api-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "linear-api-key",
        prefix: "lin_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "algolia-api-key",
        prefix: "algolia-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "typesense-api-key",
        prefix: "typesense-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "meilisearch-api-key",
        prefix: "meili_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "weaviate-api-key",
        prefix: "weaviate-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "auth0-management-token",
        prefix: "mgmt_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "auth0-client-secret",
        prefix: "client_secret=",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "cognito-user-pool-id",
        prefix: "cognito-idp://",
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "keycloak-client-secret",
        prefix: "keycloak-secret=",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "servicenow-api-key",
        prefix: "servicenow-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "salesforce-api-key",
        prefix: "salesforce-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "zendesk-api-token",
        prefix: "zendesk-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "supabase-postgres-password",
        prefix: "postgres://",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "unity-api-key",
        prefix: "unity-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "unreal-api-key",
        prefix: "unreal-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "godot-export-key",
        prefix: "godot-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "steam-api-key",
        prefix: "steam-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "readme-io-api-key",
        prefix: "rdme_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "gitbook-api-token",
        prefix: "gbook_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "contentful-cpa-token",
        prefix: "CFPAT-",
        tier: PatternTier::Infrastructure,
        min_len: 40,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "strapi-api-token",
        prefix: "strapi-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "zapier-api-key",
        prefix: "zapier-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "make-integration-token",
        prefix: "make_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "n8n-api-key",
        prefix: "n8n_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "workflow-automation-token",
        prefix: "wf_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "ifttt-webhook-key",
        prefix: "ifttt-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "miro-api-token",
        prefix: "miro_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "lucidchart-api-key",
        prefix: "lucidchart-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "craft-api-token",
        prefix: "craft_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "invision-api-key",
        prefix: "invision-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "square-access-token",
        prefix: "sq0atp-",
        tier: PatternTier::Critical,
        min_len: 40,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "wise-api-token",
        prefix: "wise_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "coinbase-api-key",
        prefix: "cb-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "browserstack-api-key",
        prefix: "browserstack-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "testproject-io-token",
        prefix: "testproject_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "lambdatest-api-key",
        prefix: "lambdatest-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "new-relic-license-key",
        prefix: "NRJS-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "dynatrace-api-token",
        prefix: "dt0c01.",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "datadog-api-key-v2",
        prefix: "dda89f2d8b94f66",  // Datadog specific format
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Hex,
    },
    PrefixValidationPattern {
        name: "elastic-api-key",
        prefix: "elastic-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "splunk-hec-token",
        prefix: "splunk-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "logz-io-api-token",
        prefix: "logz_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "papertrail-api-token",
        prefix: "papertrail-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "sumo-logic-access-token",
        prefix: "sumo_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "loggly-api-token",
        prefix: "loggly-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "snyk-api-token",
        prefix: "snyk_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "crowdstrike-api-token",
        prefix: "crowdstrike-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "qualys-api-token",
        prefix: "qualys-",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "rapid7-api-key",
        prefix: "rapid7_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "terraform-cloud-token",
        prefix: "api-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "pulumi-access-token",
        prefix: "pul-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "cloudsmith-api-key",
        prefix: "cloudsmith_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "artifactory-cloud-key",
        prefix: "AKCp",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "youtrack-api-token",
        prefix: "youtrack-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "taiga-api-token",
        prefix: "taiga_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "redmine-api-key",
        prefix: "redmine-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "quay-io-token",
        prefix: "quay_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "artifactory-helm-token",
        prefix: "AKCpD",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "jfrog-xray-token",
        prefix: "xray-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "chef-api-token",
        prefix: "chef-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "puppet-api-token",
        prefix: "puppet-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "airtable-api-token",
        prefix: "patU",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "notion-integration-token",
        prefix: "secret_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "confluence-api-token",
        prefix: "confluence_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "asana-api-token",
        prefix: "asana_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "twitch-oauth-token",
        prefix: "oauth-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "matrix-homeserver-token",
        prefix: "syt_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "rocket-chat-token",
        prefix: "rocketchat-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "confluence-cloud-token",
        prefix: "atc_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "jira-cloud-token",
        prefix: "atb_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "slite-api-token",
        prefix: "slite-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "shopify-api-key",
        prefix: "shppa_",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "wix-api-token",
        prefix: "wix_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "bigcommerce-api-token",
        prefix: "bigcommerce-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "intercom-access-token",
        prefix: "intercom-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "instagram-business-token",
        prefix: "IGQWRN",
        tier: PatternTier::Infrastructure,
        min_len: 100,
        max_len: 0,
        charset: Charset::Base64,
    },
    PrefixValidationPattern {
        name: "tiktok-api-token",
        prefix: "tiktok-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "youtube-api-key",
        prefix: "AIza",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Base64Url,
    },
    PrefixValidationPattern {
        name: "segment-write-key",
        prefix: "segment-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "chartio-api-token",
        prefix: "chartio-",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "fastapi-key",
        prefix: "fk_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
    PrefixValidationPattern {
        name: "graphql-api-token",
        prefix: "gql_",
        tier: PatternTier::Infrastructure,
        min_len: 30,
        max_len: 0,
        charset: Charset::Alphanumeric,
    },
];

// ============================================================================
// JWT PATTERNS (1 total)
// Generic JWT: eyJ prefix with exactly 2 dots = all JWT algorithms
// ============================================================================

#[derive(Debug, Clone)]
pub struct JwtPattern {
    pub name: &'static str,
    pub tier: PatternTier,
}

pub const JWT_PATTERNS: &[JwtPattern] = &[
    JwtPattern { name: "jwt-generic", tier: PatternTier::Patterns },
];

// ============================================================================
// MULTILINE MARKER PATTERNS (4 total)
// Detect multiline secrets like SSH keys using bounded lookahead
// Pattern: -----BEGIN <TYPE> PRIVATE KEY-----...(multiline)...-----END <TYPE> PRIVATE KEY-----
// ============================================================================

#[derive(Debug, Clone)]
pub struct MultilineMarkerPattern {
    pub name: &'static str,
    pub start_marker: &'static str,
    pub end_marker: &'static str,
    pub tier: PatternTier,
    pub max_lookahead: usize,  // Max bytes to look ahead (SSH keys ~4KB max)
}

/// Generalized marker-based pattern for multiline secrets
/// Supports start_marker -> content -> end_marker format
/// Examples: SSH keys, certificates, PGP keys, kubeconfig
#[derive(Debug, Clone, Copy)]
pub struct GeneralizedMarkerPattern {
    pub name: &'static str,
    pub start_marker: &'static str,
    pub end_marker: &'static str,
    pub tier: PatternTier,
    pub max_lookahead: usize,
    
    // Optional validation keywords
    pub contains_keyword: Option<&'static str>,  // e.g., Some("PRIVATE KEY")
    pub exclude_keyword: Option<&'static str>,   // e.g., Some("PUBLIC") to skip public keys
    
    // Content characteristics (optimization hints)
    pub min_body_len: usize,                // Minimum content size between markers
    pub pattern_type: u16,                  // Type ID for pattern classification (300+ for multiline)
}

impl From<MultilineMarkerPattern> for GeneralizedMarkerPattern {
    fn from(p: MultilineMarkerPattern) -> Self {
        GeneralizedMarkerPattern {
            name: p.name,
            start_marker: p.start_marker,
            end_marker: p.end_marker,
            tier: p.tier,
            max_lookahead: p.max_lookahead,
            contains_keyword: None,
            exclude_keyword: None,
            min_body_len: 0,
            pattern_type: 300, // Default pattern type for multiline markers
        }
    }
}

pub const MULTILINE_MARKER_PATTERNS: &[MultilineMarkerPattern] = &[
    MultilineMarkerPattern {
        name: "ssh-rsa-private-key",
        start_marker: "-----BEGIN RSA PRIVATE KEY-----",
        end_marker: "-----END RSA PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
    },
    MultilineMarkerPattern {
        name: "ssh-openssh-private-key",
        start_marker: "-----BEGIN OPENSSH PRIVATE KEY-----",
        end_marker: "-----END OPENSSH PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
    },
    MultilineMarkerPattern {
        name: "ssh-private-key",
        start_marker: "-----BEGIN PRIVATE KEY-----",
        end_marker: "-----END PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
    },
    MultilineMarkerPattern {
        name: "ssh-ec-private-key",
        start_marker: "-----BEGIN EC PRIVATE KEY-----",
        end_marker: "-----END EC PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
    },
    // X.509 Certificate patterns (Phase 4b)
    MultilineMarkerPattern {
        name: "x509-certificate",
        start_marker: "-----BEGIN CERTIFICATE-----",
        end_marker: "-----END CERTIFICATE-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 10240,  // Certificates can be 5-10KB
    },
    MultilineMarkerPattern {
        name: "x509-certificate-request",
        start_marker: "-----BEGIN CERTIFICATE REQUEST-----",
        end_marker: "-----END CERTIFICATE REQUEST-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 5120,  // CSRs typically 1-3KB
    },
    MultilineMarkerPattern {
        name: "encrypted-private-key",
        start_marker: "-----BEGIN ENCRYPTED PRIVATE KEY-----",
        end_marker: "-----END ENCRYPTED PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 5120,  // Encrypted keys typically 2-4KB
    },
    MultilineMarkerPattern {
        name: "public-key",
        start_marker: "-----BEGIN PUBLIC KEY-----",
        end_marker: "-----END PUBLIC KEY-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 3072,  // Public keys typically 1-2KB
    },
    // PGP key patterns (Phase 4c)
    MultilineMarkerPattern {
        name: "pgp-private-key-block",
        start_marker: "-----BEGIN PGP PRIVATE KEY BLOCK-----",
        end_marker: "-----END PGP PRIVATE KEY BLOCK-----",
        tier: PatternTier::Critical,
        max_lookahead: 20480,  // PGP keys can be 2-20KB
    },
    MultilineMarkerPattern {
        name: "pgp-public-key-block",
        start_marker: "-----BEGIN PGP PUBLIC KEY BLOCK-----",
        end_marker: "-----END PGP PUBLIC KEY BLOCK-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 6144,  // PGP public keys typically 1-5KB
    },
    MultilineMarkerPattern {
        name: "pgp-message",
        start_marker: "-----BEGIN PGP MESSAGE-----",
        end_marker: "-----END PGP MESSAGE-----",
        tier: PatternTier::Critical,
        max_lookahead: 15360,  // PGP messages are variable (up to 15KB)
    },
];

/// Generalized multiline pattern array - Phase A1 refactoring
/// Converts MultilineMarkerPattern to GeneralizedMarkerPattern for optimization
pub const GENERALIZED_MARKER_PATTERNS: &[GeneralizedMarkerPattern] = &[
    GeneralizedMarkerPattern {
        name: "ssh-rsa-private-key",
        start_marker: "-----BEGIN RSA PRIVATE KEY-----",
        end_marker: "-----END RSA PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
        contains_keyword: Some("PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 300,
    },
    GeneralizedMarkerPattern {
        name: "ssh-openssh-private-key",
        start_marker: "-----BEGIN OPENSSH PRIVATE KEY-----",
        end_marker: "-----END OPENSSH PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
        contains_keyword: Some("OPENSSH PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 301,
    },
    GeneralizedMarkerPattern {
        name: "ssh-private-key",
        start_marker: "-----BEGIN PRIVATE KEY-----",
        end_marker: "-----END PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
        contains_keyword: Some("PRIVATE KEY"),
        exclude_keyword: Some("RSA"), // Skip RSA (covered separately)
        min_body_len: 100,
        pattern_type: 302,
    },
    GeneralizedMarkerPattern {
        name: "ssh-ec-private-key",
        start_marker: "-----BEGIN EC PRIVATE KEY-----",
        end_marker: "-----END EC PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 4096,
        contains_keyword: Some("EC PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 303,
    },
    GeneralizedMarkerPattern {
        name: "x509-certificate",
        start_marker: "-----BEGIN CERTIFICATE-----",
        end_marker: "-----END CERTIFICATE-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 10240,
        contains_keyword: Some("CERTIFICATE"),
        exclude_keyword: Some("REQUEST"), // Skip CSRs (covered separately)
        min_body_len: 200,
        pattern_type: 304,
    },
    GeneralizedMarkerPattern {
        name: "x509-certificate-request",
        start_marker: "-----BEGIN CERTIFICATE REQUEST-----",
        end_marker: "-----END CERTIFICATE REQUEST-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 5120,
        contains_keyword: Some("CERTIFICATE REQUEST"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 305,
    },
    GeneralizedMarkerPattern {
        name: "encrypted-private-key",
        start_marker: "-----BEGIN ENCRYPTED PRIVATE KEY-----",
        end_marker: "-----END ENCRYPTED PRIVATE KEY-----",
        tier: PatternTier::Critical,
        max_lookahead: 5120,
        contains_keyword: Some("ENCRYPTED PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 306,
    },
    GeneralizedMarkerPattern {
        name: "public-key",
        start_marker: "-----BEGIN PUBLIC KEY-----",
        end_marker: "-----END PUBLIC KEY-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 3072,
        contains_keyword: Some("PUBLIC KEY"),
        exclude_keyword: None,
        min_body_len: 100,
        pattern_type: 307,
    },
    GeneralizedMarkerPattern {
        name: "pgp-private-key-block",
        start_marker: "-----BEGIN PGP PRIVATE KEY BLOCK-----",
        end_marker: "-----END PGP PRIVATE KEY BLOCK-----",
        tier: PatternTier::Critical,
        max_lookahead: 20480,
        contains_keyword: Some("PRIVATE KEY"),
        exclude_keyword: None,
        min_body_len: 500,
        pattern_type: 308,
    },
    GeneralizedMarkerPattern {
        name: "pgp-public-key-block",
        start_marker: "-----BEGIN PGP PUBLIC KEY BLOCK-----",
        end_marker: "-----END PGP PUBLIC KEY BLOCK-----",
        tier: PatternTier::Infrastructure,
        max_lookahead: 6144,
        contains_keyword: Some("PUBLIC KEY"),
        exclude_keyword: None,
        min_body_len: 200,
        pattern_type: 309,
    },
    GeneralizedMarkerPattern {
        name: "pgp-message",
        start_marker: "-----BEGIN PGP MESSAGE-----",
        end_marker: "-----END PGP MESSAGE-----",
        tier: PatternTier::Critical,
        max_lookahead: 15360,
        contains_keyword: None,
        exclude_keyword: None,
        min_body_len: 50,
        pattern_type: 310,
    },
];

// ============================================================================
// REGEX PATTERNS (203 total) - NOT IMPLEMENTED YET
// Complex patterns: anchors, character classes, repetition
// For MVP, we'll implement a subset or skip entirely
// ============================================================================

pub const REGEX_PATTERN_COUNT: usize = 18; // Implemented in regex_patterns.rs

// ============================================================================
// SUMMARY & COUNTS
// ============================================================================

pub const SIMPLE_PREFIX_COUNT: usize = 23; // Removed 4 overly broad patterns (generic-password, etc.)
pub const PREFIX_VALIDATION_COUNT: usize = 349; // Removed 12 database/service URI patterns to eliminate false positives
pub const JWT_COUNT: usize = 1;
pub const MULTILINE_MARKER_COUNT: usize = 11; // SSH keys + certificate + PGP patterns (Phase 4a-4c)

pub const TOTAL_PATTERNS: usize = SIMPLE_PREFIX_COUNT + PREFIX_VALIDATION_COUNT + JWT_COUNT + MULTILINE_MARKER_COUNT + REGEX_PATTERN_COUNT;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_counts() {
        assert_eq!(SIMPLE_PREFIX_PATTERNS.len(), SIMPLE_PREFIX_COUNT);
        assert_eq!(PREFIX_VALIDATION_PATTERNS.len(), PREFIX_VALIDATION_COUNT);
        assert_eq!(JWT_PATTERNS.len(), JWT_COUNT);
    }

    #[test]
    fn test_aws_patterns_exist() {
        let aws_prefixes: Vec<_> = SIMPLE_PREFIX_PATTERNS
            .iter()
            .filter(|p| p.name.starts_with("aws-"))
            .collect();
        assert_eq!(aws_prefixes.len(), 4, "Should have 4 AWS patterns (AKIA, ASIA, ABIA, ACCA)");
    }

    #[test]
    fn test_github_patterns_exist() {
        let gh_prefixes: Vec<_> = SIMPLE_PREFIX_PATTERNS
            .iter()
            .filter(|p| p.name.starts_with("github-"))
            .collect();
        assert_eq!(gh_prefixes.len(), 3, "Should have 3 GitHub patterns");
    }

    #[test]
    fn test_critical_patterns() {
        let critical: Vec<_> = SIMPLE_PREFIX_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Critical)
            .collect();
        assert!(critical.len() >= 10, "Should have at least 10 critical patterns");
    }
}
