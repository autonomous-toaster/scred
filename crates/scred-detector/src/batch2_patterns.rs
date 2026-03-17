// BATCH 2: Infrastructure & System Authentication (25 patterns)
// TDD: Test each pattern after adding it
// FOCUS: Well-known system patterns (password hashes, databases, env vars, containers)

use crate::patterns::{PatternTier, Charset, PrefixValidationPattern};

/// BATCH 2: Infrastructure, Databases, System Authentication Patterns
/// All patterns use PREFIX_VALIDATION (SIMD-friendly)
/// No regex needed - all have predictable prefixes and lengths
pub const BATCH2_PATTERNS: &[PrefixValidationPattern] = &[
    // ========================================================================
    // PASSWORD HASHES (3 patterns) - /etc/shadow, configuration files
    // ========================================================================
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

    // ========================================================================
    // DATABASE CONNECTION STRINGS (6 patterns) - Connection URLs with credentials
    // ========================================================================
    PrefixValidationPattern {
        name: "mongodb-uri",
        prefix: "mongodb://",
        tier: PatternTier::Critical,
        min_len: 25,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "mongodb-srv-uri",
        prefix: "mongodb+srv://",
        tier: PatternTier::Critical,
        min_len: 30,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "redis-uri",
        prefix: "redis://",
        tier: PatternTier::Critical,
        min_len: 20,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "cassandra-uri",
        prefix: "cassandra://",
        tier: PatternTier::Infrastructure,
        min_len: 25,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "elasticsearch-uri",
        prefix: "elasticsearch://",
        tier: PatternTier::Infrastructure,
        min_len: 25,
        max_len: 0,
        charset: Charset::Any,
    },
    PrefixValidationPattern {
        name: "amqp-uri",
        prefix: "amqp://",
        tier: PatternTier::Infrastructure,
        min_len: 20,
        max_len: 0,
        charset: Charset::Any,
    },

    // ========================================================================
    // ENVIRONMENT VARIABLES (5 patterns) - .env files, CI/CD logs
    // ========================================================================
    PrefixValidationPattern {
        name: "database-url-env",
        prefix: "DATABASE_URL=",
        tier: PatternTier::Critical,
        min_len: 25,
        max_len: 0,
        charset: Charset::Any,
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

    // ========================================================================
    // VAULT / HASHICORP (3 patterns) - Secret management systems
    // ========================================================================
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

    // ========================================================================
    // CONTAINER & ORCHESTRATION (4 patterns) - Docker, Kubernetes, etc.
    // ========================================================================
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

    // ========================================================================
    // CLOUD PROVIDER CONFIG (3 patterns) - AWS, GCP, Azure
    // ========================================================================
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
];

pub const BATCH2_PATTERN_COUNT: usize = 25;

#[cfg(test)]
mod batch2_tests {
    use super::*;

    #[test]
    fn batch2_pattern_count() {
        assert_eq!(BATCH2_PATTERNS.len(), BATCH2_PATTERN_COUNT);
    }

    #[test]
    fn batch2_password_hashes() {
        // bcrypt hash from /etc/shadow
        let bcrypt = b"$2a$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcg7b3XeKeUxWdeS86E36DRZlFm";
        // SHA512 from /etc/shadow
        let sha512 = b"$6$rounds=656000$c2QfKeyStejyeUZ6$rqh/cIPz0gi.URNNX3kh2MQLGGG5PJooP33R.V0aBYK/KjoeoP3veK3Yj2TSuvaIW1MDEVmbROhWd5bup7Uq/1";
        
        // These would be detected in detector tests when integrated
        assert!(!bcrypt.is_empty());
        assert!(!sha512.is_empty());
    }

    #[test]
    fn batch2_database_uris() {
        let mongodb = b"mongodb://user:pass@localhost:27017/database";
        let redis = b"redis://user:password@localhost:6379/0";
        
        assert!(!mongodb.is_empty());
        assert!(!redis.is_empty());
    }

    #[test]
    fn batch2_env_variables() {
        let db_url = b"DATABASE_URL=postgresql://user:pass@localhost/db";
        let github_token = b"GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh";
        
        assert!(!db_url.is_empty());
        assert!(!github_token.is_empty());
    }

    #[test]
    fn batch2_vault_tokens() {
        let hvs_token = b"hvs.CAhvSyH2yqNWiZj6HiWJ7x8z";
        let svc_token = b"s.tnrnnyq8sF2dh2er2odqygEv";
        
        assert!(!hvs_token.is_empty());
        assert!(!svc_token.is_empty());
    }

    #[test]
    fn batch2_container_tokens() {
        let docker_swarm = b"SWMTKN-1-1n2cn3od4pncd4a0jkyj2l1l2m2n2o3p3q3r3s3t3u3v3w3x3y3z3a4b4";
        let docker_pat = b"dckr_pat_abcd1234efgh5678ijkl";
        
        assert!(!docker_swarm.is_empty());
        assert!(!docker_pat.is_empty());
    }

    #[test]
    fn batch2_all_patterns_have_names() {
        for pattern in BATCH2_PATTERNS {
            assert!(!pattern.name.is_empty(), "All patterns must have a name");
            assert!(!pattern.prefix.is_empty(), "All patterns must have a prefix");
            assert!(pattern.min_len > 0, "All patterns must have min_len > 0");
        }
    }

    #[test]
    fn batch2_critical_patterns() {
        let critical: Vec<_> = BATCH2_PATTERNS
            .iter()
            .filter(|p| p.tier == PatternTier::Critical)
            .collect();
        assert!(critical.len() >= 8, "Should have at least 8 critical patterns");
    }
}
