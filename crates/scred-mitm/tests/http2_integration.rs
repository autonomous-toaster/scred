/// Integration tests for HTTP/2 support in SCRED MITM
///
/// Tests ALPN negotiation, client/upstream HTTP/2 detection,
/// and transparent downgrade to HTTP/1.1 (Phase 1 strategy).
///
/// Note: Full end-to-end tests require running actual MITM and connecting
/// with HTTP/2 clients. These tests focus on the ALPN/protocol detection
/// integration points.

#[cfg(test)]
mod http2_integration_tests {
    use scred_http::h2::alpn::{HttpProtocol, alpn_protocols};

    /// Test ALPN protocols list
    #[test]
    fn test_alpn_protocols_advertised() {
        let protos = alpn_protocols();
        assert_eq!(protos.len(), 2);
        assert!(protos.iter().any(|p| p == b"h2"));
        assert!(protos.iter().any(|p| p == b"http/1.1"));
    }

    /// Test HTTP/2 protocol detection
    #[test]
    fn test_http2_protocol_detection() {
        let proto = HttpProtocol::from_bytes(b"h2").unwrap();
        assert!(proto.is_h2());
        assert!(!proto.is_http11());
    }

    /// Test HTTP/1.1 protocol detection
    #[test]
    fn test_http11_protocol_detection() {
        let proto = HttpProtocol::from_bytes(b"http/1.1").unwrap();
        assert!(proto.is_http11());
        assert!(!proto.is_h2());
    }

    /// Test unknown protocol handling
    #[test]
    fn test_unknown_protocol_returns_none() {
        let proto = HttpProtocol::from_bytes(b"unknown");
        assert_eq!(proto, None);
    }

    /// Test protocol display
    #[test]
    fn test_protocol_display() {
        assert_eq!(
            format!("{}", HttpProtocol::Http2),
            "h2 (HTTP/2)"
        );
        assert_eq!(
            format!("{}", HttpProtocol::Http11),
            "http/1.1 (HTTP/1.1)"
        );
    }

    /// Test protocol string representation
    #[test]
    fn test_protocol_as_str() {
        assert_eq!(HttpProtocol::Http2.as_str(), "h2 (HTTP/2)");
        assert_eq!(HttpProtocol::Http11.as_str(), "http/1.1 (HTTP/1.1)");
    }

    /// Test cloning protocols
    #[test]
    fn test_protocol_clone() {
        let proto1 = HttpProtocol::Http2;
        let proto2 = proto1;
        assert_eq!(proto1, proto2);
    }

    /// Test h2_reader conversion functionality
    #[test]
    fn test_h2_response_reader() {
        use scred_http::h2::h2_reader::H2ResponseReader;
        
        let mut reader = H2ResponseReader::new();
        assert!(!reader.is_headers_complete());
        assert_eq!(reader.status_code(), None);

        reader.set_status_code(200);
        assert_eq!(reader.status_code(), Some(200));

        reader.set_headers_complete();
        assert!(reader.is_headers_complete());
    }

    /// Test h2 transcode state machine
    #[test]
    fn test_h2_transcode_state_machine() {
        use scred_http::h2::transcode::{H2Transcoder, TranscodeState};

        let transcoder = H2Transcoder::new();
        assert_eq!(*transcoder.state(), TranscodeState::WaitingForHeaders);
        assert!(!transcoder.is_complete());
    }

    /// Test protocol fallback behavior
    #[test]
    fn test_protocol_fallback_to_http11() {
        // When ALPN protocol is not negotiated, default to HTTP/1.1
        let proto = HttpProtocol::Http11;
        assert!(!proto.is_h2());
    }

    /// Test protocol in http2 module
    #[test]
    fn test_http2_frame_parsing() {
        use scred_http::h2::frame::Frame;
        
        // Example HTTP/2 DATA frame header (9 bytes)
        let frame_data = &[
            0x00, 0x00, 0x0A, // Length: 10 bytes
            0x00,             // Type: DATA (0)
            0x01,             // Flags: END_STREAM
            0x00, 0x00, 0x00, 0x01, // Stream ID: 1
        ];

        let frame = Frame::parse(frame_data).unwrap();
        assert_eq!(frame.length, 10);
        assert_eq!(frame.stream_id, 1);
    }
}

/// Helper module for manual testing with real servers
/// (Not automated, but useful for manual verification)
#[cfg(test)]
mod manual_h2_test_notes {
    //! Manual testing guide for HTTP/2 integration:
    //!
    //! 1. Start MITM:
    //!    cargo run --bin scred-mitm -- --listen 0.0.0.0:8443 \
    //!      --redact-requests --redact-responses --cert /path/to/cert.pem
    //!
    //! 2. Test with curl (HTTP/2 client):
    //!    curl --http2 --insecure --cacert mitm-ca.pem https://localhost:8443
    //!
    //! 3. Verify protocol negotiation:
    //!    - Look for "TLS handshake successful with client, protocol: h2 (HTTP/2)"
    //!    - Look for "Upstream server supports HTTP/2" or "HTTP/1.1"
    //!    - Verify response is received (transparent downgrade)
    //!
    //! 4. Test with real HTTP/2 servers:
    //!    - nghttp.org: curl --http2 https://nghttp.org
    //!    - Test that MITM detects h2 from upstream
    //!    - Verify transparent transcoding to HTTP/1.1 for downstream
    //!
    //! 5. Expected behavior (Phase 1):
    //!    - Client connects with h2 ALPN → MITM accepts, logs protocol
    //!    - MITM connects to upstream, detects protocol
    //!    - If upstream is h2: logs "will transcode to HTTP/1.1"
    //!    - If upstream is http/1.1: logs "HTTP/1.1"
    //!    - All traffic is HTTP/1.1 between MITM and client (transparent)
    //!
    //! 6. Phase 2 (future):
    //!    - True HTTP/2 multiplexing for upstream h2 servers
    //!    - Improved throughput for concurrent requests
    //!    - Per-stream redaction state tracking
}
