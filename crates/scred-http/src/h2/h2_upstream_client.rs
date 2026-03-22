/// HTTP/2 Upstream Client
///
/// Handles communication with HTTP/2 servers, reading responses and converting to HTTP/1.1
/// format for downstream clients.

use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, error, warn};

/// HTTP/2 Connection handler for upstream servers
pub struct H2UpstreamClient {
    stream_id: u32,
}

impl H2UpstreamClient {
    pub fn new() -> Self {
        Self { stream_id: 1 }
    }

    /// Read HTTP/2 preface and settings from upstream
    /// The client sends: PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n followed by SETTINGS frame
    pub async fn read_connection_preface<R: AsyncReadExt + Unpin>(
        &self,
        reader: &mut R,
    ) -> Result<()> {
        let mut preface = [0u8; 24];
        reader.read_exact(&mut preface).await?;

        const EXPECTED_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        if &preface != EXPECTED_PREFACE {
            return Err(anyhow!("Invalid HTTP/2 connection preface"));
        }

        debug!("Received valid HTTP/2 connection preface");
        Ok(())
    }

    /// Send HTTP/2 connection preface and SETTINGS frame
    pub async fn send_connection_preface<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
    ) -> Result<()> {
        const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        writer.write_all(PREFACE).await?;

        // Send SETTINGS frame (type=4)
        // Frame format: 9-byte header + payload
        // SETTINGS frame has no END_STREAM flag
        let settings_payload = Self::encode_settings_frame();
        writer.write_all(&settings_payload).await?;

        writer.flush().await?;
        debug!("Sent HTTP/2 connection preface and SETTINGS");
        Ok(())
    }

    /// Encode SETTINGS frame
    pub fn encode_settings_frame() -> Vec<u8> {
        let mut frame = Vec::new();

        // Frame header: 9 bytes
        // Length: 0 (no settings)
        frame.push(0u8);
        frame.push(0u8);
        frame.push(0u8);

        // Type: 4 (SETTINGS)
        frame.push(4u8);

        // Flags: 0 (no ACK)
        frame.push(0u8);

        // Stream ID: 0 (connection stream)
        frame.push(0u8);
        frame.push(0u8);
        frame.push(0u8);
        frame.push(0u8);

        // Payload: empty (no settings)

        frame
    }

    /// Read and handle SETTINGS frame from upstream
    pub async fn read_settings_frame<R: AsyncReadExt + Unpin>(
        &self,
        reader: &mut R,
    ) -> Result<()> {
        let mut header = [0u8; 9];
        reader.read_exact(&mut header).await?;

        let length = ((header[0] as u32) << 16) | ((header[1] as u32) << 8) | (header[2] as u32);
        let frame_type = header[3];
        let flags = header[4];

        if frame_type != 4 {
            return Err(anyhow!("Expected SETTINGS frame, got type {}", frame_type));
        }

        // Read settings payload
        if length > 0 {
            let mut settings = vec![0u8; length as usize];
            reader.read_exact(&mut settings).await?;
            debug!("Received SETTINGS frame with {} bytes of settings", length);
        }

        // If ACK flag is set, upstream is acknowledging our SETTINGS
        if (flags & 0x01) != 0 {
            debug!("Received SETTINGS ACK from upstream");
        } else {
            debug!("Received SETTINGS from upstream");
        }

        Ok(())
    }

    /// Send SETTINGS ACK frame
    pub async fn send_settings_ack<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
    ) -> Result<()> {
        let mut frame = Vec::new();

        // Frame header
        frame.push(0u8); // Length: 0
        frame.push(0u8);
        frame.push(0u8);
        frame.push(4u8); // Type: SETTINGS
        frame.push(0x01u8); // Flags: ACK
        frame.push(0u8); // Stream ID: 0
        frame.push(0u8);
        frame.push(0u8);
        frame.push(0u8);

        writer.write_all(&frame).await?;
        writer.flush().await?;
        debug!("Sent SETTINGS ACK to upstream");
        Ok(())
    }

    /// Read HTTP/2 frame header
    pub async fn read_frame_header<R: AsyncReadExt + Unpin>(
        &self,
        reader: &mut R,
    ) -> Result<(u32, u8, u8, u32)> {
        // length, type, flags, stream_id
        let mut header = [0u8; 9];
        match reader.read_exact(&mut header).await {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(anyhow!("Connection closed by upstream"));
            }
            Err(e) => return Err(anyhow!("Failed to read frame header: {}", e)),
        }

        let length = ((header[0] as u32) << 16) | ((header[1] as u32) << 8) | (header[2] as u32);
        let frame_type = header[3];
        let flags = header[4];
        let stream_id = u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & 0x7fffffff;

        Ok((length, frame_type, flags, stream_id))
    }

    /// Read frame payload
    pub async fn read_frame_payload<R: AsyncReadExt + Unpin>(
        &self,
        reader: &mut R,
        length: u32,
    ) -> Result<Vec<u8>> {
        let mut payload = vec![0u8; length as usize];
        if length > 0 {
            reader.read_exact(&mut payload).await?;
        }
        Ok(payload)
    }

    /// Read complete HTTP/2 HEADERS frame and convert to HTTP/1.1 response header
    pub async fn read_headers_frame<R: AsyncReadExt + Unpin>(
        &self,
        reader: &mut R,
    ) -> Result<String> {
        loop {
            let (length, frame_type, flags, _stream_id) = self.read_frame_header(reader).await?;

            match frame_type {
                4 => {
                    // SETTINGS frame
                    let _settings = self.read_frame_payload(reader, length).await?;
                    debug!("Received SETTINGS frame from upstream");
                    continue;
                }
                0 => {
                    // DATA frame - skip, look for HEADERS first
                    let _data = self.read_frame_payload(reader, length).await?;
                    debug!("Skipping DATA frame before HEADERS");
                    continue;
                }
                1 => {
                    // HEADERS frame (type 1)
                    let payload = self.read_frame_payload(reader, length).await?;
                    let is_end_headers = (flags & 0x04) != 0;

                    debug!("Received HEADERS frame, end_headers={}", is_end_headers);

                    // Extract status code from HPACK-encoded payload
                    let status = self.extract_status_from_hpack(&payload)?;

                    // Build HTTP/1.1 response header
                    let response_header = format!(
                        "HTTP/1.1 {} OK\r\nContent-Type: application/octet-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                        status
                    );

                    return Ok(response_header);
                }
                3 => {
                    // RST_STREAM frame
                    let _payload = self.read_frame_payload(reader, length).await?;
                    warn!("Received RST_STREAM from upstream");
                    return Err(anyhow!("Upstream reset stream"));
                }
                9 => {
                    // GOAWAY frame
                    let _payload = self.read_frame_payload(reader, length).await?;
                    warn!("Received GOAWAY from upstream");
                    return Err(anyhow!("Upstream sent GOAWAY"));
                }
                _ => {
                    // Other frame type
                    let _payload = self.read_frame_payload(reader, length).await?;
                    debug!("Skipping frame type {} from upstream", frame_type);
                }
            }
        }
    }

    /// Read DATA frame and return body chunk
    pub async fn read_data_frame<R: AsyncReadExt + Unpin>(
        &self,
        reader: &mut R,
    ) -> Result<(Vec<u8>, bool)> {
        // is_end_stream
        loop {
            let (length, frame_type, flags, _stream_id) = self.read_frame_header(reader).await?;

            match frame_type {
                0 => {
                    // DATA frame
                    let data = self.read_frame_payload(reader, length).await?;
                    let is_end_stream = (flags & 0x01) != 0;
                    debug!("Received DATA frame: {} bytes, end_stream={}", data.len(), is_end_stream);
                    return Ok((data, is_end_stream));
                }
                4 => {
                    // SETTINGS frame - skip
                    let _payload = self.read_frame_payload(reader, length).await?;
                    continue;
                }
                3 => {
                    // RST_STREAM
                    let _payload = self.read_frame_payload(reader, length).await?;
                    return Err(anyhow!("Stream reset by upstream"));
                }
                _ => {
                    // Skip other frames
                    let _payload = self.read_frame_payload(reader, length).await?;
                    debug!("Skipping frame type {} while reading DATA", frame_type);
                }
            }
        }
    }

    /// Extract status code from HPACK-encoded HEADERS frame payload
    pub fn extract_status_from_hpack(&self, payload: &[u8]) -> Result<String> {
        if payload.is_empty() {
            return Ok("200".to_string());
        }

        let first_byte = payload[0];

        // Check if this is an indexed header (high bit set)
        if (first_byte & 0x80) != 0 {
            let index = (first_byte & 0x7F) as usize;

            // Static table indices for :status pseudo-header (RFC 7541 Table 2)
            let status = match index {
                8 => "200",
                9 => "204",
                10 => "206",
                11 => "304",
                12 => "400",
                13 => "404",
                14 => "500",
                _ => {
                    debug!("Unknown indexed header {}, trying pattern match", index);
                    return self.find_status_code_pattern(payload);
                }
            };

            debug!("Extracted status {} from indexed header", status);
            return Ok(status.to_string());
        }

        // Try pattern matching for literal representations
        self.find_status_code_pattern(payload)
    }

    /// Find status code pattern in payload bytes
    pub fn find_status_code_pattern(&self, payload: &[u8]) -> Result<String> {
        let codes = [
            (b"200", "200"), (b"301", "301"), (b"302", "302"),
            (b"304", "304"), (b"400", "400"), (b"401", "401"),
            (b"403", "403"), (b"404", "404"), (b"500", "500"),
            (b"502", "502"), (b"503", "503"),
        ];

        for (code_bytes, code_str) in &codes {
            if payload.windows(3).any(|w| w == *code_bytes) {
                debug!("Found status {} via pattern matching", code_str);
                return Ok(code_str.to_string());
            }
        }

        debug!("No status code found, defaulting to 200");
        Ok("200".to_string())
    }

    /// Send an HTTP/2 request to the upstream server
    pub async fn send_request<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        method: &str,
        path: &str,
        host: &str,
    ) -> Result<()> {
        use crate::h2::hpack_encoder::HpackEncoder;
        
        // Encode the request headers
        let headers_payload = HpackEncoder::encode_request_headers(method, path, host)?;
        
        // Build HEADERS frame
        let frame = HpackEncoder::encode_headers_frame(&headers_payload, self.stream_id);
        
        // Send frame
        writer.write_all(&frame).await?;
        writer.flush().await?;
        
        debug!("Sent HTTP/2 request: {} {} to {}", method, path, host);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h2_client_creation() {
        let client = H2UpstreamClient::new();
        assert_eq!(client.stream_id, 1);
    }

    #[test]
    fn test_extract_status_indexed_200() {
        let client = H2UpstreamClient::new();
        let payload = vec![0x88]; // Index 8 = :status 200
        let result = client.extract_status_from_hpack(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "200");
    }

    #[test]
    fn test_extract_status_indexed_404() {
        let client = H2UpstreamClient::new();
        let payload = vec![0x8D]; // Index 13 = :status 404
        let result = client.extract_status_from_hpack(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "404");
    }

    #[test]
    fn test_find_status_pattern_200() {
        let client = H2UpstreamClient::new();
        let payload = b"status: 200 OK".to_vec();
        let result = client.find_status_code_pattern(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "200");
    }

    #[test]
    fn test_find_status_pattern_404() {
        let client = H2UpstreamClient::new();
        let payload = b"error 404 not found".to_vec();
        let result = client.find_status_code_pattern(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "404");
    }

    #[test]
    fn test_settings_frame_encoding() {
        let client = H2UpstreamClient::new();
        let frame = H2UpstreamClient::encode_settings_frame();
        assert_eq!(frame.len(), 9); // 9-byte header, no payload
        assert_eq!(frame[3], 4); // Type: SETTINGS
        assert_eq!(frame[4], 0); // Flags: no ACK
    }
}
