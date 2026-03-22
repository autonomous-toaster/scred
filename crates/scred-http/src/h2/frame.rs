/// HTTP/2 Frame Parsing
///
/// Parse HTTP/2 frame headers and identify frame types.
/// RFC 7540: https://tools.ietf.org/html/rfc7540#section-4.1
///
/// Frame Format (9 bytes + payload):
/// ```
/// +-----------------------------------------------+
/// |                 Length (24)                   |  bytes 0-2 (big-endian)
/// +---------------+---------------+-------------+
/// |   Type (8)    |   Flags (8)    | R | Stream ID (31) |  bytes 3-8
/// +---------------+---------------+---+---------+
/// |                     Payload (0...)             |  bytes 9+ (variable length)
/// +-----------------------------------------------+
/// ```

use anyhow::{anyhow, Result};
use std::fmt;

/// HTTP/2 Frame Type (RFC 7540)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    Data = 0x0,
    Headers = 0x1,
    Priority = 0x2,
    RstStream = 0x3,
    Settings = 0x4,
    PushPromise = 0x5,
    Ping = 0x6,
    GoAway = 0x7,
    WindowUpdate = 0x8,
    Continuation = 0x9,
    /// Unknown frame type (for forward compatibility)
    Unknown(u8),
}

impl FrameType {
    /// Parse frame type from byte
    pub fn from_u8(ty: u8) -> Self {
        match ty {
            0x0 => FrameType::Data,
            0x1 => FrameType::Headers,
            0x2 => FrameType::Priority,
            0x3 => FrameType::RstStream,
            0x4 => FrameType::Settings,
            0x5 => FrameType::PushPromise,
            0x6 => FrameType::Ping,
            0x7 => FrameType::GoAway,
            0x8 => FrameType::WindowUpdate,
            0x9 => FrameType::Continuation,
            other => FrameType::Unknown(other),
        }
    }

    /// Convert to byte representation
    pub fn to_u8(&self) -> u8 {
        match self {
            FrameType::Data => 0x0,
            FrameType::Headers => 0x1,
            FrameType::Priority => 0x2,
            FrameType::RstStream => 0x3,
            FrameType::Settings => 0x4,
            FrameType::PushPromise => 0x5,
            FrameType::Ping => 0x6,
            FrameType::GoAway => 0x7,
            FrameType::WindowUpdate => 0x8,
            FrameType::Continuation => 0x9,
            FrameType::Unknown(n) => *n,
        }
    }

    /// Convert to string for logging
    pub fn as_str(&self) -> &'static str {
        match self {
            FrameType::Data => "DATA",
            FrameType::Headers => "HEADERS",
            FrameType::Priority => "PRIORITY",
            FrameType::RstStream => "RST_STREAM",
            FrameType::Settings => "SETTINGS",
            FrameType::PushPromise => "PUSH_PROMISE",
            FrameType::Ping => "PING",
            FrameType::GoAway => "GOAWAY",
            FrameType::WindowUpdate => "WINDOW_UPDATE",
            FrameType::Continuation => "CONTINUATION",
            FrameType::Unknown(_) => "UNKNOWN",
        }
    }
}

impl fmt::Display for FrameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameType::Unknown(n) => write!(f, "UNKNOWN({})", n),
            _ => write!(f, "{}", self.as_str()),
        }
    }
}

/// HTTP/2 Frame Flags (RFC 7540)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameFlags(u8);

impl FrameFlags {
    /// Create flags from byte
    pub fn new(byte: u8) -> Self {
        FrameFlags(byte)
    }

    /// END_STREAM flag (0x1) - final chunk of body
    pub fn end_stream(&self) -> bool {
        self.0 & 0x1 != 0
    }

    /// END_HEADERS flag (0x4) - headers complete (no CONTINUATION)
    pub fn end_headers(&self) -> bool {
        self.0 & 0x4 != 0
    }

    /// PADDED flag (0x8)
    pub fn padded(&self) -> bool {
        self.0 & 0x8 != 0
    }

    /// PRIORITY flag (0x20)
    pub fn priority(&self) -> bool {
        self.0 & 0x20 != 0
    }

    /// ACK flag (0x1 for SETTINGS/PING)
    pub fn ack(&self) -> bool {
        self.0 & 0x1 != 0
    }

    /// Raw flags byte
    pub fn raw(&self) -> u8 {
        self.0
    }
}

impl fmt::Display for FrameFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut flags = Vec::new();
        if self.end_stream() {
            flags.push("END_STREAM");
        }
        if self.end_headers() {
            flags.push("END_HEADERS");
        }
        if self.padded() {
            flags.push("PADDED");
        }
        if self.priority() {
            flags.push("PRIORITY");
        }
        if flags.is_empty() {
            write!(f, "none")
        } else {
            write!(f, "{}", flags.join("|"))
        }
    }
}

/// HTTP/2 Frame Header (9 bytes, no payload)
#[derive(Debug, Clone)]
pub struct Frame {
    /// Payload length in bytes (0-16,383 bytes for normal frames)
    pub length: u32,
    /// Frame type
    pub frame_type: FrameType,
    /// Frame flags
    pub flags: FrameFlags,
    /// Stream identifier (31-bit, 0 reserved for connection)
    pub stream_id: u32,
}

