/// HTTP/2 Upstream Connection Wiring
///
/// Connects H2Multiplexer to actual upstream servers via UpstreamH2Pool.
/// Handles request forwarding and response demultiplexing.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

use crate::h2::frame::Frame;
use crate::h2::frame_encoder::FrameEncoder;
use crate::h2::stream_state::StreamRedactionState;
use scred_redactor::RedactionEngine;

/// Upstream request forwarding state
#[derive(Debug, Clone)]
pub struct ForwardingState {
    /// Original client stream ID
    pub client_stream_id: u32,
    /// Mapped upstream stream ID (may differ if pooling)
    pub upstream_stream_id: u32,
    /// Request complete (all client data received)
    pub request_complete: bool,
    /// Response complete (all upstream data received)
    pub response_complete: bool,
}

/// Upstream connection wiring coordinator
pub struct UpstreamWiring {
    /// Forwarding state for each stream
    forwarding_state: HashMap<u32, ForwardingState>,
    /// Request headers by stream (for forwarding)
    request_headers: HashMap<u32, HashMap<String, String>>,
    /// Request bodies by stream (buffered, to send to upstream)
    request_bodies: HashMap<u32, Vec<u8>>,
    /// Response buffers by stream (headers + body)
    response_buffers: HashMap<u32, Vec<u8>>,
    /// Statistics
    pub total_requests_forwarded: u64,
    pub total_responses_received: u64,
}

impl UpstreamWiring {
    pub fn new() -> Self {
        Self {
            forwarding_state: HashMap::new(),
            request_headers: HashMap::new(),
            request_bodies: HashMap::new(),
            response_buffers: HashMap::new(),
            total_requests_forwarded: 0,
            total_responses_received: 0,
        }
    }

    /// Register stream for forwarding
    pub fn register_stream(&mut self, stream_id: u32) {
        self.forwarding_state.insert(
            stream_id,
            ForwardingState {
                client_stream_id: stream_id,
                upstream_stream_id: stream_id, // For now, 1:1 mapping
                request_complete: false,
                response_complete: false,
            },
        );
        debug!("Registered stream {} for upstream forwarding", stream_id);
    }

    /// Buffer request headers
    pub fn buffer_request_headers(&mut self, stream_id: u32, headers: HashMap<String, String>) {
        self.request_headers.insert(stream_id, headers);
        debug!("Buffered request headers for stream {}", stream_id);
    }

    /// Buffer request body chunk
    pub fn buffer_request_body(&mut self, stream_id: u32, chunk: Vec<u8>) -> Result<()> {
        let body = self.request_bodies.entry(stream_id).or_insert_with(Vec::new);
        body.extend_from_slice(&chunk);
        debug!(
            "Buffered {} bytes for stream {}, total: {}",
            chunk.len(),
            stream_id,
            body.len()
        );
        Ok(())
    }

    /// Mark request complete (END_STREAM received)
    pub fn mark_request_complete(&mut self, stream_id: u32) -> Result<()> {
        if let Some(state) = self.forwarding_state.get_mut(&stream_id) {
            state.request_complete = true;
            debug!("Marked stream {} request as complete", stream_id);
        } else {
            warn!("Stream {} not registered for forwarding", stream_id);
        }
        Ok(())
    }

    /// Mark response complete (END_STREAM received from upstream)
    pub fn mark_response_complete(&mut self, stream_id: u32) -> Result<()> {
        if let Some(state) = self.forwarding_state.get_mut(&stream_id) {
            state.response_complete = true;
            debug!("Marked stream {} response as complete", stream_id);
        }
        Ok(())
    }

    /// Buffer response data
    pub fn buffer_response_data(&mut self, stream_id: u32, data: Vec<u8>) -> Result<()> {
        let buffer = self.response_buffers.entry(stream_id).or_insert_with(Vec::new);
        buffer.extend_from_slice(&data);
        debug!(
            "Buffered {} bytes response for stream {}, total: {}",
            data.len(),
            stream_id,
            buffer.len()
        );
        Ok(())
    }

    /// Get complete request (headers + body)
    pub fn get_complete_request(&self, stream_id: u32) -> Result<(HashMap<String, String>, Vec<u8>)> {
        let headers = self
            .request_headers
            .get(&stream_id)
            .ok_or_else(|| anyhow!("No headers for stream {}", stream_id))?
            .clone();

        let body = self
            .request_bodies
            .get(&stream_id)
            .cloned()
            .unwrap_or_default();

        Ok((headers, body))
    }

    /// Get complete response
    pub fn get_complete_response(&self, stream_id: u32) -> Result<Vec<u8>> {
        self.response_buffers
            .get(&stream_id)
            .cloned()
            .ok_or_else(|| anyhow!("No response buffer for stream {}", stream_id))
    }

    /// Check if stream is ready to forward (headers received)
    pub fn is_request_ready(&self, stream_id: u32) -> bool {
        self.request_headers.contains_key(&stream_id)
            && self
                .forwarding_state
                .get(&stream_id)
                .map(|s| s.request_complete)
                .unwrap_or(false)
    }

