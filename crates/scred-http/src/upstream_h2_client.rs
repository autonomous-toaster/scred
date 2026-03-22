/// HTTP/2 Upstream Client - Transparent h2 Support for MITM & Proxy
///
/// This module handles HTTP/2 connections to upstream servers with automatic
/// protocol negotiation and transparent HTTP/1.1 transcoding.
///
/// Key features:
/// - ALPN negotiation (h2 + http/1.1)
/// - Automatic h2 → HTTP/1.1 transcoding
/// - Integration with existing redaction engine
/// - Reusable for both MITM and proxy modes
///
/// Architecture:
/// 1. Client connects to SCRED (HTTP/1.1 or h2, both downgraded to http/1.1)
/// 2. SCRED connects upstream with h2 ALPN support
/// 3. If upstream negotiates h2: use this module to transcode
/// 4. If upstream negotiates http/1.1: use existing streaming_request path
/// 5. Transcode result fed to redaction engine (unchanged)
/// 6. Response streamed back to client as HTTP/1.1
///
/// Design principle: Keep HTTP/2 complexity isolated in scred-http,
/// invisible to MITM and proxy logic.

use anyhow::{anyhow, Result};
use tracing::{debug, warn};

use crate::h2::transcode::H2Transcoder;
use crate::h2::alpn::HttpProtocol;

/// Information about upstream connection
#[derive(Debug, Clone)]
pub struct UpstreamConnectionInfo {
    /// Protocol negotiated with upstream (h2 or http/1.1)
    pub protocol: HttpProtocol,
    /// Server address for logging
    pub server_addr: String,
}

/// HTTP/2 upstream response wrapper
///
/// Wraps a tokio stream and provides:
/// - Automatic h2 frame parsing (via http2 crate)
/// - Transcoding to HTTP/1.1
/// - Per-chunk reading for streaming integration
pub struct H2UpstreamReader<R> {
    /// Underlying stream (e.g., TLS connection to upstream)
    #[allow(dead_code)]
    reader: R,
    /// Transcode state machine
    transcoder: H2Transcoder,
    /// Buffer for transcoded HTTP/1.1 headers
    header_buffer: Option<Vec<u8>>,
    /// Whether we've sent headers yet
    headers_sent: bool,
    /// Buffer for reading h2 frames
    #[allow(dead_code)]
    frame_buffer: Vec<u8>,
}

impl<R> H2UpstreamReader<R> {
    /// Create new HTTP/2 upstream reader
    ///
    /// # Arguments
    /// * `reader` - Underlying stream (typically TLS connection to upstream)
    ///
    /// # Returns
    /// New reader ready to process HTTP/2 frames
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            transcoder: H2Transcoder::new(),
            header_buffer: None,
            headers_sent: false,
            frame_buffer: vec![0u8; 16384], // 16KB buffer for frames
        }
    }

    /// Initialize HTTP/2 connection
    ///
    /// Call after TLS handshake to set up h2 protocol handler
    pub fn init(&mut self) -> Result<()> {
        debug!("Initializing HTTP/2 upstream connection");
        // http2::client::Connection will be created when we first read
        // For now, just mark as initialized
        Ok(())
    }

    /// Process next chunk of data and return transcoded HTTP/1.1 bytes
    ///
    /// This function:
    /// 1. Reads h2 frames from upstream
    /// 2. Detects HEADERS frame → transcode to HTTP/1.1 status + headers
    /// 3. Detects DATA frames → pass through as body bytes
    /// 4. Detects END_STREAM → mark complete
    /// 5. Returns HTTP/1.1 formatted bytes for redaction
    ///
    /// Returns:
    /// - Ok(Some(bytes)) - Transcoded HTTP/1.1 content
    /// - Ok(None) - END_STREAM received, response complete
    /// - Err - Connection error or protocol violation
    pub async fn read_transcoded_chunk(&mut self) -> Result<Option<Vec<u8>>> {
        debug!("Reading transcoded h2 chunk");

        // If we have buffered headers, send them first
        if let Some(headers) = self.header_buffer.take() {
            if !self.headers_sent {
                self.headers_sent = true;
                debug!("Returning transcoded headers ({} bytes)", headers.len());
                return Ok(Some(headers));
            }
        }

        // Check if transcoding is complete
        if self.transcoder.is_complete() {
            debug!("HTTP/2 response complete (END_STREAM received)");
            return Ok(None);
        }

        // Read next h2 frame and process
        // This is a simplified version - real implementation would use http2 crate
        // to properly handle connection preface, SETTINGS, etc.
        //
        // TODO: Full integration with http2 crate's Connection handler
        // For Phase 1, we route this to existing h1 path; Phase 2 uses full h2
        
        Err(anyhow!("HTTP/2 upstream not yet fully implemented (Phase 2)"))
    }
}