impl Frame {
    /// Parse HTTP/2 frame header from 9 bytes
    ///
    /// # Examples
    /// ```
    /// use scred_http::h2::Frame;
    ///
    /// // DATA frame (type=0), length=10, stream=1, no flags
    /// let header_bytes = [
    ///     0x00, 0x00, 0x0A,  // length: 10
    ///     0x00,              // type: DATA
    ///     0x00,              // flags: none
    ///     0x00, 0x00, 0x00, 0x01,  // stream_id: 1
    /// ];
    /// let frame = Frame::parse(&header_bytes).unwrap();
    /// assert_eq!(frame.length, 10);
    /// assert_eq!(frame.stream_id, 1);
    /// ```
    pub fn parse(header: &[u8]) -> Result<Self> {
        if header.len() < 9 {
            return Err(anyhow!("Frame header too short: {} bytes", header.len()));
        }

        // Parse 24-bit length (big-endian)
        let length = ((header[0] as u32) << 16)
            | ((header[1] as u32) << 8)
            | (header[2] as u32);

        // Parse type
        let frame_type = FrameType::from_u8(header[3]);

        // Parse flags
        let flags = FrameFlags::new(header[4]);

        // Parse stream ID (31-bit, skip reserved bit at position 0)
        let stream_id = (((header[5] as u32) & 0x7F) << 24)
            | ((header[6] as u32) << 16)
            | ((header[7] as u32) << 8)
            | (header[8] as u32);

        Ok(Frame {
            length,
            frame_type,
            flags,
            stream_id,
        })
    }

    /// Check if this is a HEADERS frame
    pub fn is_headers(&self) -> bool {
        matches!(self.frame_type, FrameType::Headers)
    }

    /// Check if this is a DATA frame
    pub fn is_data(&self) -> bool {
        matches!(self.frame_type, FrameType::Data)
    }

    /// Check if this is a CONTINUATION frame
    pub fn is_continuation(&self) -> bool {
        matches!(self.frame_type, FrameType::Continuation)
    }

    /// Check if END_STREAM flag is set
    pub fn is_end_stream(&self) -> bool {
        self.flags.end_stream()
    }

    /// Check if END_HEADERS flag is set
    pub fn is_end_headers(&self) -> bool {
        self.flags.end_headers()
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} length={} flags={} stream={}",
            self.frame_type, self.length, self.flags, self.stream_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_from_u8() {
        assert_eq!(FrameType::from_u8(0x0), FrameType::Data);
        assert_eq!(FrameType::from_u8(0x1), FrameType::Headers);
        assert_eq!(FrameType::from_u8(0x9), FrameType::Continuation);
        assert!(matches!(FrameType::from_u8(0xFF), FrameType::Unknown(0xFF)));
    }

    #[test]
    fn test_frame_flags_end_stream() {
        let flags = FrameFlags::new(0x1);
        assert!(flags.end_stream());
        assert!(!flags.end_headers());
    }

    #[test]
    fn test_frame_flags_end_headers() {
        let flags = FrameFlags::new(0x4);
        assert!(!flags.end_stream());
        assert!(flags.end_headers());
    }

    #[test]
    fn test_frame_flags_combined() {
        let flags = FrameFlags::new(0x5); // END_STREAM | END_HEADERS
        assert!(flags.end_stream());
        assert!(flags.end_headers());
    }

    #[test]
    fn test_parse_data_frame() {
        // DATA frame: length=10, type=0x0, flags=0x0, stream=1
        let header = [0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let frame = Frame::parse(&header).unwrap();

        assert_eq!(frame.length, 10);
        assert_eq!(frame.frame_type, FrameType::Data);
        assert_eq!(frame.stream_id, 1);
        assert!(!frame.is_end_stream());
    }

    #[test]
    fn test_parse_headers_with_end_stream() {
        // HEADERS frame: length=100, type=0x1, flags=0x5 (END_STREAM|END_HEADERS), stream=1
        let header = [0x00, 0x00, 0x64, 0x01, 0x05, 0x00, 0x00, 0x00, 0x01];
        let frame = Frame::parse(&header).unwrap();

        assert_eq!(frame.length, 100);
        assert_eq!(frame.frame_type, FrameType::Headers);
        assert!(frame.is_end_stream());
        assert!(frame.is_end_headers());
    }

    #[test]
    fn test_parse_with_large_stream_id() {
        // Stream ID = 0x7FFFFFFF (max 31-bit value)
        let header = [0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0xFF, 0xFF, 0xFF];
        let frame = Frame::parse(&header).unwrap();

        assert_eq!(frame.stream_id, 0x7FFFFFFF);
    }

    #[test]
    fn test_parse_too_short() {
        let header = [0x00, 0x00]; // Only 2 bytes
        assert!(Frame::parse(&header).is_err());
    }

    #[test]
    fn test_frame_display() {
        let header = [0x00, 0x00, 0x0A, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01];
        let frame = Frame::parse(&header).unwrap();
        let display = format!("{}", frame);
        assert!(display.contains("DATA"));
        assert!(display.contains("length=10"));
        assert!(display.contains("stream=1"));
    }
}
