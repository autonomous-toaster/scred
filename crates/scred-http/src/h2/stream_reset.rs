//! HTTP/2 Stream Reset (RST_STREAM) Support
//! 
//! RFC 7540 Section 5.4 & 6.4: RST_STREAM frame type 0x03

use std::collections::HashMap;

/// Error codes for RST_STREAM
/// RFC 7540 Section 7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ErrorCode {
    /// No error
    NoError = 0x0,
    /// Protocol error detected
    ProtocolError = 0x1,
    /// Implementation fault
    InternalError = 0x2,
    /// Flow control error
    FlowControlError = 0x3,
    /// Frame received when stream is closed
    StreamClosedError = 0x5,
    /// Frame size error
    FrameSizeError = 0x6,
    /// Stream refused (refused to process stream before sending response headers)
    RefusedStreamError = 0x7,
    /// Stream cancelled
    CancelError = 0x8,
    /// Compression state error
    CompressionError = 0x9,
    /// Connection error
    ConnectionError = 0xa,
}

impl ErrorCode {
    /// Convert from u32 to ErrorCode
    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            0x0 => Some(ErrorCode::NoError),
            0x1 => Some(ErrorCode::ProtocolError),
            0x2 => Some(ErrorCode::InternalError),
            0x3 => Some(ErrorCode::FlowControlError),
            0x5 => Some(ErrorCode::StreamClosedError),
            0x6 => Some(ErrorCode::FrameSizeError),
            0x7 => Some(ErrorCode::RefusedStreamError),
            0x8 => Some(ErrorCode::CancelError),
            0x9 => Some(ErrorCode::CompressionError),
            0xa => Some(ErrorCode::ConnectionError),
            _ => None,
        }
    }

    /// Get error code as u32
    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    /// Get human-readable error name
    pub fn name(&self) -> &'static str {
        match self {
            ErrorCode::NoError => "NO_ERROR",
            ErrorCode::ProtocolError => "PROTOCOL_ERROR",
            ErrorCode::InternalError => "INTERNAL_ERROR",
            ErrorCode::FlowControlError => "FLOW_CONTROL_ERROR",
            ErrorCode::StreamClosedError => "STREAM_CLOSED_ERROR",
            ErrorCode::FrameSizeError => "FRAME_SIZE_ERROR",
            ErrorCode::RefusedStreamError => "REFUSED_STREAM_ERROR",
            ErrorCode::CancelError => "CANCEL_ERROR",
            ErrorCode::CompressionError => "COMPRESSION_ERROR",
            ErrorCode::ConnectionError => "CONNECTION_ERROR",
        }
    }
}

/// Stream reset information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamReset {
    /// Stream ID that was reset
    pub stream_id: u32,
    /// Error code
    pub error_code: ErrorCode,
    /// Timestamp when reset occurred
    pub reset_time: u64,
}

impl StreamReset {
    /// Create a new stream reset
    pub fn new(stream_id: u32, error_code: ErrorCode) -> Self {
        StreamReset {
            stream_id,
            error_code,
            reset_time: 0, // Would be set by caller with actual timestamp
        }
    }

    /// Create with timestamp
    pub fn with_time(stream_id: u32, error_code: ErrorCode, time: u64) -> Self {
        StreamReset {
            stream_id,
            error_code,
            reset_time: time,
        }
    }
}

/// RST_STREAM frame handler
pub struct RstStreamFrame;

impl RstStreamFrame {
    /// Frame type for RST_STREAM (0x03)
    pub const FRAME_TYPE: u8 = 0x03;
    /// RST_STREAM payload size (always 4 bytes)
    pub const PAYLOAD_SIZE: usize = 4;

    /// Parse RST_STREAM frame payload
    /// 
    /// RFC 7540 Section 6.4: RST_STREAM
    /// Payload is exactly 4 bytes containing the error code (32-bit unsigned)
    pub fn parse(payload: &[u8]) -> Result<ErrorCode, String> {
        if payload.len() != Self::PAYLOAD_SIZE {
            return Err(format!(
                "RST_STREAM payload size invalid: {} bytes (expected: {})",
                payload.len(),
                Self::PAYLOAD_SIZE
            ));
        }

        let error_code_raw = u32::from_be_bytes([
            payload[0],
            payload[1],
            payload[2],
            payload[3],
        ]);

        ErrorCode::from_u32(error_code_raw)
            .ok_or_else(|| format!("Unknown error code: 0x{:x}", error_code_raw))
    }

