//! SCRED HTTP Library
//!
//! Comprehensive HTTP utilities for SCRED proxies and CLI tools.
//!
//! ## HTTP Core
//! - `parser`: HTTP/1.1 request/response parsing
//! - `models`: HttpRequest, HttpResponse data structures
//! - `connect`: HTTP CONNECT tunneling for HTTPS proxies
//!
//! ## Redaction (feature: "redaction")
//! - `configurable_engine`: Pattern detection and selective redaction
//! - `streaming_request`: Stream request bodies through redactor
//! - `streaming_response`: Stream response bodies through redactor
//! - `chunked_parser`: Parse HTTP chunked transfer-encoding
//!
//! ## Policy (feature: "policy")
//! - `streaming_policy`: Placeholder replacement for proxy mode
//!
//! ## Proxy Utilities
//! - `duplex`: Combined AsyncRead + AsyncWrite socket wrapper
//! - `host_identification`: Extract hostnames from HTTP/TLS
//! - `proxy_resolver`: Detect system proxy settings
//! - `tcp_relay`: Generic TCP relay
//!
//! ## Configuration
//! - `config`: Configuration types
//! - `secrets`: Secret filtering rules
//!
//! ## Logging
//! - `logging`: Structured logging (JSON, compact, pretty)

pub mod cached_dns_resolver;
pub mod config;
pub mod connect;
pub mod connection_pool;
pub mod dns_cache;
pub mod dns_resolver;
pub mod duplex;
pub mod env_detection;
pub mod fixed_upstream;
pub mod h2;
pub mod header_rewriter;
pub mod host_identification;
pub mod http_headers;
pub mod http_line_reader;
pub mod http_proxy_handler;
pub mod location_rewriter;
pub mod logging;
pub mod models;
pub mod multi_upstream_pool;
pub mod optimized_dns_resolver;
pub mod parser;
pub mod pattern_metadata;
pub mod pooled_dns_resolver;
pub mod proxy_resolver;
pub mod response_reader;
pub mod secrets;
pub mod tcp_relay;
pub mod tls_roots;
pub mod upstream_connection;
pub mod upstream_h2_client;

#[cfg(feature = "redaction")]
pub mod configurable_engine;

#[cfg(feature = "redaction")]
pub mod streaming_request;

#[cfg(feature = "redaction")]
pub mod streaming_response;

#[cfg(feature = "redaction")]
pub mod chunked_parser;

#[cfg(feature = "policy")]
pub mod streaming_policy;
#[cfg(feature = "policy")]
pub mod policy_handler;

// Core exports (always available)
pub use cached_dns_resolver::{CachedDnsConfig, CachedDnsResolver};
pub use connection_pool::ConnectionPool;
pub use dns_cache::DnsCache;
pub use multi_upstream_pool::MultiUpstreamPool;
pub use optimized_dns_resolver::{OptimizedDnsResolver, OptimizedDnsResolverBuilder};
pub use pooled_dns_resolver::{PoolConfig, PooledDnsResolver, PooledTcpStream};

// Redaction exports (feature-gated)
#[cfg(feature = "redaction")]
pub use configurable_engine::{ConfigurableEngine, FilteredRedactionResult};
#[cfg(feature = "redaction")]
pub use pattern_metadata::get_pattern_tier;
#[cfg(feature = "redaction")]
pub use scred_redactor::pattern_selector::{Origin, PatternKind, ServiceCategory, Severity};
#[cfg(feature = "redaction")]
pub use scred_redactor::{CompositePatternSelector, PatternFilter, PatternSelector, PatternTier};

#[cfg(feature = "policy")]
pub use streaming_policy::{
    stream_request_with_policy, stream_response_with_policy, StreamingPolicyConfig,
};
#[cfg(feature = "policy")]
pub use policy_handler::{handle_http_with_policy, PolicyResult};

pub const VERSION: &str = "0.1.0";
