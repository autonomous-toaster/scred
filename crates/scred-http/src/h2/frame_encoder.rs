/// HTTP/2 Frame Encoder
///
/// Encodes redacted headers and body back into HTTP/2 frame format.
/// Supports HEADERS and DATA frame generation for responses.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::debug;

/// Simple HPACK encoder (RFC 7541)
pub struct HpackEncoder {
    dynamic_table: Vec<(String, String)>,
    max_size: usize,
}

impl HpackEncoder {
    pub fn new() -> Self {
        Self {
            dynamic_table: Vec::new(),
            max_size: 4096,
        }
    }

    /// Encode headers to HPACK binary format
    pub fn encode(&mut self, headers: &HashMap<String, String>) -> Result<Vec<u8>> {
        let mut output = Vec::new();

        for (name, value) in headers {
            // For simplicity, encode as literal without indexing
            output.extend_from_slice(&self.encode_literal_header(name, value)?);
        }

        Ok(output)
    }

    /// Encode single literal header without indexing
    fn encode_literal_header(&self, name: &str, value: &str) -> Result<Vec<u8>> {
        let mut output = vec![0x00]; // Literal without indexing

        // Encode name length and value
        output.extend_from_slice(&self.encode_string(name)?);
        output.extend_from_slice(&self.encode_string(value)?);

        Ok(output)
    }

    /// Encode string with optional Huffman encoding (RFC 7541 Section 5.2)
    fn encode_string(&self, s: &str) -> Result<Vec<u8>> {
        let bytes = s.as_bytes();
        let mut output = Vec::new();

        // Length prefix (no Huffman encoding for now)
        output.extend_from_slice(&self.encode_integer(bytes.len() as u64, 7, 0x00)?);
        output.extend_from_slice(bytes);

        Ok(output)
    }

    /// Encode integer (RFC 7541 Section 5.1)
    fn encode_integer(
        &self,
        value: u64,
        prefix_bits: u8,
        pattern: u8,
    ) -> Result<Vec<u8>> {
        let max_prefix_value = (1u64 << prefix_bits) - 1;
        let mut output = vec![pattern];

        if value < max_prefix_value {
            output[0] |= (value & 0xFF) as u8;
            return Ok(output);
        }

        // Multi-byte encoding
        output[0] |= (max_prefix_value & 0xFF) as u8;
        let mut remaining = value - max_prefix_value;

        while remaining >= 128 {
            output.push((remaining % 128 + 128) as u8);
            remaining /= 128;
        }
        output.push(remaining as u8);

        Ok(output)
    }
}

impl Default for HpackEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP/2 Frame encoder
pub struct FrameEncoder;

impl FrameEncoder {
    /// Encode HEADERS frame (RFC 9113 Section 6.2)
    /// 
    /// Format:
    /// - 3 bytes: length (24-bit big-endian)
    /// - 1 byte: type (0x01 for HEADERS)
    /// - 1 byte: flags
    /// - 4 bytes: stream_id (with reserved bit)
    /// - N bytes: payload (HPACK-encoded headers)
    pub fn encode_headers_frame(
        stream_id: u32,
        headers: &HashMap<String, String>,
        end_stream: bool,
    ) -> Result<Vec<u8>> {
        let mut encoder = HpackEncoder::new();
        let payload = encoder.encode(headers)?;

        Self::encode_frame(0x01, stream_id, &payload, end_stream)
    }

    /// Encode DATA frame (RFC 9113 Section 6.1)
    ///
    /// Format:
    /// - 3 bytes: length (24-bit big-endian)
    /// - 1 byte: type (0x00 for DATA)
    /// - 1 byte: flags
    /// - 4 bytes: stream_id (with reserved bit)
    /// - N bytes: payload (body data)
    pub fn encode_data_frame(stream_id: u32, data: &[u8], end_stream: bool) -> Result<Vec<u8>> {
        Self::encode_frame(0x00, stream_id, data, end_stream)
    }

