use super::{PatternTier, SimplePrefixPattern};

pub const SIMPLE_PREFIX_PATTERNS: &[SimplePrefixPattern] = &[
    SimplePrefixPattern {
        name: "artifactoryreferencetoken",
        prefix: "cmVmdGtu",
        tier: PatternTier::Infrastructure,
    },
    SimplePrefixPattern {
        name: "azure-storage",
        prefix: "AccountName",
        tier: PatternTier::Infrastructure,
    },
    SimplePrefixPattern {
        name: "azure-app-config",
        prefix: "Endpoint=https://",
        tier: PatternTier::Infrastructure,
    },
    SimplePrefixPattern {
        name: "coinbase",
        prefix: "organizations/",
        tier: PatternTier::Services,
    },
    SimplePrefixPattern {
        name: "context7-api-key",
        prefix: "ctx7sk_",
        tier: PatternTier::Critical,
    },
    SimplePrefixPattern {
        name: "context7-secret",
        prefix: "ctx7sk-",
        tier: PatternTier::Critical,
    },
    SimplePrefixPattern {
        name: "langsmith-deployment-key",
        prefix: "lsv2_sk_",
        tier: PatternTier::Critical,
    },
    SimplePrefixPattern {
        name: "pypi-upload-token",
        prefix: "pypi-AgEIcHlwaS5vcmc",
        tier: PatternTier::Services,
    },
    SimplePrefixPattern {
        name: "salad-cloud-api-key",
        prefix: "salad_cloud_",
        tier: PatternTier::Infrastructure,
    },
    SimplePrefixPattern {
        name: "sentry-access-token",
        prefix: "bsntrys_",
        tier: PatternTier::ApiKeys,
    },
    SimplePrefixPattern {
        name: "travisoauth",
        prefix: "travis_",
        tier: PatternTier::ApiKeys,
    },
    SimplePrefixPattern {
        name: "tumblr-api-key",
        prefix: "tumblr_",
        tier: PatternTier::Services,
    },
    SimplePrefixPattern {
        name: "upstash-redis",
        prefix: "redis_",
        tier: PatternTier::Infrastructure,
    },
    SimplePrefixPattern {
        name: "vercel-token",
        prefix: "vercel_",
        tier: PatternTier::ApiKeys,
    },
    // AWS patterns (all use simple prefix check)
    SimplePrefixPattern {
        name: "aws-akia",
        prefix: "AKIA",
        tier: PatternTier::Critical,
    },
    SimplePrefixPattern {
        name: "aws-asia",
        prefix: "ASIA",
        tier: PatternTier::Critical,
    },
    SimplePrefixPattern {
        name: "aws-abia",
        prefix: "ABIA",
        tier: PatternTier::Critical,
    },
    SimplePrefixPattern {
        name: "aws-acca",
        prefix: "ACCA",
        tier: PatternTier::Critical,
    },
    // GitHub patterns
    SimplePrefixPattern {
        name: "github-ghp",
        prefix: "ghp_",
        tier: PatternTier::Critical,
    },
    SimplePrefixPattern {
        name: "github-ghu",
        prefix: "ghu_",
        tier: PatternTier::Critical,
    },
    SimplePrefixPattern {
        name: "github-ghs",
        prefix: "ghs_",
        tier: PatternTier::Critical,
    },
    // OpenAI patterns
    SimplePrefixPattern {
        name: "openai-sk-proj",
        prefix: "sk-proj-",
        tier: PatternTier::Critical,
    },
    SimplePrefixPattern {
        name: "openai-sk",
        prefix: "sk-",
        tier: PatternTier::Critical,
    },
    // NOTE: Removed 4 overly broad generic patterns (generic-password, generic-password-colon,
    // generic-password-lower, generic-secret) to eliminate false positives.
    // Real secrets are caught by more specific patterns:
    // - Database passwords: MYSQL_PASSWORD=, POSTGRES_PASSWORD=, REDIS_PASSWORD=, etc.
    // - Secrets/tokens: Specific env patterns (_SECRET=, _TOKEN=, _API_KEY=)
    // - Generic "PASSWORD=" was matching demo values (PASSWORD=demo123)
];

// ============================================================================
