//! P1 Infrastructure Secrets Pattern Tests
//!
//! Tests for 8 infrastructure & build system patterns:
//! 1. docker-dockercfg-auth - Docker registry authentication in .dockercfg
//! 2. aws-ecr-token - AWS ECR authentication tokens (base64)
//! 3. rabbitmq-amqp-connection - RabbitMQ AMQP connection strings
//! 4. kafka-sasl-credentials - Kafka SASL authentication
//! 5. amqp-connection-string - Generic AMQP URLs
//! 6. maven-password - Maven settings.xml password elements
//! 7. npm-auth-token - npm .npmrc authentication tokens
//! 8. gradle-api-key - Gradle build cache API keys
//!
//! Total: 40 test cases (5 tests per pattern)

use regex::Regex;

// ============================================================================
// P1-1: docker-dockercfg-auth Tests (5 tests)
// ============================================================================

#[test]
fn p1_docker_dockercfg_auth_valid() {
    let input = r#""auth": "dXNlcm5hbWU6cGFzc3dvcmRzYXZlZGluYmFzZTY0Zm9ybWF0Zm9yZG9ja2Vychg=""#;
    let re = Regex::new(r#""auth"\s*:\s*"([a-zA-Z0-9+/]{20,}={0,2})""#).unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_docker_dockercfg_full_registry() {
    let input = r#"
{
  "auths": {
    "docker.io": {
      "username": "user",
      "password": "pass",
      "email": "user@example.com",
      "auth": "dXNlcjpwYXNzc2F2ZWRpbmJhc2U2NGZvcm1hdGZvcmRvY2tlcnJlZ2lzdHJ5"
    }
  }
}"#;
    let re = Regex::new(r#""auth"\s*:\s*"([a-zA-Z0-9+/]{20,}={0,2})""#).unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_docker_dockercfg_gcr_format() {
    // GCR (Google Container Registry) format
    let input = r#""auth": "X2pzb25fa2V5OiB7ImNsaWVudF9pZCIsIAkJImNsaWVudF9zZWNyZXQiLCAieW91clt0cnVlIiwgInR5cGVfdm""#;
    let re = Regex::new(r#""auth"\s*:\s*"([a-zA-Z0-9+/]{20,}={0,2})""#).unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_docker_dockercfg_short_token() {
    // Valid but short token (still >= 20 chars)
    let input = r#""auth": "dXNlcjpwYXNzd2hhdGV2ZXI=""#;
    let re = Regex::new(r#""auth"\s*:\s*"([a-zA-Z0-9+/]{20,}={0,2})""#).unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_docker_dockercfg_invalid_too_short() {
    // Token too short
    let input = r#""auth": "dXNlcjpwYXNz""#;
    let re = Regex::new(r#""auth"\s*:\s*"([a-zA-Z0-9+/]{20,}={0,2})""#).unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P1-2: aws-ecr-token Tests (5 tests)
// ============================================================================

#[test]
fn p1_aws_ecr_token_valid() {
    let input = "AQEtzJvK7e9D8sL3mN4p2Q5r6S7t8U9v0W1x2Y3z4A5b6C7d8E9f0G1h2I3j4K5l6M7n8O9p0Q1r2S3t4U5v6W7x8Y9z0A1b2C12345";
    let re = Regex::new(r"\bAQE[a-zA-Z0-9+/]{100,}={0,2}\b").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_aws_ecr_token_from_get_authorization_token() {
    let input = "AQEtv9j8Rn3mL5kP2sQ9wX4yZ1aB6cD7eF0gH3iJ2kL5mN8oP1qR4sT7uV0wX3yZ6aB9cD2eF5gH8iJ1kL4mN7oP0qR3sT6uV9wX2yZ5aB8c";
    let re = Regex::new(r"\bAQE[a-zA-Z0-9+/]{100,}={0,2}\b").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_aws_ecr_token_in_docker_login() {
    let input = r#"
    "authorizationData": [
      {
        "authorizationToken": "AQEt7V4m2L9pK3sR6wX1yZ5aB8cD2eF5gH8iJ1kL4mN7oP0qR3sT6uV9wX2yZ5aB8cD2eF5gH8iJ1kL4mN7oP0qR3sT6uV9wX2yZ5a1234",
        "proxyEndpoint": "https://999999999.dkr.ecr.us-east-1.amazonaws.com"
      }
    ]
    "#;
    let re = Regex::new(r"\bAQE[a-zA-Z0-9+/]{100,}={0,2}\b").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_aws_ecr_token_with_padding() {
    let input = "AQEtvKr3mL9pK2sQ6wX1yZ4aB7cD0eF3gH6iJ9kL2mN5oP8qR1sT4uV7wX0yZ3aB6cD9eF2gH5iJ8kL1mN4oP7qR0sT3uV6wX9yZ2aB5c==";
    let re = Regex::new(r"\bAQE[a-zA-Z0-9+/]{100,}={0,2}\b").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_aws_ecr_token_too_short() {
    let input = "AQEtvKr3mL9pK2sQ6wX1yZ4aB7cD0eF3gH6iJ9kL2";
    let re = Regex::new(r"\bAQE[a-zA-Z0-9+/]{100,}={0,2}\b").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P1-3: rabbitmq-amqp-connection Tests (5 tests)
// ============================================================================

#[test]
fn p1_rabbitmq_amqp_connection_basic() {
    let input = "amqp://guest:password123@rabbitmq.example.com:5672/";
    let re = Regex::new(r"amqps?://[a-zA-Z0-9._-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_rabbitmq_amqp_connection_secure() {
    let input = "amqps://admin:SecureP@ssw0rd!@rabbitmq-prod.internal:5671/my-vhost";
    let re = Regex::new(r"amqps?://[a-zA-Z0-9._-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_rabbitmq_amqp_connection_complex_password() {
    let input = "amqp://user:P@ss$w0rd!#%^&@rabbitmq-cluster:5672/main";
    let re = Regex::new(r"amqps?://[a-zA-Z0-9._-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_rabbitmq_amqp_connection_no_port() {
    let input = "amqp://testuser:testpass@rabbitmq.example.com/vhost";
    let re = Regex::new(r"amqps?://[a-zA-Z0-9._-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_rabbitmq_amqp_connection_invalid_no_password() {
    let input = "amqp://testuser@rabbitmq.example.com:5672/";
    let re = Regex::new(r"amqps?://[a-zA-Z0-9._-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P1-4: kafka-sasl-credentials Tests (5 tests)
// ============================================================================

#[test]
fn p1_kafka_sasl_scram_sha256() {
    let input = "SCRAM-SHA-256 username:password_value_here";
    let re = Regex::new(r"SCRAM-SHA-(?:256|512)\s+[a-zA-Z0-9._-]+:[^\s]+").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_kafka_sasl_scram_sha512() {
    let input = "sasl.mechanism=SCRAM-SHA-512 user_test:p@ssw0rd!";
    let re = Regex::new(r"SCRAM-SHA-(?:256|512)\s+[a-zA-Z0-9._-]+:[^\s]+").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_kafka_sasl_in_config_file() {
    let input = "security.protocol=SASL_SSL\nSCRAM-SHA-256 kafka_user:MySecurePassword123";
    let re = Regex::new(r"SCRAM-SHA-(?:256|512)\s+[a-zA-Z0-9._-]+:[^\s]+").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_kafka_sasl_complex_password() {
    let input = "SCRAM-SHA-256 admin:P@ss$w0rd!#%^&*()_+-=[]{}|;:,.<>?";
    let re = Regex::new(r"SCRAM-SHA-(?:256|512)\s+[a-zA-Z0-9._-]+:[^\s]+").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_kafka_sasl_missing_password() {
    let input = "SCRAM-SHA-256 kafka_user:";
    let re = Regex::new(r"SCRAM-SHA-(?:256|512)\s+[a-zA-Z0-9._-]+:[^\s]+").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P1-5: amqp-connection-string Tests (5 tests)
// ============================================================================

#[test]
fn p1_amqp_connection_string_basic() {
    let input = "amqp://guest:guest@localhost:5672/";
    let re = Regex::new(r"amqp://[a-zA-Z0-9:._%-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_amqp_connection_string_remote_host() {
    let input = "amqp://app_user:SecurePass@amqp.prod.internal:5672/app_vhost";
    let re = Regex::new(r"amqp://[a-zA-Z0-9:._%-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_amqp_connection_string_with_encoding() {
    let input = "amqp://user%40email.com:p%40ssword@rabbitmq:5672/";
    let re = Regex::new(r"amqp://[a-zA-Z0-9:._%-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_amqp_connection_string_default_port() {
    let input = "amqp://testuser:testpass@broker.example.com/";
    let re = Regex::new(r"amqp://[a-zA-Z0-9:._%-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_amqp_connection_string_invalid_no_auth() {
    let input = "amqp://localhost:5672/";
    let re = Regex::new(r"amqp://[a-zA-Z0-9:._%-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P1-6: maven-password Tests (5 tests)
// ============================================================================

#[test]
fn p1_maven_password_simple() {
    let input = "<password>maven_secure_password_here</password>";
    let re = Regex::new(r"<password>([^<]{8,})</password>").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_maven_password_in_settings_xml() {
    let input = r#"
  <server>
    <id>repo.example.com</id>
    <username>maven_user</username>
    <password>SecureP@ssw0rd123!$%^</password>
  </server>"#;
    let re = Regex::new(r"<password>([^<]{8,})</password>").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_maven_password_with_special_chars() {
    // Note: Avoid < and > in password as they terminate the XML tag in regex
    let input = "<password>P@ss$w0rd!#%^&*()_+-=[]{}|;:',./~</password>";
    let re = Regex::new(r"<password>([^<]{8,})</password>").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_maven_password_base64() {
    let input = "<password>YWRtaW46UGFzc3dvcmQxMjMhQCMkJSZ</password>";
    let re = Regex::new(r"<password>([^<]{8,})</password>").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_maven_password_too_short() {
    let input = "<password>short</password>";
    let re = Regex::new(r"<password>([^<]{8,})</password>").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P1-7: npm-auth-token Tests (5 tests)
// ============================================================================

#[test]
fn p1_npm_auth_token_valid() {
    // NPM tokens are typically 36 characters
    let input = "abcdef1234567890ghijklmnopqrstuvwxyz";
    let re = Regex::new(r"\b([a-zA-Z0-9-_]{36})\b").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_npm_auth_token_in_npmrc() {
    let input = "//registry.npmjs.org/:_authToken=abcdef1234567890ghijklmnopqrstuvwxyz";
    let re = Regex::new(r"\b([a-zA-Z0-9-_]{36})\b").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_npm_auth_token_in_file() {
    let input = "registry=https://registry.npmjs.org/\n//registry.npmjs.org/:_authToken=abcdef1234567890ghijklmnopqrstuvwxyz";
    let re = Regex::new(r"\b([a-zA-Z0-9-_]{36})\b").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_npm_auth_token_with_underscore() {
    let input = "abcde_ghij_klmn_opqr_stuv_wxyz_12345";
    let re = Regex::new(r"\b([a-zA-Z0-9-_]{36})\b").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_npm_auth_token_too_short() {
    let input = "npm_short_token_here";
    let re = Regex::new(r"\b([a-zA-Z0-9-_]{36})\b").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P1-8: gradle-api-key Tests (5 tests)
// ============================================================================

#[test]
fn p1_gradle_api_key_build_cache() {
    let input = "org.gradle.caching.http.authentication = gradle_cache_api_key_1234567890";
    let re = Regex::new(r"org\.gradle\.caching\.http\.authentication.*=\s*([a-zA-Z0-9_-]{20,})").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_gradle_api_key_in_properties_file() {
    let input = "org.gradle.caching.http.authentication=gradle_secure_token_for_build_cache_management";
    let re = Regex::new(r"org\.gradle\.caching\.http\.authentication.*=\s*([a-zA-Z0-9_-]{20,})").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_gradle_api_key_with_spaces() {
    let input = "org.gradle.caching.http.authentication = gradle_buildcache_key_abc123def456ghi789";
    let re = Regex::new(r"org\.gradle\.caching\.http\.authentication.*=\s*([a-zA-Z0-9_-]{20,})").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_gradle_api_key_multiline_properties() {
    let input = "org.gradle.cache.cleanup=warn\norg.gradle.caching.http.authentication = gradle_token_api_key_production_secure";
    let re = Regex::new(r"org\.gradle\.caching\.http\.authentication.*=\s*([a-zA-Z0-9_-]{20,})").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p1_gradle_api_key_too_short() {
    let input = "org.gradle.caching.http.authentication = short_key";
    let re = Regex::new(r"org\.gradle\.caching\.http\.authentication.*=\s*([a-zA-Z0-9_-]{20,})").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// Cross-Pattern Integration Tests (2 tests)
// ============================================================================

#[test]
fn p1_all_patterns_in_infrastructure_config() {
    let input = r#"
# Docker config
{
  "auths": {
    "docker.io": {
      "auth": "dXNlcm5hbWU6UGFzc3dvcmQxMjM0NTY3ODkwYWJjZGVmZ2hpams="
    }
  }
}

# AWS ECR
aws ecr get-authorization-token --query 'authorizationData[0].authorizationToken'
# AQEtv9j8Rn3mL5kP2sQ9wX4yZ1aB6cD7eF0gH3iJ2kL5mN8oP1qR4sT7uV0wX3yZ6aB9cD2eF5gH8iJ1kL4mN7oP0qR3sT6uV9wX2yZ5aB8cD2eF5gH8iJ1kL4mN7oP0qR3sT6uV9

# RabbitMQ
RABBITMQ_URL=amqp://admin:SecurePass@rabbitmq.internal:5672/

# Kafka
KAFKA_SASL_USER=SCRAM-SHA-256 kafka_prod:ProducerPassword123!

# Maven
<server>
  <id>nexus</id>
  <password>MavenNexusPassword1234567890</password>
</server>

# npm
//registry.npmjs.org/:_authToken=abcdefghij1234567890klmnopqrst123456

# Gradle
org.gradle.caching.http.authentication = gradle_build_cache_api_token_production_secure
    "#;
    
    let docker_re = Regex::new(r#""auth"\s*:\s*"([a-zA-Z0-9+/]{20,}={0,2})""#).unwrap();
    let ecr_re = Regex::new(r"\bAQE[a-zA-Z0-9+/]{100,}={0,2}\b").unwrap();
    let rabbitmq_re = Regex::new(r"amqps?://[a-zA-Z0-9._-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    let kafka_re = Regex::new(r"SCRAM-SHA-(?:256|512)\s+[a-zA-Z0-9._-]+:[^\s]+").unwrap();
    let maven_re = Regex::new(r"<password>([^<]{8,})</password>").unwrap();
    let npm_re = Regex::new(r"\b([a-zA-Z0-9-_]{36})\b").unwrap();
    let gradle_re = Regex::new(r"org\.gradle\.caching\.http\.authentication.*=\s*([a-zA-Z0-9_-]{20,})").unwrap();
    
    assert!(docker_re.is_match(input));
    assert!(ecr_re.is_match(input));
    assert!(rabbitmq_re.is_match(input));
    assert!(kafka_re.is_match(input));
    assert!(maven_re.is_match(input));
    assert!(npm_re.is_match(input));
    assert!(gradle_re.is_match(input));
}

#[test]
fn p1_all_patterns_in_deployment_script() {
    let input = r#"#!/bin/bash
# Deployment automation script

# Docker registry setup
"auth": "dXNlcjpwYXNzd29yZF9iYXNlNjRfZW5jb2RlZF9mb3JtYXQ="

# ECR authentication
export ECR_TOKEN=AQEt4mK2pL9sQ6wX1yZ5aB8cD2eF5gH8iJ1kL4mN7oP0qR3sT6uV9wX2yZ5aB8cD2eF5gH8iJ1kL4mN7oP0qR3sT6uV9wX2yZ5aB8cD2eF5

# Message queue connections
RABBITMQ_URI="amqps://msgqueue_user:Msg@Queue#Pass123@rabbitmq-prod.us-east-1:5671/"
KAFKA_AUTH="SCRAM-SHA-512 kafka_consumer:ConsumerSecret!@#$%^&*()"

# Build configuration
build.xml contains: <password>BuildServerCredentials2024</password>
npm_token=abcdefghij1234567890klmnopqrst123456
gradle_cache_key="org.gradle.caching.http.authentication = gradle_prod_cache_api_key_secure"
    "#;
    
    let docker_re = Regex::new(r#""auth"\s*:\s*"([a-zA-Z0-9+/]{20,}={0,2})""#).unwrap();
    let ecr_re = Regex::new(r"\bAQE[a-zA-Z0-9+/]{100,}={0,2}\b").unwrap();
    let rabbitmq_re = Regex::new(r"amqps?://[a-zA-Z0-9._-]+:[^@\s]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    let kafka_re = Regex::new(r"SCRAM-SHA-(?:256|512)\s+[a-zA-Z0-9._-]+:[^\s]+").unwrap();
    let maven_re = Regex::new(r"<password>([^<]{8,})</password>").unwrap();
    let npm_re = Regex::new(r"\b([a-zA-Z0-9-_]{36})\b").unwrap();
    let gradle_re = Regex::new(r"org\.gradle\.caching\.http\.authentication.*=\s*([a-zA-Z0-9_-]{20,})").unwrap();
    
    assert!(docker_re.is_match(input));
    assert!(ecr_re.is_match(input));
    assert!(rabbitmq_re.is_match(input));
    assert!(kafka_re.is_match(input));
    assert!(maven_re.is_match(input));
    assert!(npm_re.is_match(input));
    assert!(gradle_re.is_match(input));
}