    /// Encode RST_STREAM frame payload
    /// 
    /// Returns 4-byte payload
    pub fn encode(error_code: ErrorCode) -> Vec<u8> {
        error_code.as_u32().to_be_bytes().to_vec()
    }

    /// Validate RST_STREAM for a given stream
    pub fn validate(stream_id: u32) -> Result<(), String> {
        // Stream ID must not be 0 (connection stream)
        if stream_id == 0 {
            return Err("RST_STREAM cannot be sent on stream 0 (connection)".to_string());
        }

        Ok(())
    }
}

/// Stream reset manager
pub struct StreamResetManager {
    /// Mapping of stream ID to reset information
    resets: HashMap<u32, StreamReset>,
    /// Total resets tracked
    total_resets: u64,
}

impl StreamResetManager {
    /// Create a new reset manager
    pub fn new() -> Self {
        StreamResetManager {
            resets: HashMap::new(),
            total_resets: 0,
        }
    }

    /// Record a stream reset
    pub fn record_reset(&mut self, reset: StreamReset) -> Result<(), String> {
        if self.resets.contains_key(&reset.stream_id) {
            return Err(format!("Stream {} already reset", reset.stream_id));
        }

        self.resets.insert(reset.stream_id, reset);
        self.total_resets += 1;
        Ok(())
    }

    /// Get reset information for a stream
    pub fn get_reset(&self, stream_id: u32) -> Option<&StreamReset> {
        self.resets.get(&stream_id)
    }

    /// Check if a stream was reset
    pub fn is_reset(&self, stream_id: u32) -> bool {
        self.resets.contains_key(&stream_id)
    }

    /// Get all reset streams
    pub fn get_all_resets(&self) -> Vec<u32> {
        self.resets.keys().copied().collect()
    }

    /// Get reset statistics
    pub fn stats(&self) -> StreamResetStats {
        let mut by_error: HashMap<ErrorCode, usize> = HashMap::new();

        for reset in self.resets.values() {
            *by_error.entry(reset.error_code).or_insert(0) += 1;
        }

        StreamResetStats {
            total_resets: self.total_resets,
            reset_streams: self.resets.len(),
            by_error_code: by_error,
        }
    }

    /// Clear all reset records
    pub fn clear(&mut self) {
        self.resets.clear();
        self.total_resets = 0;
    }

    /// Get reset count
    pub fn reset_count(&self) -> usize {
        self.resets.len()
    }
}

impl Default for StreamResetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Reset statistics
#[derive(Debug, Clone)]
pub struct StreamResetStats {
    /// Total resets recorded
    pub total_resets: u64,
    /// Number of unique streams reset
    pub reset_streams: usize,
    /// Distribution by error code
    pub by_error_code: HashMap<ErrorCode, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_conversion() {
        assert_eq!(ErrorCode::from_u32(0x0), Some(ErrorCode::NoError));
        assert_eq!(ErrorCode::from_u32(0x1), Some(ErrorCode::ProtocolError));
        assert_eq!(ErrorCode::from_u32(0x3), Some(ErrorCode::FlowControlError));
        assert_eq!(ErrorCode::from_u32(0x8), Some(ErrorCode::CancelError));
    }

    #[test]
    fn test_error_code_unknown() {
        assert_eq!(ErrorCode::from_u32(0xFF), None);
        assert_eq!(ErrorCode::from_u32(0x999), None);
    }

    #[test]
    fn test_error_code_as_u32() {
        assert_eq!(ErrorCode::NoError.as_u32(), 0x0);
        assert_eq!(ErrorCode::ProtocolError.as_u32(), 0x1);
        assert_eq!(ErrorCode::FlowControlError.as_u32(), 0x3);
    }

    #[test]
    fn test_error_code_name() {
        assert_eq!(ErrorCode::NoError.name(), "NO_ERROR");
        assert_eq!(ErrorCode::ProtocolError.name(), "PROTOCOL_ERROR");
        assert_eq!(ErrorCode::FlowControlError.name(), "FLOW_CONTROL_ERROR");
    }

    #[test]
    fn test_rst_stream_encode() {
        let payload = RstStreamFrame::encode(ErrorCode::ProtocolError);
        assert_eq!(payload.len(), 4);
        assert_eq!(payload, vec![0x00, 0x00, 0x00, 0x01]);
    }

    #[test]
    fn test_rst_stream_parse() {
        let payload = vec![0x00, 0x00, 0x00, 0x01];
        let error_code = RstStreamFrame::parse(&payload).unwrap();
        assert_eq!(error_code, ErrorCode::ProtocolError);
    }

