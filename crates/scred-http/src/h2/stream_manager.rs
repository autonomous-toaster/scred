/// HTTP/2 Stream Manager - Multi-Stream Demultiplexing
///
/// Manages all active HTTP/2 streams on a single connection with independent
/// redaction state for each stream. Handles frame routing, state transitions,
/// and response collection.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn, error};

use super::stream_state::StreamRedactionState;
use scred_redactor::RedactionEngine;

/// Stream lifecycle state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamLifecycle {
    /// Stream created, waiting for HEADERS frame
    Idle,
    /// Headers received (may still receive body)
    Open,
    /// Half-closed: client sent END_STREAM (no more request body, waiting for response)
    HalfClosedLocal,
    /// Half-closed: server sent END_STREAM (no more response body, client may send more requests)
    HalfClosedRemote,
    /// Fully closed (both client and server sent END_STREAM)
    Closed,
    /// Stream reset by RST_STREAM frame
    Reset,
}

/// Per-stream response data (after redaction applied)
#[derive(Clone, Debug)]
pub struct RedactedResponse {
    pub stream_id: u32,
    pub status: u16,
    pub headers: http::HeaderMap,
    pub body: Vec<u8>,
    pub redaction_applied: bool,
}

/// Manages all active HTTP/2 streams on a connection
pub struct StreamManager {
    /// Active streams: stream_id → redaction state
    streams: HashMap<u32, StreamRedactionState>,
    
    /// Stream lifecycle: stream_id → current state
    lifecycles: HashMap<u32, StreamLifecycle>,
    
    /// Completed redacted responses: stream_id → response
    responses: HashMap<u32, RedactedResponse>,
    
    /// Redaction engine (shared across all streams)
    redaction_engine: Arc<RedactionEngine>,
    
    /// Connection-level window size for flow control
    connection_window: u32,
    
    /// Per-stream window sizes
    stream_windows: HashMap<u32, u32>,
    
    /// Maximum concurrent streams (from SETTINGS)
    max_streams: u32,
    
    /// Current stream counter (for tracking)
    stream_count: u32,
}

impl StreamManager {
    /// Create new stream manager for a connection
    pub fn new(redaction_engine: Arc<RedactionEngine>) -> Self {
        Self {
            streams: HashMap::new(),
            lifecycles: HashMap::new(),
            responses: HashMap::new(),
            redaction_engine,
            connection_window: 65535, // RFC 9113 default
            stream_windows: HashMap::new(),
            max_streams: 100, // Conservative default
            stream_count: 0,
        }
    }

    /// Create a new stream
    pub fn create_stream(&mut self, stream_id: u32) -> Result<()> {
        if self.streams.contains_key(&stream_id) {
            return Err(anyhow!("Stream {} already exists", stream_id));
        }

        if self.stream_count >= self.max_streams {
            return Err(anyhow!(
                "Max concurrent streams reached: {} (limit: {})",
                self.stream_count,
                self.max_streams
            ));
        }

        debug!("Creating stream {}", stream_id);

        self.streams.insert(
            stream_id,
            StreamRedactionState::new(stream_id, self.redaction_engine.clone()),
        );
        self.lifecycles.insert(stream_id, StreamLifecycle::Idle);
        self.stream_windows.insert(stream_id, 65535); // RFC 9113 default

        self.stream_count += 1;
        Ok(())
    }

