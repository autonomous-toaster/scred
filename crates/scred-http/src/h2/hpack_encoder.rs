/// HTTP/2 Request Builder with HPACK encoding
/// 
/// Handles encoding HTTP/1.1 requests as HTTP/2 HEADERS frames

use anyhow::{anyhow, Result};
use tracing::debug;

/// Simple HPACK encoder for common cases
pub struct HpackEncoder;

impl HpackEncoder {
    /// Encode a simple HTTP request as HPACK
    /// For now, uses indexed headers from static table for common cases
    pub fn encode_request_headers(
        method: &str,
        path: &str,
        host: &str,
    ) -> Result<Vec<u8>> {
        let mut payload = Vec::new();

        // Encode :method pseudo-header
        // Index 2 = GET, Index 3 = POST
        let method_index = match method {
            "GET" => 2u8,
            "POST" => 3u8,
            "HEAD" => 21u8, // Static table index
            "DELETE" => 5u8,
            "PUT" => 4u8,
            "CONNECT" => 6u8,
            "OPTIONS" => 7u8,
            "TRACE" => 8u8,
            _ => {
                // Literal header - not indexed
                payload.push(0x0F); // Literal without indexing
                payload.push(method.len() as u8);
                payload.extend_from_slice(method.as_bytes());
                0
            }
        };

        if method_index > 0 {
            // Indexed representation
            payload.push(0x80 | method_index);
        }

        // Encode :path pseudo-header (index 4)
        // Literal with incremental indexing
        payload.push(0x44); // Literal, index 4 (:path)
        payload.push(path.len() as u8);
        payload.extend_from_slice(path.as_bytes());

        // Encode :scheme pseudo-header (index 7 = https)
        payload.push(0x87); // Indexed representation, index 7 (:scheme https)

        // Encode :authority pseudo-header (index 1)
        payload.push(0x41); // Literal with incremental indexing, index 1
        payload.push(host.len() as u8);
        payload.extend_from_slice(host.as_bytes());

        debug!(
            "Encoded HPACK headers: method={}, path={}, host={}",
            method, path, host
        );

        Ok(payload)
    }

    /// Encode a HEADERS frame with END_STREAM flag
    pub fn encode_headers_frame(payload: &[u8], stream_id: u32) -> Vec<u8> {
        let mut frame = Vec::new();

        // Frame header: 9 bytes
        let length = payload.len() as u32;
        frame.push((length >> 16) as u8);
        frame.push((length >> 8) as u8);
        frame.push(length as u8);

        // Type: 1 (HEADERS)
        frame.push(1u8);

        // Flags: 0x05 = END_HEADERS (0x04) | END_STREAM (0x01)
        frame.push(0x05u8);

        // Stream ID (4 bytes, big-endian, high bit must be 0)
        frame.push((stream_id >> 24) as u8);
        frame.push((stream_id >> 16) as u8);
        frame.push((stream_id >> 8) as u8);
        frame.push(stream_id as u8);

        // Payload
        frame.extend_from_slice(payload);

        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_get_request() {
        let payload = HpackEncoder::encode_request_headers("GET", "/anything", "httpbin.org");
        assert!(payload.is_ok());
        let data = payload.unwrap();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_encode_post_request() {
        let payload = HpackEncoder::encode_request_headers("POST", "/post", "example.com");
        assert!(payload.is_ok());
        let data = payload.unwrap();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_encode_headers_frame() {
        let payload = b"test payload";
        let frame = HpackEncoder::encode_headers_frame(payload, 1);

        // Should have 9-byte header + payload length
        assert_eq!(frame.len(), 9 + payload.len());

        // Frame type should be 1 (HEADERS)
        assert_eq!(frame[3], 1);

        // Flags should be 0x05 (END_HEADERS | END_STREAM)
        assert_eq!(frame[4], 0x05);

        // Stream ID should be 1
        assert_eq!(frame[5..9], [0u8, 0u8, 0u8, 1u8]);
    }

    #[test]
    fn test_encode_method_get() {
        let payload = HpackEncoder::encode_request_headers("GET", "/", "example.com");
        assert!(payload.is_ok());
        let data = payload.unwrap();
        // GET should be indexed as 2
        assert!(data[0] == 0x82); // 10000010 = indexed header, index 2
    }

    #[test]
    fn test_encode_method_post() {
        let payload = HpackEncoder::encode_request_headers("POST", "/", "example.com");
        assert!(payload.is_ok());
        let data = payload.unwrap();
        // POST should be indexed as 3
        assert!(data[0] == 0x83); // 10000011 = indexed header, index 3
    }

    #[test]
    fn test_encode_custom_method() {
        let payload = HpackEncoder::encode_request_headers("PATCH", "/", "example.com");
        assert!(payload.is_ok());
        // Should use literal encoding for PATCH
    }
}