    #[test]
    fn test_rst_stream_parse_cancel() {
        let payload = vec![0x00, 0x00, 0x00, 0x08];
        let error_code = RstStreamFrame::parse(&payload).unwrap();
        assert_eq!(error_code, ErrorCode::CancelError);
    }

    #[test]
    fn test_rst_stream_parse_invalid_size() {
        let payload = vec![0x00, 0x00, 0x01]; // Only 3 bytes
        let result = RstStreamFrame::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_rst_stream_parse_unknown_code() {
        let payload = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let result = RstStreamFrame::parse(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_rst_stream_validate_valid() {
        let result = RstStreamFrame::validate(1);
        assert!(result.is_ok());

        let result = RstStreamFrame::validate(100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rst_stream_validate_stream_zero() {
        let result = RstStreamFrame::validate(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_stream_reset_creation() {
        let reset = StreamReset::new(1, ErrorCode::ProtocolError);
        assert_eq!(reset.stream_id, 1);
        assert_eq!(reset.error_code, ErrorCode::ProtocolError);
    }

    #[test]
    fn test_stream_reset_with_time() {
        let reset = StreamReset::with_time(5, ErrorCode::CancelError, 12345);
        assert_eq!(reset.stream_id, 5);
        assert_eq!(reset.error_code, ErrorCode::CancelError);
        assert_eq!(reset.reset_time, 12345);
    }

    #[test]
    fn test_reset_manager_record() {
        let mut manager = StreamResetManager::new();
        let reset = StreamReset::new(1, ErrorCode::ProtocolError);

        let result = manager.record_reset(reset);
        assert!(result.is_ok());
        assert_eq!(manager.reset_count(), 1);
    }

    #[test]
    fn test_reset_manager_duplicate() {
        let mut manager = StreamResetManager::new();
        let reset1 = StreamReset::new(1, ErrorCode::ProtocolError);
        let reset2 = StreamReset::new(1, ErrorCode::CancelError);

        let _ = manager.record_reset(reset1);
        let result = manager.record_reset(reset2);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset_manager_get() {
        let mut manager = StreamResetManager::new();
        let reset = StreamReset::new(3, ErrorCode::FlowControlError);

        let _ = manager.record_reset(reset);
        let retrieved = manager.get_reset(3).unwrap();
        assert_eq!(retrieved.stream_id, 3);
        assert_eq!(retrieved.error_code, ErrorCode::FlowControlError);
    }

    #[test]
    fn test_reset_manager_is_reset() {
        let mut manager = StreamResetManager::new();
        let reset = StreamReset::new(5, ErrorCode::CancelError);

        assert!(!manager.is_reset(5));
        let _ = manager.record_reset(reset);
        assert!(manager.is_reset(5));
    }

    #[test]
    fn test_reset_manager_get_all() {
        let mut manager = StreamResetManager::new();

        let _ = manager.record_reset(StreamReset::new(1, ErrorCode::ProtocolError));
        let _ = manager.record_reset(StreamReset::new(3, ErrorCode::CancelError));
        let _ = manager.record_reset(StreamReset::new(5, ErrorCode::FlowControlError));

        let resets = manager.get_all_resets();
        assert_eq!(resets.len(), 3);
        assert!(resets.contains(&1));
        assert!(resets.contains(&3));
        assert!(resets.contains(&5));
    }

    #[test]
    fn test_reset_manager_stats() {
        let mut manager = StreamResetManager::new();

        let _ = manager.record_reset(StreamReset::new(1, ErrorCode::ProtocolError));
        let _ = manager.record_reset(StreamReset::new(3, ErrorCode::ProtocolError));
        let _ = manager.record_reset(StreamReset::new(5, ErrorCode::CancelError));

        let stats = manager.stats();
        assert_eq!(stats.total_resets, 3);
        assert_eq!(stats.reset_streams, 3);
        assert_eq!(stats.by_error_code.get(&ErrorCode::ProtocolError), Some(&2));
        assert_eq!(stats.by_error_code.get(&ErrorCode::CancelError), Some(&1));
    }

    #[test]
    fn test_reset_manager_clear() {
        let mut manager = StreamResetManager::new();

        let _ = manager.record_reset(StreamReset::new(1, ErrorCode::ProtocolError));
        assert_eq!(manager.reset_count(), 1);

        manager.clear();
        assert_eq!(manager.reset_count(), 0);
    }

    #[test]
    fn test_rst_stream_encode_decode_roundtrip() {
        let original = ErrorCode::FlowControlError;
        let payload = RstStreamFrame::encode(original);
        let decoded = RstStreamFrame::parse(&payload).unwrap();

        assert_eq!(decoded, original);
    }
}
