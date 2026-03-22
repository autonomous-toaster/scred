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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_h2_protocol() {
        let protocol = extract_upstream_protocol(Some(b"h2")).unwrap();
        assert_eq!(protocol, HttpProtocol::Http2);
    }

    #[test]
    fn test_extract_http11_protocol() {
        let protocol = extract_upstream_protocol(Some(b"http/1.1")).unwrap();
        assert_eq!(protocol, HttpProtocol::Http11);
    }

    #[test]
    fn test_extract_unknown_protocol() {
        let protocol = extract_upstream_protocol(Some(b"unknown")).unwrap();
        assert_eq!(protocol, HttpProtocol::Http11);
    }

    #[test]
    fn test_extract_none_protocol() {
        let protocol = extract_upstream_protocol(None).unwrap();
        assert_eq!(protocol, HttpProtocol::Http11);
    }
}
