//! SCRED HTTP Library
//!
//! Comprehensive HTTP utilities for SCRED proxies and CLI tools.
//!
//! ## HTTP Core
//! - `parser`: HTTP/1.1 request/response parsing
//! - `models`: HttpRequest, HttpResponse data structures
//! - `connect`: HTTP CONNECT tunneling for HTTPS proxies
//!
//! ## Redaction
//! - `configurable_engine`: Pattern detection and selective redaction
//! - `streaming_request`: Stream request bodies through redactor
//! - `streaming_response`: Stream response bodies through redactor
//! - `chunked_parser`: Parse HTTP chunked transfer-encoding
//!
//! ## Proxy Utilities
//! - `duplex`: Combined AsyncRead + AsyncWrite socket wrapper
//! - `host_identification`: Extract hostnames from HTTP/TLS
//! - `proxy_resolver`: Detect system proxy settings (HTTP_PROXY env vars)
//! - `tcp_relay`: Generic TCP relay with bidirectional redaction
//!
//! ## Configuration & Secrets
//! - `config`: Redaction configuration and pattern selection
//! - `secrets`: Secret filtering rules and configuration
//!
//! ## Logging
//! - `logging`: Structured logging (JSON, compact, pretty)

pub mod config;
pub mod configurable_engine;
pub mod connect;
pub mod connection_pool;
pub mod dns_cache;
pub mod dns_resolver;
pub mod cached_dns_resolver;
pub mod optimized_dns_resolver;
pub mod pooled_dns_resolver;
pub mod duplex;
pub mod fixed_upstream;
pub mod h2;
pub mod header_rewriter;
pub mod host_identification;
pub mod http_line_reader;
pub mod http_proxy_handler;
pub mod location_rewriter;
pub mod logging;
pub mod models;
pub mod parser;
pub mod pattern_metadata;
pub mod proxy_resolver;
pub mod response_reader;
pub mod secrets;
pub mod tcp_relay;
pub mod http_headers;
pub mod streaming_request;
pub mod streaming_response;
pub mod chunked_parser;
pub mod upstream_h2_client;
pub mod env_detection;

// Re-export pattern selector from scred_redactor (single source of truth)
pub use scred_redactor::{PatternSelector, CompositePatternSelector, PatternFilter, PatternTier};
pub use scred_redactor::pattern_selector::{Severity, ServiceCategory, PatternKind, Origin};
pub use pattern_metadata::get_pattern_tier;
pub use configurable_engine::{ConfigurableEngine, FilteredRedactionResult};
pub use connection_pool::ConnectionPool;
pub use dns_cache::DnsCache;
pub use cached_dns_resolver::{CachedDnsResolver, CachedDnsConfig};
pub use optimized_dns_resolver::{OptimizedDnsResolver, OptimizedDnsResolverBuilder};
pub use pooled_dns_resolver::{PooledDnsResolver, PoolConfig, PooledTcpStream};

pub const VERSION: &str = "0.1.0";
