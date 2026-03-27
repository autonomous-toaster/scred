/// Upstream protocol extraction utilities
///
/// NOTE: This is a minimal utility module retained for compatibility
/// The actual HTTP/2 handling is now done by the h2 crate + H2MitmAdapter

use anyhow::Result;
use tracing::debug;
use crate::h2::alpn::HttpProtocol;

/// Information about upstream connection
#[derive(Debug, Clone)]
pub struct UpstreamConnectionInfo {
    /// Protocol negotiated with upstream (h2 or http/1.1)
    pub protocol: HttpProtocol,
    /// Server address for logging
    pub server_addr: String,
}

/// Extract protocol from ALPN bytes
pub fn extract_upstream_protocol(alpn_protocol: Option<&[u8]>) -> Result<HttpProtocol> {
    match alpn_protocol {
        Some(b"h2") => Ok(HttpProtocol::Http2),
        Some(b"http/1.1") => Ok(HttpProtocol::Http11),
        Some(proto) => {
            debug!("Unknown ALPN protocol: {:?}, defaulting to HTTP/1.1", proto);
            Ok(HttpProtocol::Http11)
        }
        None => {
            debug!("No ALPN protocol negotiated, assuming HTTP/1.1");
            Ok(HttpProtocol::Http11)
        }
    }
}

