pub mod mitm {
    pub mod config;
    pub mod connection_pool;
    pub mod h2_e2e_tests;
    /// HTTP/2 MITM Handler - Handles H2 protocol with detect-only and redact modes
    /// Integrated with h2 crate for multiplexing and per-stream redaction support
    pub mod h2_mitm_handler;
    pub mod h2_upstream_forwarder;
    pub mod http_handler;
    pub mod proxy;
    pub mod tls;
    pub mod tls_acceptor;
    pub mod tls_mitm;
    pub mod upstream_connector;
}

// Policy integration
mod policy_integration;
pub use policy_integration::init_policy;
pub use policy_integration::init_policy as init_policy_from_config;

// Re-export TLS roots helper from scred-http
pub use scred_http::tls_roots::build_root_cert_store;

pub use mitm::config::Config;
pub use mitm::connection_pool::{ConnectionPool, PooledConnectionGuard, PoolStats};
pub use scred_config::ConnectionPoolConfig;

/// HTTP/2 MITM support: Full implementation with per-stream redaction
/// - ALPN negotiation: Automatic protocol selection (h2 vs http/1.1)
/// - Redaction modes: Detect-Only and Redact both fully supported
/// - Pattern selector: Flexible per-request pattern filtering
pub use mitm::h2_mitm_handler::{H2MitmConfig, H2MitmHandler};
pub use mitm::proxy::ProxyServer;
