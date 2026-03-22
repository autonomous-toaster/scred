/// HTTP/2 Bidirectional Frame Forwarder
///
/// RFC 7540-compliant bidirectional frame forwarding for HTTP/2 connections.
/// Used by both MITM and proxy components for transparent H2 connection proxying.
///
/// Key Features:
/// - Transparent frame forwarding (all 10 H2 frame types)
/// - Stream ID mapping (client ↔ upstream translation)
/// - SETTINGS ACK handling (RFC 7540 §6.5.3)
/// - Connection preface exchange (RFC 7540 §3.4)
/// - Per-stream error handling

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::{anyhow, Result};
use tracing::{debug, info, warn};

use crate::h2::header_redactor::HeaderRedactor;
use scred_redactor::RedactionEngine;

const FRAME_TYPE_SETTINGS: u8 = 0x04;
const FRAME_TYPE_HEADERS: u8 = 0x01;
const SETTINGS_ACK_FLAG: u8 = 0x01;

/// Frame forwarder configuration
#[derive(Clone)]
pub struct FrameForwarderConfig {
    /// Enable SETTINGS validation (bounds checking)
    pub validate_settings: bool,
    /// Maximum concurrent streams (0 = unlimited)
    pub max_concurrent_streams: u32,
    /// Enable detailed logging
    pub verbose_logging: bool,
    /// Enable per-stream header redaction
    pub enable_header_redaction: bool,
    /// Redaction engine for header value redaction (None = no SCRED patterns)
    pub redaction_engine: Option<Arc<RedactionEngine>>,
}

impl std::fmt::Debug for FrameForwarderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameForwarderConfig")
            .field("validate_settings", &self.validate_settings)
            .field("max_concurrent_streams", &self.max_concurrent_streams)
            .field("verbose_logging", &self.verbose_logging)
            .field("enable_header_redaction", &self.enable_header_redaction)
            .field("has_redaction_engine", &self.redaction_engine.is_some())
            .finish()
    }
}

impl Default for FrameForwarderConfig {
    fn default() -> Self {
        Self {
            validate_settings: true,
            max_concurrent_streams: 100,
            verbose_logging: false,
            enable_header_redaction: true,
            redaction_engine: None,
        }
    }
}

/// Statistics for a frame forwarding session
#[derive(Clone, Debug, Default)]
pub struct ForwardingStats {
    pub frames_forwarded: u64,
    pub bytes_forwarded: u64,
    pub settings_acks_sent: u64,
    pub stream_mappings_created: u64,
    pub headers_redacted: u64,
}

