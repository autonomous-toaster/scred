pub mod mitm {
    pub mod config;
    pub mod http_handler;
    pub mod proxy;
    pub mod tls;
    pub mod tls_acceptor;
    pub mod tls_mitm;
    // TODO: Replace with h2_mitm_handler (new h2 crate integration)
    // pub mod h2_mitm;
    pub mod h2_upstream_forwarder;
    pub mod h2_e2e_tests;
    // DEPRECATED: Modules removed - using h2_mitm_handler instead
    // pub mod h2_upstream_integration;
    // pub mod h2_handler;
    pub mod h2_mitm_handler;
}

pub use mitm::proxy::ProxyServer;
pub use mitm::config::Config;
// TODO: Export new h2_mitm_handler instead
// pub use mitm::h2_mitm::{H2Multiplexer, H2MultiplexerConfig};
pub use mitm::h2_mitm_handler::{H2MitmHandler, H2MitmConfig};