/// Convenience wrapper for routing based on protocol negotiation
///
/// This function is called by MITM/proxy after upstream TLS handshake
/// to determine how to handle the connection.
///
/// # Arguments
/// * `protocol` - Protocol negotiated with upstream (Http2 or Http11)
/// * `info` - Connection information
///
/// # Returns
/// - If Http2: Returns handler for HTTP/2 path (transcode to http/1.1)
/// - If Http11: Returns handler for HTTP/1.1 path (existing streaming)
pub fn select_upstream_handler(
    protocol: HttpProtocol,
    info: UpstreamConnectionInfo,
) -> UpstreamHandler {
    match protocol {
        HttpProtocol::Http2 => {
            warn!(
                "Upstream http/2 detected: {}, will transcode to HTTP/1.1 (Phase 1 transparent fallback)",
                info.server_addr
            );
            UpstreamHandler::H2(info)
        }
        HttpProtocol::Http11 => {
            debug!("Upstream http/1.1: {}, using existing streaming path", info.server_addr);
            UpstreamHandler::Http11(info)
        }
    }
}

/// Upstream handler selection
#[derive(Debug, Clone)]
pub enum UpstreamHandler {
    /// Upstream server negotiated HTTP/2
    H2(UpstreamConnectionInfo),
    /// Upstream server negotiated HTTP/1.1 (or downgrade occurred)
    Http11(UpstreamConnectionInfo),
}

impl UpstreamHandler {
    /// Check if this is HTTP/2
    pub fn is_h2(&self) -> bool {
        matches!(self, UpstreamHandler::H2(_))
    }

    /// Check if this is HTTP/1.1
    pub fn is_http11(&self) -> bool {
        matches!(self, UpstreamHandler::Http11(_))
    }

    /// Get connection info
    pub fn info(&self) -> &UpstreamConnectionInfo {
        match self {
            UpstreamHandler::H2(info) | UpstreamHandler::Http11(info) => info,
        }
    }
}

/// Helper to extract upstream protocol from TLS connection
///
/// Call after TLS handshake with upstream to determine protocol.
/// This is typically called in tls_mitm.rs or proxy code.
///
/// # Example
/// ```rust,ignore
/// let tls_stream = connector.connect(...).await?;
/// let protocol = extract_upstream_protocol(&tls_stream)?;
/// let handler = select_upstream_handler(protocol, connection_info);
/// ```
pub fn extract_upstream_protocol(alpn_protocol: Option<&[u8]>) -> Result<HttpProtocol> {
    match alpn_protocol {
        Some(b"h2") => Ok(HttpProtocol::Http2),
        Some(b"http/1.1") => Ok(HttpProtocol::Http11),
        Some(proto) => {
            warn!("Unknown ALPN protocol: {:?}, defaulting to HTTP/1.1", proto);
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
    fn test_select_upstream_handler_h2() {
        let info = UpstreamConnectionInfo {
            protocol: HttpProtocol::Http2,
            server_addr: "example.com:443".to_string(),
        };
        let handler = select_upstream_handler(HttpProtocol::Http2, info);
        assert!(handler.is_h2());
        assert!(!handler.is_http11());
    }

    #[test]
    fn test_select_upstream_handler_http11() {
        let info = UpstreamConnectionInfo {
            protocol: HttpProtocol::Http11,
            server_addr: "example.com:443".to_string(),
        };
        let handler = select_upstream_handler(HttpProtocol::Http11, info);
        assert!(handler.is_http11());
        assert!(!handler.is_h2());
    }

    #[test]
    fn test_extract_upstream_protocol_h2() {
        let result = extract_upstream_protocol(Some(b"h2")).unwrap();
        assert_eq!(result, HttpProtocol::Http2);
    }

    #[test]
    fn test_extract_upstream_protocol_http11() {
        let result = extract_upstream_protocol(Some(b"http/1.1")).unwrap();
        assert_eq!(result, HttpProtocol::Http11);
    }

    #[test]
    fn test_extract_upstream_protocol_none() {
        let result = extract_upstream_protocol(None).unwrap();
        assert_eq!(result, HttpProtocol::Http11);
    }

    #[test]
    fn test_extract_upstream_protocol_unknown() {
        let result = extract_upstream_protocol(Some(b"unknown")).unwrap();
        // Should default to HTTP/1.1
        assert_eq!(result, HttpProtocol::Http11);
    }

    #[test]
    fn test_upstream_connection_info() {
        let info = UpstreamConnectionInfo {
            protocol: HttpProtocol::Http2,
            server_addr: "api.example.com:443".to_string(),
        };
        assert_eq!(info.server_addr, "api.example.com:443");
        assert_eq!(info.protocol, HttpProtocol::Http2);
    }
}
