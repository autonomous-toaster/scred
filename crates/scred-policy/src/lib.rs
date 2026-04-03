//! scred-policy: Policy-based Secret Value Replacement
//!
//! This crate provides placeholder-based secret injection for HTTP proxies.
//! Secrets are never exposed directly - instead, deterministic placeholders
//! are generated and exchanged for real values at proxy runtime.
//!
//! # Features
//! - Environment variable provider (prefix, glob, explicit vars)
//! - SHA-256 based placeholder generation
//! - Secret validation (dangerous character detection)
//! - Value collision detection
//! - Discovery API for placeholder distribution
//! - Streaming-optimized replacement (Aho-Corasick, O(n) single-pass)

pub mod discovery;
pub mod engine;
pub mod placeholder;
pub mod provider;
pub mod streaming;
pub mod validation;

pub use discovery::{DiscoveryConfig, DiscoveryServer};
pub use engine::{
    BodyProcessingResult,
    DetectionEvent,
    Direction,
    HeaderProcessingResult,
    PolicyEngine,
};
pub use placeholder::{Placeholder, PlaceholderGenerator};
pub use provider::{EnvProvider, SecretProvider};
pub use streaming::{PlaceholderAutomaton, ReplacementTracker as StreamingTracker};
pub use validation::{validate_secret, OnInvalid, ValidationError};

// Re-export config types from scred-config
pub use scred_config::{
    BodyAction,
    HeaderAction,
    HostPolicy,
    PatternFilter,
    PolicyConfig,
    ProviderConfig,
    ResolvedPolicy,
};

/// Error type for policy operations
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("Secret validation failed: {0}")]
    Validation(#[from] ValidationError),

    #[error("Value collision detected: {0}")]
    ValueCollision(String),

    #[error("Secret not found: {0}")]
    NotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Domain not allowed for secret: {0}")]
    DomainNotAllowed(String),

    #[error("Pattern matching error: {0}")]
    PatternError(String),
}

impl From<aho_corasick::BuildError> for PolicyError {
    fn from(e: aho_corasick::BuildError) -> Self {
        PolicyError::PatternError(e.to_string())
    }
}

/// Result type for policy operations
pub type PolicyResult<T> = Result<T, PolicyError>;
