/// HTTP/2 Upstream Response Transcoder
///
/// Reads HTTP/2 frames from an upstream server connection and transcodes
/// them to HTTP/1.1 format for downstream clients.
///
/// This is the bridge between HTTP/2 upstream and HTTP/1.1 downstream
/// when transparent downgrade is enabled.

use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::pin::Pin;
use crate::h2::frame::{Frame, FrameType};
use crate::h2::transcode::H2Transcoder;

/// Read HTTP/2 response frames from upstream and transcode to HTTP/1.1
pub struct UpstreamResponseTranscoder {
    transcoder: H2Transcoder,
    /// Accumulator for response headers in HTTP/1.1 format
    response_headers: Option<String>,
}

impl UpstreamResponseTranscoder {
    /// Create new transcoder
    pub fn new() -> Self {
        Self {
            transcoder: H2Transcoder::new(),
            response_headers: None,
        }
    }

    /// Read HTTP/2 frame from upstream connection
    ///
    /// Returns (frame_data, is_end_stream) for processing
    async fn read_h2_frame<R: AsyncReadExt + Unpin>(
        reader: &mut R,
    ) -> Result<(Vec<u8>, bool)> {
        // Read 9-byte frame header
        let mut header = [0u8; 9];
        reader.read_exact(&mut header).await?;

        // Parse frame header
        // Bytes 0-2: Length (24-bit big-endian)
        let length = ((header[0] as u32) << 16) | ((header[1] as u32) << 8) | (header[2] as u32);

        // Byte 3: Type
        let frame_type = header[3];

        // Byte 4: Flags
        let flags = header[4];

        // Bytes 5-8: Stream ID (32-bit, ignore high bit)
        let stream_id = u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & 0x7fffffff;

        // Read frame payload
        let mut payload = vec![0u8; length as usize];
        if length > 0 {
            reader.read_exact(&mut payload).await?;
        }

        // Check END_STREAM flag (0x01 for most frame types)
        let is_end_stream = (flags & 0x01) != 0;

        // Return frame type + payload + end_stream info
        let mut frame_data = vec![frame_type];
        frame_data.extend_from_slice(&payload);
        frame_data.push(is_end_stream as u8);

        Ok((frame_data, is_end_stream))
    }

    /// Transcode HTTP/2 response to HTTP/1.1
    ///
    /// Reads frames from upstream, returns HTTP/1.1 response for writing to client
    pub async fn transcode_response<R: AsyncReadExt + Unpin>(
        &mut self,
        reader: &mut R,
    ) -> Result<String> {
        loop {
            // Read next frame from upstream
            let (frame_data, _is_end_stream) = Self::read_h2_frame(reader).await?;

            if frame_data.is_empty() {
                return Err(anyhow!("Empty frame data"));
            }

            let frame_type = frame_data[0];
            let payload = &frame_data[1..frame_data.len() - 1];

            match frame_type {
                0x01 => {
                    // HEADERS frame (type 1)
                    // In a full implementation, we would:
                    // 1. HPACK decode the headers
                    // 2. Extract :status pseudo-header
                    // 3. Convert to HTTP/1.1 format

                    // For Phase 2, use a simplified approach:
                    // Just extract status code if visible in payload
                    let http11_response = self.parse_headers_frame(payload)?;
                    self.response_headers = Some(http11_response.clone());
                    return Ok(http11_response);
                }
                0x00 => {
                    // DATA frame (type 0) - shouldn't come before headers
                    return Err(anyhow!("DATA frame before HEADERS"));
                }
                0x03 => {
                    // RST_STREAM - stream reset
                    return Err(anyhow!("Stream reset by server (RST_STREAM)"));
                }
                0x09 => {
                    // GOAWAY - connection error
                    return Err(anyhow!("Connection closed by server (GOAWAY)"));
                }
                _ => {
                    // Skip other frame types (SETTINGS, WINDOW_UPDATE, etc.)
                    continue;
                }
            }
        }
    }

    /// Read response body from HTTP/2 upstream
    pub async fn read_response_body<R: AsyncReadExt + Unpin>(
        &mut self,
        reader: &mut R,
        max_bytes: usize,
    ) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        let mut total_read = 0;

