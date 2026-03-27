pub mod mitm {
    pub mod config;
    pub mod http_handler;
    pub mod proxy;
    pub mod tls;
    pub mod tls_acceptor;
    pub mod tls_mitm;
    pub mod h2_upstream_forwarder;
    pub mod h2_e2e_tests;
    /// HTTP/2 MITM Handler - Handles H2 protocol with detect-only and redact modes
    /// Integrated with h2 crate for multiplexing and per-stream redaction support
    pub mod h2_mitm_handler;
}

pub use mitm::proxy::ProxyServer;
pub use mitm::config::Config;
/// HTTP/2 MITM support: Full implementation with per-stream redaction
/// - ALPN negotiation: Automatic protocol selection (h2 vs http/1.1)
/// - Redaction modes: Detect-Only and Redact both fully supported
/// - Pattern selector: Flexible per-request pattern filtering
pub use mitm::h2_mitm_handler::{H2MitmHandler, H2MitmConfig};
