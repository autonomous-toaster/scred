// SCRED MITM proxy modules
pub mod config;
pub mod connection_pool;
pub mod http_handler;
pub mod proxy;
pub mod tls;
pub mod tls_acceptor;
pub mod tls_mitm;
pub mod upstream_connector;
// TODO: Replace with h2_mitm_handler (new h2 crate integration)
// pub mod h2_mitm;
pub mod h2_upstream_forwarder;
// DEPRECATED: h2_handler replaced by h2_mitm_handler (h2 crate)
// pub mod h2_handler;
pub mod h2_mitm_handler;
