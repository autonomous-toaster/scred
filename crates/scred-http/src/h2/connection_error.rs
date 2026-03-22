//! HTTP/2 Connection Error Handling
//! 
//! RFC 7540 Section 5.4.2 & 6.8: GOAWAY frames for connection errors

use std::collections::HashMap;

/// Connection error codes for GOAWAY
/// RFC 7540 Section 7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ConnectionErrorCode {
    /// No error
    NoError = 0x0,
    /// Protocol error
    ProtocolError = 0x1,
    /// Internal error
    InternalError = 0x2,
    /// Flow control error
    FlowControlError = 0x3,
    /// Settings timeout
    SettingsTimeout = 0x4,
    /// Stream closed error
    StreamClosedError = 0x5,
    /// Frame size error
    FrameSizeError = 0x6,
    /// Refused stream
    RefusedStreamError = 0x7,
    /// Cancel
    CancelError = 0x8,
    /// Compression error
    CompressionError = 0x9,
    /// Connection error
    ConnectionErrorCode = 0xa,
}

impl ConnectionErrorCode {
    /// Convert from u32
    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            0x0 => Some(ConnectionErrorCode::NoError),
            0x1 => Some(ConnectionErrorCode::ProtocolError),
            0x2 => Some(ConnectionErrorCode::InternalError),
            0x3 => Some(ConnectionErrorCode::FlowControlError),
            0x4 => Some(ConnectionErrorCode::SettingsTimeout),
            0x5 => Some(ConnectionErrorCode::StreamClosedError),
            0x6 => Some(ConnectionErrorCode::FrameSizeError),
            0x7 => Some(ConnectionErrorCode::RefusedStreamError),
            0x8 => Some(ConnectionErrorCode::CancelError),
            0x9 => Some(ConnectionErrorCode::CompressionError),
            0xa => Some(ConnectionErrorCode::ConnectionErrorCode),
            _ => None,
        }
    }

    /// Get as u32
    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            ConnectionErrorCode::NoError => "NO_ERROR",
            ConnectionErrorCode::ProtocolError => "PROTOCOL_ERROR",
            ConnectionErrorCode::InternalError => "INTERNAL_ERROR",
            ConnectionErrorCode::FlowControlError => "FLOW_CONTROL_ERROR",
            ConnectionErrorCode::SettingsTimeout => "SETTINGS_TIMEOUT",
            ConnectionErrorCode::StreamClosedError => "STREAM_CLOSED_ERROR",
            ConnectionErrorCode::FrameSizeError => "FRAME_SIZE_ERROR",
            ConnectionErrorCode::RefusedStreamError => "REFUSED_STREAM_ERROR",
            ConnectionErrorCode::CancelError => "CANCEL_ERROR",
            ConnectionErrorCode::CompressionError => "COMPRESSION_ERROR",
            ConnectionErrorCode::ConnectionErrorCode => "CONNECTION_ERROR",
        }
    }
}

/// GOAWAY frame information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAwayFrame {
    /// Last stream ID
    pub last_stream_id: u32,
    /// Error code
    pub error_code: ConnectionErrorCode,
    /// Additional debug data
    pub debug_data: Vec<u8>,
}

impl GoAwayFrame {
    /// Create a new GOAWAY frame
    pub fn new(last_stream_id: u32, error_code: ConnectionErrorCode) -> Self {
        GoAwayFrame {
            last_stream_id,
            error_code,
            debug_data: Vec::new(),
        }
    }

    /// Create with debug data
    pub fn with_debug(
        last_stream_id: u32,
        error_code: ConnectionErrorCode,
        debug_data: Vec<u8>,
    ) -> Result<Self, String> {
        // RFC 7540: additional data must not exceed max frame size (16KB)
        if debug_data.len() > 16384 {
            return Err("Debug data too large".to_string());
        }

        Ok(GoAwayFrame {
            last_stream_id,
            error_code,
            debug_data,
        })
    }

    /// Encode to bytes
    /// Format: 4 bytes last_stream_id + 4 bytes error_code + debug_data
    pub fn encode(&self) -> Vec<u8> {
        let mut payload = Vec::new();

        // Last stream ID (4 bytes, MSB is reserved)
        payload.extend_from_slice(&(self.last_stream_id & 0x7FFFFFFF).to_be_bytes());

        // Error code (4 bytes)
        payload.extend_from_slice(&self.error_code.as_u32().to_be_bytes());

        // Debug data
        payload.extend_from_slice(&self.debug_data);

        payload
    }

    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 8 {
            return Err("GOAWAY frame too short (min 8 bytes)".to_string());
        }

