/// ALPN (Application Layer Protocol Negotiation)
///
/// HTTP protocol selection during TLS handshake.
/// Both MITM and proxy use this to detect client protocol preference.

use std::fmt;

/// HTTP protocol selected during TLS ALPN negotiation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpProtocol {
    /// HTTP/2 selected by client
    Http2,
    /// HTTP/1.1 selected by client (or no ALPN)
    Http11,
}

impl HttpProtocol {
    /// Parse from ALPN protocol name (bytes)
    ///
    /// # Examples
    /// ```
    /// use scred_http::h2::HttpProtocol;
    ///
    /// assert_eq!(HttpProtocol::from_bytes(b"h2"), Some(HttpProtocol::Http2));
    /// assert_eq!(HttpProtocol::from_bytes(b"http/1.1"), Some(HttpProtocol::Http11));
    /// assert_eq!(HttpProtocol::from_bytes(b"h2c"), None);
    /// ```
    pub fn from_bytes(protocol: &[u8]) -> Option<Self> {
        match protocol {
            b"h2" => Some(HttpProtocol::Http2),
            b"http/1.1" => Some(HttpProtocol::Http11),
            _ => None,
        }
    }

    /// Convert to human-readable string for logging
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpProtocol::Http2 => "h2 (HTTP/2)",
            HttpProtocol::Http11 => "http/1.1 (HTTP/1.1)",
        }
    }

    /// Check if this is HTTP/2
    pub fn is_h2(&self) -> bool {
        matches!(self, HttpProtocol::Http2)
    }

    /// Check if this is HTTP/1.1
    pub fn is_http11(&self) -> bool {
        matches!(self, HttpProtocol::Http11)
    }
}

impl fmt::Display for HttpProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// ALPN protocols advertised by SCRED
///
/// When accepting TLS connections, we advertise both h2 and http/1.1
/// so clients can select their preferred protocol.
///
/// Phase 2 (Full HTTP/2): Now supports full h2 multiplexing with per-stream redaction
///
/// # Examples
/// ```
/// use scred_http::h2::alpn_protocols;
///
/// let protos = alpn_protocols();
/// assert_eq!(protos.len(), 2);
/// assert!(protos.iter().any(|p| p == b"h2"));
/// assert!(protos.iter().any(|p| p == b"http/1.1"));
/// ```
pub fn alpn_protocols() -> Vec<Vec<u8>> {
    vec![
        b"h2".to_vec(),        // HTTP/2 - Phase 2: Full support with multiplexing
        b"http/1.1".to_vec(),  // HTTP/1.1 fallback
    ]
}