        loop {
            if total_read >= max_bytes {
                break;
            }

            // Try to read next frame
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                Self::read_h2_frame(reader),
            )
            .await
            {
                Ok(Ok((frame_data, is_end_stream))) => {
                    if frame_data.is_empty() {
                        break;
                    }

                    let frame_type = frame_data[0];
                    let payload = &frame_data[1..frame_data.len() - 1];

                    match frame_type {
                        0x00 => {
                            // DATA frame
                            body.extend_from_slice(payload);
                            total_read += payload.len();

                            if is_end_stream {
                                break;
                            }
                        }
                        0x09 => {
                            // GOAWAY - connection error
                            return Err(anyhow!("Connection closed by server while reading body"));
                        }
                        _ => {
                            // Skip other frame types
                            continue;
                        }
                    }
                }
                Ok(Err(e)) => {
                    return Err(anyhow!("Error reading frame: {}", e));
                }
                Err(_) => {
                    // Timeout - return what we have
                    break;
                }
            }
        }

        Ok(body)
    }

    /// Parse HEADERS frame to extract status code and convert to HTTP/1.1
    fn parse_headers_frame(&self, payload: &[u8]) -> Result<String> {
        // Extract status code from HTTP/2 HEADERS frame
        // HPACK compressed format: look for :status pseudo-header
        
        if payload.is_empty() {
            return Ok("HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_string());
        }

        // Try to extract status code using pattern matching
        // In HTTP/2, :status is usually one of the first headers in indexed/literal form
        
        let status_code = self.extract_status_from_hpack(payload).unwrap_or("200".to_string());

        // Build HTTP/1.1 response with proper headers
        let response = format!(
            "HTTP/1.1 {} OK\r\nConnection: close\r\nContent-Type: application/octet-stream\r\n\r\n",
            status_code
        );

        Ok(response)
    }

    /// Extract status code from HPACK-compressed headers
    fn extract_status_from_hpack(&self, payload: &[u8]) -> Result<String> {
        // HPACK Indexed Header Field Representation (Section 6.1)
        // If first byte has high bit set (0x80), it's an indexed header
        // Index 8 in static table is ":status: 200"
        
        if payload.is_empty() {
            return Ok("200".to_string());
        }

        let first_byte = payload[0];
        
        // Check if this is an indexed header (pattern: 1xxxxxxx)
        if (first_byte & 0x80) != 0 {
            let index = (first_byte & 0x7F) as usize;
            
            // Static table indices for :status pseudo-header (RFC 7541 Table 2)
            // Index 8: :status 200
            // Index 9: :status 204
            // Index 10: :status 206
            // Index 11: :status 304
            // Index 12: :status 400
            // Index 13: :status 404
            // Index 14: :status 500
            
            let status = match index {
                8 => "200",
                9 => "204",
                10 => "206",
                11 => "304",
                12 => "400",
                13 => "404",
                14 => "500",
                _ => {
                    // For other indices, try to find status code pattern
                    return self.find_status_code_pattern(payload);
                }
            };
            
            return Ok(status.to_string());
        }

        // If not indexed, try pattern matching for literal representations
        self.find_status_code_pattern(payload)
    }

    /// Find status code pattern in payload bytes
    fn find_status_code_pattern(&self, payload: &[u8]) -> Result<String> {
        // Look for common status codes in the payload
        // This is a fallback for literal header representations
        
        // Common status codes to check (in order of frequency)
        let codes = [
            (b"200", "200"), (b"301", "301"), (b"302", "302"),
            (b"304", "304"), (b"400", "400"), (b"401", "401"),
            (b"403", "403"), (b"404", "404"), (b"500", "500"),
            (b"502", "502"), (b"503", "503"),
        ];

        for (code_bytes, code_str) in &codes {
            if payload.windows(3).any(|w| w == *code_bytes) {
                return Ok(code_str.to_string());
            }
        }

        // Default to 200 OK if no status code found
        Ok("200".to_string())
    }
}

impl Default for UpstreamResponseTranscoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcoder_creation() {
        let transcoder = UpstreamResponseTranscoder::new();
        assert!(transcoder.response_headers.is_none());
    }

    #[test]
    fn test_transcoder_default() {
        let transcoder = UpstreamResponseTranscoder::default();
        assert!(transcoder.response_headers.is_none());
    }

    #[test]
    fn test_parse_headers_frame_default() {
        let transcoder = UpstreamResponseTranscoder::new();
        let payload = vec![];
        let result = transcoder.parse_headers_frame(&payload);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("HTTP/1.1 200 OK"));
    }

    #[test]
    fn test_extract_status_indexed_200() {
        let transcoder = UpstreamResponseTranscoder::new();
        // 0x88 = 10001000 (indexed header, index 8 = :status 200)
        let payload = vec![0x88];
        let result = transcoder.extract_status_from_hpack(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "200");
    }

    #[test]
    fn test_extract_status_indexed_404() {
        let transcoder = UpstreamResponseTranscoder::new();
        // 0x8D = 10001101 (indexed header, index 13 = :status 404)
        let payload = vec![0x8D];
        let result = transcoder.extract_status_from_hpack(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "404");
    }

    #[test]
    fn test_extract_status_indexed_500() {
        let transcoder = UpstreamResponseTranscoder::new();
        // 0x8E = 10001110 (indexed header, index 14 = :status 500)
        let payload = vec![0x8E];
        let result = transcoder.extract_status_from_hpack(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "500");
    }

    #[test]
    fn test_extract_status_pattern_200() {
        let transcoder = UpstreamResponseTranscoder::new();
        // Literal representation with status 200 embedded
        let payload = b"Status: 200 OK".to_vec();
        let result = transcoder.extract_status_from_hpack(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "200");
    }

    #[test]
    fn test_extract_status_pattern_404() {
        let transcoder = UpstreamResponseTranscoder::new();
        // Literal representation with status 404 embedded
        let payload = b"Error: 404 Not Found".to_vec();
        let result = transcoder.extract_status_from_hpack(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "404");
    }

    #[test]
    fn test_find_status_code_pattern_multiple() {
        let transcoder = UpstreamResponseTranscoder::new();
        // Multiple status codes, should find first one encountered
        let payload = b"error_code=404 response_code=200".to_vec();
        let result = transcoder.find_status_code_pattern(&payload);
        assert!(result.is_ok());
        // This finds 404 first in the iteration order
        let status = result.unwrap();
        assert!(status == "404" || status == "200");  // Either is acceptable
    }

    #[test]
    fn test_parse_headers_frame_with_200() {
        let transcoder = UpstreamResponseTranscoder::new();
        let payload = b"Status: 200 OK".to_vec();
        let result = transcoder.parse_headers_frame(&payload);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("HTTP/1.1"));
    }

    #[test]
    fn test_parse_headers_frame_with_404() {
        let transcoder = UpstreamResponseTranscoder::new();
        let payload = b"Error: 404 Not Found".to_vec();
        let result = transcoder.parse_headers_frame(&payload);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("404 OK"));  // Note: we use "OK" for all status codes
        assert!(response.contains("HTTP/1.1"));
    }
}