        let last_stream_id =
            u32::from_be_bytes([data[0], data[1], data[2], data[3]]) & 0x7FFFFFFF;

        let error_code_raw = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        let error_code = ConnectionErrorCode::from_u32(error_code_raw)
            .ok_or_else(|| format!("Unknown error code: 0x{:x}", error_code_raw))?;

        let debug_data = if data.len() > 8 {
            data[8..].to_vec()
        } else {
            Vec::new()
        };

        Ok(GoAwayFrame {
            last_stream_id,
            error_code,
            debug_data,
        })
    }
}

/// Connection error event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionError {
    /// Error code
    pub error_code: ConnectionErrorCode,
    /// Description
    pub description: String,
    /// Stream that triggered the error (if applicable)
    pub stream_id: Option<u32>,
}

impl ConnectionError {
    /// Create a new connection error
    pub fn new(error_code: ConnectionErrorCode, description: String) -> Self {
        ConnectionError {
            error_code,
            description,
            stream_id: None,
        }
    }

    /// Create with stream context
    pub fn with_stream(
        error_code: ConnectionErrorCode,
        description: String,
        stream_id: u32,
    ) -> Self {
        ConnectionError {
            error_code,
            description,
            stream_id: Some(stream_id),
        }
    }
}

/// Connection error manager
pub struct ConnectionErrorManager {
    /// Current connection state
    pub state: ConnectionState,
    /// Recorded connection errors
    errors: Vec<ConnectionError>,
    /// Last error timestamp
    last_error_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is open and healthy
    Open,
    /// Connection has received GOAWAY
    GoAwayReceived,
    /// Connection should send GOAWAY
    GoAwaySend,
    /// Connection is closed
    Closed,
}

impl ConnectionErrorManager {
    /// Create a new error manager
    pub fn new() -> Self {
        ConnectionErrorManager {
            state: ConnectionState::Open,
            errors: Vec::new(),
            last_error_time: 0,
        }
    }

    /// Record a connection error
    pub fn record_error(&mut self, error: ConnectionError) {
        self.errors.push(error);
        self.last_error_time = 0; // Would be set with actual timestamp
    }

    /// Get all errors
    pub fn get_errors(&self) -> &[ConnectionError] {
        &self.errors
    }

    /// Get last error
    pub fn get_last_error(&self) -> Option<&ConnectionError> {
        self.errors.last()
    }

    /// Clear errors
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    /// Set connection state
    pub fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }

    /// Check if connection is open
    pub fn is_open(&self) -> bool {
        self.state == ConnectionState::Open
    }

    /// Check if connection is closed
    pub fn is_closed(&self) -> bool {
        self.state == ConnectionState::Closed
    }

    /// Error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get error summary
    pub fn summary(&self) -> HashMap<String, usize> {
        let mut summary = HashMap::new();
        for error in &self.errors {
            *summary.entry(error.error_code.name().to_string()).or_insert(0) += 1;
        }
        summary
    }
}

