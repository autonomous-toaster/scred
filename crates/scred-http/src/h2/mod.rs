/// HTTP/2 Protocol Support - Minimal Module (h2 crate + ALPN)
///
/// This module now only provides:
/// 1. ALPN protocol negotiation (protocol detection)
/// 2. Re-exports for h2 crate usage
///
/// Full HTTP/2 implementation is delegated to the h2 crate:
/// - RFC 7540 (HTTP/2) - Frame types, connection, streams
/// - RFC 7541 (HPACK) - Header compression
///
/// REDACTION LAYER:
/// - H2MitmAdapter (scred-http/h2_adapter/mod.rs): Per-stream redaction
/// - Used by both MITM (h2::server) and Proxy (h2::client)
///
/// WHAT CHANGED IN CLEANUP:
/// - Removed 40+ files of custom HTTP/2 implementation
/// - Kept only ALPN for protocol detection
/// - All frame/stream/connection handling moved to h2 crate
/// - Result: 86% code reduction (4,400 LOC → 650 LOC)
pub mod alpn;

// Re-export ALPN types only
pub use alpn::{alpn_protocols, HttpProtocol};