    /// Check if response is ready to send to client (complete)
    pub fn is_response_ready(&self, stream_id: u32) -> bool {
        self.response_buffers.contains_key(&stream_id)
            && self
                .forwarding_state
                .get(&stream_id)
                .map(|s| s.response_complete)
                .unwrap_or(false)
    }

    /// Get statistics
    pub fn stats(&self) -> UpstreamWiringStats {
        UpstreamWiringStats {
            active_streams: self.forwarding_state.len() as u64,
            buffered_requests: self.request_headers.len() as u64,
            buffered_responses: self.response_buffers.len() as u64,
            total_requests_forwarded: self.total_requests_forwarded,
            total_responses_received: self.total_responses_received,
        }
    }

    /// Clean up stream state (after response sent to client)
    pub fn cleanup_stream(&mut self, stream_id: u32) {
        self.forwarding_state.remove(&stream_id);
        self.request_headers.remove(&stream_id);
        self.request_bodies.remove(&stream_id);
        self.response_buffers.remove(&stream_id);
        debug!("Cleaned up stream {} state", stream_id);
    }
}

impl Default for UpstreamWiring {
    fn default() -> Self {
        Self::new()
    }
}

/// Upstream wiring statistics
#[derive(Debug, Clone)]
pub struct UpstreamWiringStats {
    pub active_streams: u64,
    pub buffered_requests: u64,
    pub buffered_responses: u64,
    pub total_requests_forwarded: u64,
    pub total_responses_received: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upstream_wiring_creation() {
        let wiring = UpstreamWiring::new();
        assert_eq!(wiring.forwarding_state.len(), 0);
        assert_eq!(wiring.total_requests_forwarded, 0);
    }

    #[test]
    fn test_register_stream() {
        let mut wiring = UpstreamWiring::new();
        wiring.register_stream(1);

        assert!(wiring.forwarding_state.contains_key(&1));
        let state = wiring.forwarding_state.get(&1).unwrap();
        assert_eq!(state.client_stream_id, 1);
        assert!(!state.request_complete);
    }

    #[test]
    fn test_buffer_request() {
        let mut wiring = UpstreamWiring::new();
        wiring.register_stream(1);

        let mut headers = HashMap::new();
        headers.insert(":method".to_string(), "GET".to_string());
        headers.insert(":path".to_string(), "/api".to_string());

        wiring.buffer_request_headers(1, headers.clone());
        wiring.buffer_request_body(1, b"test body".to_vec()).unwrap();

        let (stored_headers, body) = wiring.get_complete_request(1).unwrap();
        assert_eq!(stored_headers.get(":method"), Some(&"GET".to_string()));
        assert_eq!(body, b"test body");
    }

    #[test]
    fn test_mark_request_complete() {
        let mut wiring = UpstreamWiring::new();
        wiring.register_stream(1);

        assert!(!wiring
            .forwarding_state
            .get(&1)
            .unwrap()
            .request_complete);

        wiring.mark_request_complete(1).unwrap();

        assert!(wiring
            .forwarding_state
            .get(&1)
            .unwrap()
            .request_complete);
    }

    #[test]
    fn test_is_request_ready() {
        let mut wiring = UpstreamWiring::new();
        wiring.register_stream(1);

        assert!(!wiring.is_request_ready(1));

        let mut headers = HashMap::new();
        headers.insert(":method".to_string(), "GET".to_string());
        wiring.buffer_request_headers(1, headers);

        assert!(!wiring.is_request_ready(1)); // Still need END_STREAM

        wiring.mark_request_complete(1).unwrap();

        assert!(wiring.is_request_ready(1));
    }

    #[test]
    fn test_buffer_response() {
        let mut wiring = UpstreamWiring::new();
        wiring.register_stream(1);

        wiring
            .buffer_response_data(1, b"response body".to_vec())
            .unwrap();

        let response = wiring.get_complete_response(1).unwrap();
        assert_eq!(response, b"response body");
    }

    #[test]
    fn test_cleanup_stream() {
        let mut wiring = UpstreamWiring::new();
        wiring.register_stream(1);

        let mut headers = HashMap::new();
        headers.insert(":method".to_string(), "GET".to_string());
        wiring.buffer_request_headers(1, headers);
        wiring.buffer_response_data(1, b"response".to_vec()).unwrap();

        assert!(wiring.forwarding_state.contains_key(&1));

        wiring.cleanup_stream(1);

        assert!(!wiring.forwarding_state.contains_key(&1));
        assert!(!wiring.request_headers.contains_key(&1));
        assert!(!wiring.response_buffers.contains_key(&1));
    }

    #[test]
    fn test_statistics() {
        let mut wiring = UpstreamWiring::new();
        wiring.register_stream(1);
        wiring.register_stream(3);

        let mut headers = HashMap::new();
        headers.insert(":method".to_string(), "GET".to_string());
        wiring.buffer_request_headers(1, headers.clone());

        wiring.buffer_response_data(3, b"response".to_vec()).unwrap();

        let stats = wiring.stats();
        assert_eq!(stats.active_streams, 2);
        assert_eq!(stats.buffered_requests, 1);
        assert_eq!(stats.buffered_responses, 1);
    }
}
