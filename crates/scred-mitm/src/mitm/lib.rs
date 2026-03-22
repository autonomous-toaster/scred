pub mod config;
pub mod mitm;
pub mod tls;
pub mod tls_acceptor;
pub mod tls_mitm;
pub mod host_identification;
pub mod logging;
pub mod connect;
pub mod upstream_resolver;
pub mod http_parser;

pub use config::{Config, matches_pattern, resolve_pattern_names};

// Import SCRED library for production-grade redaction (237+ secret patterns)
pub use crate::{RedactionEngine, RedactionConfig};