    /// Generic frame encoder
    fn encode_frame(
        frame_type: u8,
        stream_id: u32,
        payload: &[u8],
        end_stream: bool,
    ) -> Result<Vec<u8>> {
        let mut frame = Vec::new();

        // Length (24-bit big-endian)
        let length = payload.len() as u32;
        frame.push((length >> 16) as u8);
        frame.push((length >> 8) as u8);
        frame.push(length as u8);

        // Type
        frame.push(frame_type);

        // Flags
        let flags = if end_stream { 0x01 } else { 0x00 };
        frame.push(flags);

        // Stream ID (clear reserved bit, preserve stream_id)
        let stream_id_masked = stream_id & 0x7FFF_FFFF;
        frame.push((stream_id_masked >> 24) as u8);
        frame.push((stream_id_masked >> 16) as u8);
        frame.push((stream_id_masked >> 8) as u8);
        frame.push(stream_id_masked as u8);

        // Payload
        frame.extend_from_slice(payload);

        Ok(frame)
    }

    /// Parse frame header from 9 bytes
    pub fn parse_frame_header(header: &[u8; 9]) -> Result<(u32, u8, u8, u32)> {
        if header.len() != 9 {
            return Err(anyhow!("Frame header must be 9 bytes"));
        }

        // Length (3 bytes, big-endian)
        let length =
            ((header[0] as u32) << 16) | ((header[1] as u32) << 8) | (header[2] as u32);

        // Type (1 byte)
        let frame_type = header[3];

        // Flags (1 byte)
        let flags = header[4];

        // Stream ID (4 bytes, big-endian, mask out reserved bit)
        let stream_id = (((header[5] as u32) << 24)
            | ((header[6] as u32) << 16)
            | ((header[7] as u32) << 8)
            | (header[8] as u32))
            & 0x7FFF_FFFF;

        Ok((length, frame_type, flags, stream_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hpack_encoder_creation() {
        let encoder = HpackEncoder::new();
        assert_eq!(encoder.max_size, 4096);
    }

    #[test]
    fn test_encode_headers_frame() {
        let mut headers = HashMap::new();
        headers.insert(":method".to_string(), "GET".to_string());
        headers.insert(":path".to_string(), "/".to_string());

        let frame = FrameEncoder::encode_headers_frame(1, &headers, false).unwrap();

        // Should have 9-byte header + payload
        assert!(frame.len() > 9);

        // Extract length
        let length = ((frame[0] as u32) << 16) | ((frame[1] as u32) << 8) | (frame[2] as u32);
        assert_eq!(frame.len(), 9 + length as usize);
    }

    #[test]
    fn test_encode_data_frame() {
        let data = b"Hello, HTTP/2!";
        let frame = FrameEncoder::encode_data_frame(1, data, true).unwrap();

        // Should have 9-byte header + payload
        assert_eq!(frame.len(), 9 + data.len());

        // Check type (DATA = 0x00)
        assert_eq!(frame[3], 0x00);

        // Check flags (END_STREAM = 0x01)
        assert_eq!(frame[4], 0x01);
    }

    #[test]
    fn test_frame_header_parsing() {
        // Create a simple frame header
        let header = [
            0x00, 0x00, 0x0E, // Length: 14
            0x00,             // Type: DATA
            0x01,             // Flags: END_STREAM
            0x00, 0x00, 0x00, 0x01, // Stream ID: 1
        ];

        let (length, frame_type, flags, stream_id) = FrameEncoder::parse_frame_header(&header)
            .expect("Failed to parse frame header");

        assert_eq!(length, 14);
        assert_eq!(frame_type, 0x00);
        assert_eq!(flags, 0x01);
        assert_eq!(stream_id, 1);
    }

    #[test]
    fn test_encode_integer() {
        let encoder = HpackEncoder::new();

        // Small integer (fits in one byte)
        let result = encoder.encode_integer(10, 7, 0x00).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 0x0A);

        // Large integer (needs multiple bytes)
        let result = encoder.encode_integer(1337, 5, 0x00).unwrap();
        assert!(result.len() > 1);
    }

    #[test]
    fn test_encode_string() {
        let encoder = HpackEncoder::new();
        let result = encoder.encode_string("hello").unwrap();

        // Should have length prefix + "hello"
        assert!(result.len() > 5);
    }
}