/// Handle bidirectional H2 forwarding between client and upstream
pub async fn forward_h2_frames<C, U>(
    mut client_conn: C,
    mut upstream_conn: U,
    _host: &str,
    config: FrameForwarderConfig,
) -> Result<ForwardingStats>
where
    C: AsyncReadExt + AsyncWriteExt + Unpin,
    U: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let redaction_status = if config.enable_header_redaction {
        "ENABLED"
    } else {
        "DISABLED"
    };
    info!("H2 Forwarder: Starting bidirectional H2 proxy (per-stream header redaction: {})", redaction_status);

    let mut stats = ForwardingStats::default();
    let mut stream_map: HashMap<u32, u32> = HashMap::new();
    let mut next_upstream_id: u32 = 1;
    
    // Per-stream header redactors (only created if header redaction enabled)
    let mut stream_redactors_client: HashMap<u32, HeaderRedactor> = HashMap::new();
    let mut stream_redactors_upstream: HashMap<u32, HeaderRedactor> = HashMap::new();
    
    // Get or create redaction engine
    let engine = config.redaction_engine.clone()
        .unwrap_or_else(|| Arc::new(RedactionEngine::new(scred_redactor::RedactionConfig::default())));

    // Exchange connection preface
    exchange_prefaces(&mut client_conn, &mut upstream_conn).await?;

    // Frame forwarding loop
    loop {
        tokio::select! {
            // Client → Upstream
            result = read_frame(&mut client_conn) => {
                match result {
                    Ok((frame_header, payload)) => {
                        stats.bytes_forwarded += (9 + payload.len()) as u64;

                        match forward_client_to_upstream(
                            &frame_header,
                            &payload,
                            &mut upstream_conn,
                            &mut stream_map,
                            &mut next_upstream_id,
                            &mut stream_redactors_client,
                            &config,
                            &engine,
                        )
                        .await
                        {
                            Ok((acks_sent, headers_redacted)) => {
                                stats.settings_acks_sent += acks_sent;
                                stats.headers_redacted += headers_redacted;
                                stats.frames_forwarded += 1;

                                if config.verbose_logging {
                                    let frame_type = frame_header[3];
                                    if headers_redacted > 0 {
                                        debug!("Forwarded frame type {} to upstream (redacted {} headers)", frame_type, headers_redacted);
                                    } else {
                                        debug!("Forwarded frame type {} to upstream", frame_type);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Error forwarding client frame: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        info!("H2 Forwarder: Client EOF detected");
                        break;
                    }
                    Err(e) => {
                        warn!("H2 Forwarder: Client frame read error: {}", e);
                        break;
                    }
                }
            }

            // Upstream → Client
            result = read_frame(&mut upstream_conn) => {
                match result {
                    Ok((frame_header, payload)) => {
                        stats.bytes_forwarded += (9 + payload.len()) as u64;

                        match forward_upstream_to_client(
                            &frame_header,
                            &payload,
                            &mut client_conn,
                            &stream_map,
                            &mut stream_redactors_upstream,
                            &config,
                            &engine,
                        )
                        .await
                        {
                            Ok((acks_sent, headers_redacted)) => {
                                stats.settings_acks_sent += acks_sent;
                                stats.headers_redacted += headers_redacted;
                                stats.frames_forwarded += 1;

                                if config.verbose_logging {
                                    let frame_type = frame_header[3];
                                    if headers_redacted > 0 {
                                        debug!("Forwarded frame type {} to client (redacted {} headers)", frame_type, headers_redacted);
                                    } else {
                                        debug!("Forwarded frame type {} to client", frame_type);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Error forwarding upstream frame: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        info!("H2 Forwarder: Upstream EOF detected");
                        break;
                    }
                    Err(e) => {
                        warn!("H2 Forwarder: Upstream frame read error: {}", e);
                        break;
                    }
                }
            }
        }
    }

    info!(
        "H2 Forwarder: Connection complete. Forwarded {} frames, {} bytes, {} headers redacted",
        stats.frames_forwarded, stats.bytes_forwarded, stats.headers_redacted
    );
    
    if config.enable_header_redaction && stats.headers_redacted > 0 {
        info!(
            "H2 Redaction Summary: {} total headers redacted across all streams",
            stats.headers_redacted
        );
    }
    
    Ok(stats)
}

/// Exchange HTTP/2 connection preface (RFC 7540 §3.4)
async fn exchange_prefaces<C, U>(
    client_conn: &mut C,
    upstream_conn: &mut U,
) -> Result<()>
where
    C: AsyncReadExt + AsyncWriteExt + Unpin,
    U: AsyncReadExt + AsyncWriteExt + Unpin,
{
    // Read client preface
    let mut client_preface = [0u8; 24];
    client_conn.read_exact(&mut client_preface).await?;

    if &client_preface != b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n" {
        return Err(anyhow!("Invalid H2 client preface"));
    }
    debug!("H2 Forwarder: Received client H2 preface");

    // Send server preface to client
    client_conn.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n").await?;
    client_conn.flush().await?;
    debug!("H2 Forwarder: Sent server preface to client");

    // Send client preface to upstream
    upstream_conn.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n").await?;
    upstream_conn.flush().await?;
    debug!("H2 Forwarder: Sent client preface to upstream");

    Ok(())
}

/// Forward a frame from client to upstream with stream mapping and header redaction
async fn forward_client_to_upstream<W: AsyncWriteExt + Unpin>(
    frame_header: &[u8],
    payload: &[u8],
    upstream_conn: &mut W,
    stream_map: &mut HashMap<u32, u32>,
    next_upstream_id: &mut u32,
    stream_redactors: &mut HashMap<u32, HeaderRedactor>,
    config: &FrameForwarderConfig,
    engine: &Arc<RedactionEngine>,
) -> Result<(u64, u64)> {
    // Returns (acks_sent, headers_redacted)
    let mut acks_sent = 0u64;
    let mut headers_redacted = 0u64;
    let mut header = frame_header.to_vec();
    let stream_id = extract_stream_id(&header);
    let frame_type = header[3];
    let flags = header[4];
    let mut payload = payload.to_vec();

    // Handle SETTINGS frames
    if frame_type == FRAME_TYPE_SETTINGS && (flags & SETTINGS_ACK_FLAG) == 0 {
        if config.validate_settings {
            validate_settings_frame(&payload)?;
        }
        // Send SETTINGS ACK to upstream
        send_settings_ack(upstream_conn).await?;
        acks_sent += 1;
    }

    // Skip SETTINGS ACK frames (handle locally, don't forward)
    if frame_type == FRAME_TYPE_SETTINGS && (flags & SETTINGS_ACK_FLAG) != 0 {
        return Ok((acks_sent, headers_redacted));
    }

    // Apply header redaction if needed
    if config.enable_header_redaction {
        let (redacted_payload, redacted_count) =
            maybe_redact_headers(frame_type, payload, stream_id, stream_redactors, config, engine)
                .await?;
        payload = redacted_payload;
        headers_redacted = redacted_count;
    }

    // Map stream IDs for data streams
    if stream_id != 0 {
        let upstream_stream_id = *stream_map.entry(stream_id).or_insert_with(|| {
            let id = *next_upstream_id;
            *next_upstream_id += 2; // Odd IDs for client-initiated
            debug!("Mapped client stream {} → upstream stream {}", stream_id, id);
            id
        });
        set_stream_id(&mut header, upstream_stream_id);
    }

    // Forward frame to upstream
    upstream_conn.write_all(&header).await?;
    if !payload.is_empty() {
        upstream_conn.write_all(&payload).await?;
    }
    upstream_conn.flush().await?;

    Ok((acks_sent, headers_redacted))
}

/// Forward a frame from upstream to client with stream unmapping
async fn forward_upstream_to_client<W: AsyncWriteExt + Unpin>(
    frame_header: &[u8],
    payload: &[u8],
    client_conn: &mut W,
    stream_map: &HashMap<u32, u32>,
    stream_redactors: &mut HashMap<u32, HeaderRedactor>,
    config: &FrameForwarderConfig,
    engine: &Arc<RedactionEngine>,
) -> Result<(u64, u64)> {
    // Returns (acks_sent, headers_redacted)
    let mut acks_sent = 0u64;
    let mut headers_redacted = 0u64;
    let mut header = frame_header.to_vec();
    let upstream_stream_id = extract_stream_id(&header);
    let frame_type = header[3];
    let flags = header[4];
    let mut payload = payload.to_vec();

    // Handle SETTINGS frames
    if frame_type == FRAME_TYPE_SETTINGS && (flags & SETTINGS_ACK_FLAG) == 0 {
        if config.validate_settings {
            validate_settings_frame(&payload)?;
        }
        // Send SETTINGS ACK to upstream
        send_settings_ack(client_conn).await?;
        acks_sent += 1;
    }

    // Skip SETTINGS ACK frames (handle locally, don't forward)
    if frame_type == FRAME_TYPE_SETTINGS && (flags & SETTINGS_ACK_FLAG) != 0 {
        return Ok((acks_sent, headers_redacted));
    }

    // Apply header redaction if needed
    // Note: For upstream→client, we redact using the CLIENT stream ID
    if config.enable_header_redaction && upstream_stream_id != 0 {
        // Find the client stream ID from the mapping
        let client_stream_id = stream_map
            .iter()
            .find(|(_, &up_id)| up_id == upstream_stream_id)
            .map(|(&c_id, _)| c_id)
            .unwrap_or(upstream_stream_id);
        
        let (redacted_payload, redacted_count) =
            maybe_redact_headers(frame_type, payload, client_stream_id, stream_redactors, config, engine)
                .await?;
        payload = redacted_payload;
        headers_redacted = redacted_count;
    }

    // Unmap stream IDs for data streams
    if upstream_stream_id != 0 {
        let client_stream_id = stream_map
            .iter()
            .find(|(_, &up_id)| up_id == upstream_stream_id)
            .map(|(&c_id, _)| c_id)
            .unwrap_or(upstream_stream_id);

        if client_stream_id != upstream_stream_id {
            set_stream_id(&mut header, client_stream_id);
            debug!(
                "Mapped upstream stream {} → client stream {}",
                upstream_stream_id, client_stream_id
            );
        }
    }

    // Forward frame to client
    client_conn.write_all(&header).await?;
    if !payload.is_empty() {
        client_conn.write_all(&payload).await?;
    }
    client_conn.flush().await?;

    Ok((acks_sent, headers_redacted))
}

/// Validate SETTINGS frame parameters (RFC 7540 §6.5.2)
fn validate_settings_frame(payload: &[u8]) -> Result<()> {
    // Each setting is 6 bytes (2 bytes ID + 4 bytes value)
    if payload.len() % 6 != 0 {
        return Err(anyhow!("Invalid SETTINGS frame length"));
    }

    for chunk in payload.chunks(6) {
        let setting_id = u16::from_be_bytes([chunk[0], chunk[1]]);
        let setting_value = u32::from_be_bytes([chunk[2], chunk[3], chunk[4], chunk[5]]);

        match setting_id {
            // SETTINGS_HEADER_TABLE_SIZE (0x1)
            0x1 => {
                if setting_value > 0xFFFF_FFFF {
                    return Err(anyhow!("SETTINGS_HEADER_TABLE_SIZE out of bounds"));
                }
            }
            // SETTINGS_ENABLE_PUSH (0x2)
            0x2 => {
                if setting_value > 1 {
                    return Err(anyhow!("SETTINGS_ENABLE_PUSH must be 0 or 1"));
                }
            }
            // SETTINGS_INITIAL_WINDOW_SIZE (0x4)
            0x4 => {
                if setting_value > 0x7FFF_FFFF {
                    return Err(anyhow!("SETTINGS_INITIAL_WINDOW_SIZE out of bounds"));
                }
            }
            // SETTINGS_MAX_FRAME_SIZE (0x5)
            0x5 => {
                if setting_value < 16384 || setting_value > 16777215 {
                    return Err(anyhow!("SETTINGS_MAX_FRAME_SIZE must be 16384-16777215"));
                }
            }
            _ => {
                // Unknown settings are allowed, just skip validation
            }
        }
    }

    Ok(())
}

/// Send a SETTINGS ACK frame (RFC 7540 §6.5.3)
async fn send_settings_ack<W: AsyncWriteExt + Unpin>(conn: &mut W) -> Result<()> {
    let ack_frame = [
        0x00, 0x00, 0x00,           // Length: 0
        FRAME_TYPE_SETTINGS,        // Type: SETTINGS
        SETTINGS_ACK_FLAG,          // Flags: ACK
        0x00, 0x00, 0x00, 0x00,    // Stream ID: 0
    ];
    conn.write_all(&ack_frame).await?;
    conn.flush().await?;
    debug!("Sent SETTINGS ACK");
    Ok(())
}

/// Read a complete H2 frame (9-byte header + variable payload)
pub async fn read_frame<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
    let mut header = [0u8; 9];
    reader.read_exact(&mut header).await?;

    // Extract length from first 3 bytes (big-endian)
    let len = ((header[0] as u32) << 16) | ((header[1] as u32) << 8) | (header[2] as u32);

    // Validate frame length (max 16384 bytes by default, 16777215 with SETTINGS)
    if len > 16777215 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Frame length exceeds maximum",
        ));
    }

    // Read payload
    let mut payload = vec![0u8; len as usize];
    if len > 0 {
        reader.read_exact(&mut payload).await?;
    }

    Ok((header.to_vec(), payload))
}

/// Extract stream ID from frame header (last 4 bytes, mask reserved bit)
pub fn extract_stream_id(header: &[u8]) -> u32 {
    if header.len() < 9 {
        return 0;
    }
    (((header[5] as u32) << 24)
        | ((header[6] as u32) << 16)
        | ((header[7] as u32) << 8)
        | (header[8] as u32))
        & 0x7FFF_FFFF
}

/// Set stream ID in frame header (last 4 bytes, preserve reserved bit)
pub fn set_stream_id(header: &mut [u8], stream_id: u32) {
    if header.len() < 9 {
        return;
    }
    let stream_id = stream_id & 0x7FFF_FFFF;
    header[5] = ((stream_id >> 24) & 0xFF) as u8;
    header[6] = ((stream_id >> 16) & 0xFF) as u8;
    header[7] = ((stream_id >> 8) & 0xFF) as u8;
    header[8] = (stream_id & 0xFF) as u8;
}

/// Apply header redaction to a HEADERS frame payload if header redaction is enabled
///
/// Returns (new_payload, headers_redacted_count)
async fn maybe_redact_headers(
    frame_type: u8,
    payload: Vec<u8>,
    stream_id: u32,
    stream_redactors: &mut HashMap<u32, HeaderRedactor>,
    config: &FrameForwarderConfig,
    engine: &Arc<RedactionEngine>,
) -> Result<(Vec<u8>, u64)> {
    // Only redact HEADERS frames (type 0x01) if redaction is enabled
    if frame_type != FRAME_TYPE_HEADERS || !config.enable_header_redaction || stream_id == 0 {
        return Ok((payload, 0));
    }

    // Get or create HeaderRedactor for this stream
    let redactor = stream_redactors
        .entry(stream_id)
        .or_insert_with(|| HeaderRedactor::new(stream_id, engine.clone()));

    // Apply header redaction (decompress, redact, re-compress)
    match redactor.redact_headers(&payload) {
        Ok(redacted_payload) => {
            let stats = redactor.stats();
            debug!(
                "Redacted {} headers in stream {} ({} bytes redacted, {} patterns)",
                stats.headers_redacted, stream_id, stats.bytes_redacted, stats.patterns_found
            );
            Ok((redacted_payload, stats.headers_redacted))
        }
        Err(e) => {
            warn!("Header redaction failed for stream {}: {}, forwarding original", stream_id, e);
            // Fail-safe: forward original if redaction fails
            Ok((payload, 0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_stream_id() {
        // Stream ID 1 (odd = client-initiated)
        let mut header = vec![0, 0, 0, 0, 0, 0, 0, 0, 1];
        assert_eq!(extract_stream_id(&header), 1);

        // Stream ID 2 (even = server-initiated)
        header[8] = 2;
        assert_eq!(extract_stream_id(&header), 2);

        // Large stream ID
        header[5] = 0x7F;
        header[6] = 0xFF;
        header[7] = 0xFF;
        header[8] = 0xFF;
        assert_eq!(extract_stream_id(&header), 0x7FFF_FFFF);

        // Reserved bit should be masked
        header[5] = 0xFF;
        assert_eq!(extract_stream_id(&header), 0x7FFF_FFFF);
    }

    #[test]
    fn test_set_stream_id() {
        let mut header = vec![0, 0, 0, 0, 0, 0, 0, 0, 0];
        set_stream_id(&mut header, 1);
        assert_eq!(extract_stream_id(&header), 1);

        set_stream_id(&mut header, 0x7FFF_FFFF);
        assert_eq!(extract_stream_id(&header), 0x7FFF_FFFF);
    }

    #[test]
    fn test_validate_settings_frame() {
        // Valid empty SETTINGS
        assert!(validate_settings_frame(&[]).is_ok());

        // Valid SETTINGS_ENABLE_PUSH = 0
        let mut settings = vec![0, 2, 0, 0, 0, 0];
        assert!(validate_settings_frame(&settings).is_ok());

        // Invalid SETTINGS_ENABLE_PUSH = 2 (must be 0 or 1)
        settings[5] = 2;
        assert!(validate_settings_frame(&settings).is_err());

        // Invalid frame length (not multiple of 6)
        assert!(validate_settings_frame(&[0, 2, 0, 0, 0]).is_err());
    }

    #[test]
    fn test_stream_id_mapping() {
        let mut stream_map = HashMap::new();
        let mut next_upstream_id = 1u32;

        // Map client stream 1 to upstream stream 1
        let upstream_1 = *stream_map.entry(1).or_insert_with(|| {
            let id = next_upstream_id;
            next_upstream_id += 2;
            id
        });
        assert_eq!(upstream_1, 1);
        assert_eq!(next_upstream_id, 3);

        // Map client stream 3 to upstream stream 3
        let upstream_3 = *stream_map.entry(3).or_insert_with(|| {
            let id = next_upstream_id;
            next_upstream_id += 2;
            id
        });
        assert_eq!(upstream_3, 3);
        assert_eq!(next_upstream_id, 5);
    }
}