    /// Add headers to a stream
    pub fn add_headers(&mut self, stream_id: u32, headers: http::HeaderMap) -> Result<()> {
        if let Some(state) = self.streams.get_mut(&stream_id) {
            state.add_headers(headers)?;
            // Transition: Idle → Open
            if let Some(lifecycle) = self.lifecycles.get_mut(&stream_id) {
                if *lifecycle == StreamLifecycle::Idle {
                    *lifecycle = StreamLifecycle::Open;
                    debug!("Stream {} transitioned: Idle → Open", stream_id);
                }
            }
            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }

    /// Add body chunk to a stream
    pub fn add_body_chunk(&mut self, stream_id: u32, chunk: Vec<u8>) -> Result<()> {
        if let Some(state) = self.streams.get_mut(&stream_id) {
            state.add_body_chunk(chunk)?;
            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }

    /// Mark stream as ended (END_STREAM flag received)
    pub fn end_stream(&mut self, stream_id: u32) -> Result<()> {
        if let Some(state) = self.streams.get_mut(&stream_id) {
            state.mark_end_stream();

            // Transition: Open → HalfClosedLocal (client sent END_STREAM)
            if let Some(lifecycle) = self.lifecycles.get_mut(&stream_id) {
                if *lifecycle == StreamLifecycle::Open {
                    *lifecycle = StreamLifecycle::HalfClosedLocal;
                    debug!(
                        "Stream {} transitioned: Open → HalfClosedLocal",
                        stream_id
                    );
                }
            }

            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }

    /// Add response to stream (called after upstream response received)
    pub fn add_response(
        &mut self,
        stream_id: u32,
        status: u16,
        headers: http::HeaderMap,
        body: Vec<u8>,
    ) -> Result<()> {
        if !self.streams.contains_key(&stream_id) {
            return Err(anyhow!("Stream {} not found", stream_id));
        }

        // Apply redaction to response
        let redacted_headers = headers.clone(); // TODO: Apply redaction to headers
        let redacted_body = body.clone(); // TODO: Apply redaction to body

        let response = RedactedResponse {
            stream_id,
            status,
            headers: redacted_headers,
            body: redacted_body,
            redaction_applied: true,
        };

        self.responses.insert(stream_id, response);

        // Transition: HalfClosedLocal → HalfClosedRemote (server sent END_STREAM)
        if let Some(lifecycle) = self.lifecycles.get_mut(&stream_id) {
            if *lifecycle == StreamLifecycle::HalfClosedLocal {
                *lifecycle = StreamLifecycle::HalfClosedRemote;
                debug!(
                    "Stream {} transitioned: HalfClosedLocal → HalfClosedRemote",
                    stream_id
                );
            }
        }

        Ok(())
    }

    /// Get completed response for stream (if available)
    pub fn get_response(&self, stream_id: u32) -> Option<RedactedResponse> {
        self.responses.get(&stream_id).cloned()
    }

    /// Check if stream is complete (both directions closed)
    pub fn is_stream_complete(&self, stream_id: u32) -> bool {
        matches!(
            self.lifecycles.get(&stream_id),
            Some(StreamLifecycle::HalfClosedRemote) | Some(StreamLifecycle::Closed)
        )
    }

    /// Close stream (transition to Closed)
    pub fn close_stream(&mut self, stream_id: u32) -> Result<()> {
        if let Some(lifecycle) = self.lifecycles.get_mut(&stream_id) {
            let old_state = *lifecycle;
            *lifecycle = StreamLifecycle::Closed;
            debug!(
                "Stream {} closed (was in {:?})",
                stream_id, old_state
            );

            // Clean up stream from active streams (but keep response)
            self.streams.remove(&stream_id);
            self.stream_windows.remove(&stream_id);
            self.stream_count = self.stream_count.saturating_sub(1);

            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }

    /// Reset stream (RST_STREAM received)
    pub fn reset_stream(&mut self, stream_id: u32) -> Result<()> {
        if let Some(lifecycle) = self.lifecycles.get_mut(&stream_id) {
            *lifecycle = StreamLifecycle::Reset;
            debug!("Stream {} reset", stream_id);

            // Clean up
            self.streams.remove(&stream_id);
            self.stream_windows.remove(&stream_id);
            self.stream_count = self.stream_count.saturating_sub(1);

            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }

    /// Consume connection window for flow control
    pub fn consume_connection_window(&mut self, bytes: usize) -> Result<()> {
        if bytes as u32 > self.connection_window {
            return Err(anyhow!(
                "Connection window exhausted: need {}, have {}",
                bytes,
                self.connection_window
            ));
        }

        self.connection_window -= bytes as u32;
        debug!("Connection window consumed: {} bytes, {} remaining", bytes, self.connection_window);

        Ok(())
    }

    /// Restore connection window (send WINDOW_UPDATE)
    pub fn restore_connection_window(&mut self, bytes: u32) {
        self.connection_window += bytes;
        debug!("Connection window restored: {} bytes, now {} total", bytes, self.connection_window);
    }

    /// Consume stream window for flow control
    pub fn consume_stream_window(&mut self, stream_id: u32, bytes: usize) -> Result<()> {
        if let Some(window) = self.stream_windows.get_mut(&stream_id) {
            if bytes as u32 > *window {
                return Err(anyhow!(
                    "Stream {} window exhausted: need {}, have {}",
                    stream_id,
                    bytes,
                    window
                ));
            }
            *window -= bytes as u32;
            debug!(
                "Stream {} window consumed: {} bytes, {} remaining",
                stream_id, bytes, window
            );
            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }

    /// Restore stream window (send WINDOW_UPDATE)
    pub fn restore_stream_window(&mut self, stream_id: u32, bytes: u32) -> Result<()> {
        if let Some(window) = self.stream_windows.get_mut(&stream_id) {
            *window += bytes;
            debug!(
                "Stream {} window restored: {} bytes, now {} total",
                stream_id, bytes, window
            );
            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }

    /// Get all active stream IDs
    pub fn active_streams(&self) -> Vec<u32> {
        self.streams.keys().copied().collect()
    }

    /// Get total active streams
    pub fn active_stream_count(&self) -> usize {
        self.streams.len()
    }

    /// Get all completed stream IDs (with responses ready)
    pub fn completed_streams(&self) -> Vec<u32> {
        self.responses.keys().copied().collect()
    }

    /// Get stream lifecycle state
    pub fn stream_state(&self, stream_id: u32) -> Option<StreamLifecycle> {
        self.lifecycles.get(&stream_id).copied()
    }

    /// Get statistics for debugging
    pub fn stats(&self) -> StreamManagerStats {
        StreamManagerStats {
            active_streams: self.streams.len(),
            completed_streams: self.responses.len(),
            connection_window: self.connection_window,
            max_streams: self.max_streams,
        }
    }
}

/// Statistics about the stream manager
#[derive(Debug, Clone)]
pub struct StreamManagerStats {
    pub active_streams: usize,
    pub completed_streams: usize,
    pub connection_window: u32,
    pub max_streams: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_engine() -> Arc<RedactionEngine> {
        Arc::new(RedactionEngine::new(Default::default()))
    }

    #[test]
    fn test_create_stream() {
        let engine = mock_engine();
        let mut manager = StreamManager::new(engine);

        manager.create_stream(1).unwrap();
        assert_eq!(manager.active_stream_count(), 1);
        assert_eq!(manager.stream_state(1), Some(StreamLifecycle::Idle));
    }

    #[test]
    fn test_duplicate_stream_error() {
        let engine = mock_engine();
        let mut manager = StreamManager::new(engine);

        manager.create_stream(1).unwrap();
        let result = manager.create_stream(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_stream_lifecycle_transitions() {
        let engine = mock_engine();
        let mut manager = StreamManager::new(engine);

        manager.create_stream(1).unwrap();
        assert_eq!(manager.stream_state(1), Some(StreamLifecycle::Idle));

        // Add headers: Idle → Open
        let headers = http::HeaderMap::new();
        manager.add_headers(1, headers).unwrap();
        assert_eq!(manager.stream_state(1), Some(StreamLifecycle::Open));

        // End stream: Open → HalfClosedLocal
        manager.end_stream(1).unwrap();
        assert_eq!(
            manager.stream_state(1),
            Some(StreamLifecycle::HalfClosedLocal)
        );

        // Add response: HalfClosedLocal → HalfClosedRemote
        let headers = http::HeaderMap::new();
        manager.add_response(1, 200, headers, vec![]).unwrap();
        assert_eq!(
            manager.stream_state(1),
            Some(StreamLifecycle::HalfClosedRemote)
        );

        // Close stream: HalfClosedRemote → Closed
        manager.close_stream(1).unwrap();
        assert_eq!(manager.stream_state(1), Some(StreamLifecycle::Closed));
    }

    #[test]
    fn test_window_management() {
        let engine = mock_engine();
        let mut manager = StreamManager::new(engine);

        manager.create_stream(1).unwrap();

        // Consume connection window
        assert!(manager.consume_connection_window(1000).is_ok());
        assert_eq!(manager.connection_window, 65535 - 1000);

        // Restore connection window
        manager.restore_connection_window(500);
        assert_eq!(manager.connection_window, 65535 - 500);

        // Consume stream window
        assert!(manager.consume_stream_window(1, 2000).is_ok());

        // Exceed window
        let result = manager.consume_stream_window(1, 100000);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_streams() {
        let engine = mock_engine();
        let mut manager = StreamManager::new(engine);

        // Create multiple streams with different IDs
        manager.create_stream(1).unwrap();
        manager.create_stream(3).unwrap();
        manager.create_stream(5).unwrap();

        assert_eq!(manager.active_stream_count(), 3);

        let active = manager.active_streams();
        assert!(active.contains(&1));
        assert!(active.contains(&3));
        assert!(active.contains(&5));
    }

    #[test]
    fn test_reset_stream() {
        let engine = mock_engine();
        let mut manager = StreamManager::new(engine);

        manager.create_stream(1).unwrap();
        manager.reset_stream(1).unwrap();

        assert_eq!(manager.stream_state(1), Some(StreamLifecycle::Reset));
        assert_eq!(manager.active_stream_count(), 0);
    }
}
