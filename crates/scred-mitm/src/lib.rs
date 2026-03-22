pub mod mitm {
    pub mod config;
    pub mod http_handler;
    pub mod proxy;
    pub mod tls;
    pub mod tls_acceptor;
    pub mod tls_mitm;
    pub mod h2_mitm;
    pub mod h2_upstream_forwarder;
    pub mod h2_e2e_tests;
    pub mod h2_upstream_integration;
}

pub use mitm::proxy::ProxyServer;
pub use mitm::config::Config;
pub use mitm::h2_mitm::{H2Multiplexer, H2MultiplexerConfig};