impl Default for ConnectionErrorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_error_code_conversion() {
        assert_eq!(
            ConnectionErrorCode::from_u32(0x0),
            Some(ConnectionErrorCode::NoError)
        );
        assert_eq!(
            ConnectionErrorCode::from_u32(0x4),
            Some(ConnectionErrorCode::SettingsTimeout)
        );
    }

    #[test]
    fn test_connection_error_code_name() {
        assert_eq!(
            ConnectionErrorCode::ProtocolError.name(),
            "PROTOCOL_ERROR"
        );
        assert_eq!(
            ConnectionErrorCode::SettingsTimeout.name(),
            "SETTINGS_TIMEOUT"
        );
    }

    #[test]
    fn test_goaway_frame_encode() {
        let goaway = GoAwayFrame::new(100, ConnectionErrorCode::ProtocolError);
        let payload = goaway.encode();

        assert!(payload.len() >= 8);
        // Last stream ID should be 100
        assert_eq!(
            u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]),
            100
        );
        // Error code should be 1
        assert_eq!(
            u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
            1
        );
    }

    #[test]
    fn test_goaway_frame_parse() {
        let mut data = vec![0x00, 0x00, 0x00, 0x64]; // last_stream_id = 100
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // error_code = 1

        let goaway = GoAwayFrame::parse(&data).unwrap();
        assert_eq!(goaway.last_stream_id, 100);
        assert_eq!(goaway.error_code, ConnectionErrorCode::ProtocolError);
    }

    #[test]
    fn test_goaway_frame_with_debug_data() {
        let debug = b"Connection closed due to timeout".to_vec();
        let goaway =
            GoAwayFrame::with_debug(50, ConnectionErrorCode::SettingsTimeout, debug.clone())
                .unwrap();

        assert_eq!(goaway.debug_data, debug);
        let payload = goaway.encode();
        assert!(payload.len() > 8);
    }

    #[test]
    fn test_goaway_frame_debug_data_too_large() {
        let debug = vec![0u8; 20000]; // Too large
        let result = GoAwayFrame::with_debug(50, ConnectionErrorCode::NoError, debug);
        assert!(result.is_err());
    }

    #[test]
    fn test_goaway_frame_parse_too_short() {
        let data = vec![0x00, 0x00, 0x00]; // Only 3 bytes
        let result = GoAwayFrame::parse(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_connection_error_creation() {
        let error =
            ConnectionError::new(ConnectionErrorCode::ProtocolError, "Invalid frame".to_string());

        assert_eq!(error.error_code, ConnectionErrorCode::ProtocolError);
        assert_eq!(error.description, "Invalid frame");
        assert_eq!(error.stream_id, None);
    }

    #[test]
    fn test_connection_error_with_stream() {
        let error = ConnectionError::with_stream(
            ConnectionErrorCode::FlowControlError,
            "Window exceeded".to_string(),
            5,
        );

        assert_eq!(error.stream_id, Some(5));
    }

    #[test]
    fn test_error_manager_record() {
        let mut manager = ConnectionErrorManager::new();
        let error = ConnectionError::new(
            ConnectionErrorCode::ProtocolError,
            "Test error".to_string(),
        );

        manager.record_error(error);
        assert_eq!(manager.error_count(), 1);
    }

    #[test]
    fn test_error_manager_get_last() {
        let mut manager = ConnectionErrorManager::new();

        let error1 = ConnectionError::new(
            ConnectionErrorCode::ProtocolError,
            "Error 1".to_string(),
        );
        let error2 = ConnectionError::new(ConnectionErrorCode::InternalError, "Error 2".to_string());

        manager.record_error(error1);
        manager.record_error(error2);

        let last = manager.get_last_error().unwrap();
        assert_eq!(last.error_code, ConnectionErrorCode::InternalError);
    }

    #[test]
    fn test_error_manager_state() {
        let mut manager = ConnectionErrorManager::new();

        assert!(manager.is_open());
        assert!(!manager.is_closed());

        manager.set_state(ConnectionState::Closed);
        assert!(!manager.is_open());
        assert!(manager.is_closed());
    }

    #[test]
    fn test_error_manager_summary() {
        let mut manager = ConnectionErrorManager::new();

        manager.record_error(ConnectionError::new(
            ConnectionErrorCode::ProtocolError,
            "Error 1".to_string(),
        ));
        manager.record_error(ConnectionError::new(
            ConnectionErrorCode::ProtocolError,
            "Error 2".to_string(),
        ));
        manager.record_error(ConnectionError::new(
            ConnectionErrorCode::InternalError,
            "Error 3".to_string(),
        ));

        let summary = manager.summary();
        assert_eq!(summary.get("PROTOCOL_ERROR"), Some(&2));
        assert_eq!(summary.get("INTERNAL_ERROR"), Some(&1));
    }

    #[test]
    fn test_error_manager_clear() {
        let mut manager = ConnectionErrorManager::new();

        manager.record_error(ConnectionError::new(
            ConnectionErrorCode::ProtocolError,
            "Test".to_string(),
        ));

        assert_eq!(manager.error_count(), 1);
        manager.clear_errors();
        assert_eq!(manager.error_count(), 0);
    }

    #[test]
    fn test_goaway_encode_decode_roundtrip() {
        let original = GoAwayFrame::new(200, ConnectionErrorCode::FlowControlError);
        let payload = original.encode();
        let decoded = GoAwayFrame::parse(&payload).unwrap();

        assert_eq!(decoded.last_stream_id, original.last_stream_id);
        assert_eq!(decoded.error_code, original.error_code);
    }
}
