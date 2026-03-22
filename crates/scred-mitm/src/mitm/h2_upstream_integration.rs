/// Phase 3b: H2Multiplexer + UpstreamWiring Integration
///
/// Wires request/response flow through multiplexer and upstream connections.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::debug;

use scred_http::h2::{
    frame_encoder::FrameEncoder,
    hpack::HpackDecoder,
    upstream_wiring::UpstreamWiring,
};
use scred_redactor::RedactionEngine;

/// Enhanced H2Multiplexer with upstream integration
pub struct H2MultiplexerWithUpstream {
    /// Original multiplexer
    // pub multiplexer: H2Multiplexer,
    
    /// Upstream request/response wiring
    pub upstream_wiring: UpstreamWiring,
    
    /// HPACK header decoder for incoming requests
    pub hpack_decoder: HpackDecoder,
    
    /// Redaction engine for secrets
    pub redaction_engine: Arc<RedactionEngine>,
    
    /// Statistics
    pub total_requests_forwarded: u64,
    pub total_responses_sent: u64,
}

impl H2MultiplexerWithUpstream {
    pub fn new(redaction_engine: Arc<RedactionEngine>) -> Self {
        Self {
            upstream_wiring: UpstreamWiring::new(),
            hpack_decoder: HpackDecoder::new(),
            redaction_engine,
            total_requests_forwarded: 0,
            total_responses_sent: 0,
        }
    }

    /// Process incoming HEADERS frame from client
    /// 
    /// Flow:
    /// 1. Decode HPACK headers
    /// 2. Apply redaction to header values
    /// 3. Buffer for upstream forwarding
    pub async fn process_client_headers(
        &mut self,
        stream_id: u32,
        encoded_headers: &[u8],
    ) -> Result<HashMap<String, String>> {
        debug!("Processing client HEADERS for stream {}", stream_id);

        // Register stream for upstream forwarding
        self.upstream_wiring.register_stream(stream_id);

        // Decode HPACK headers
        let mut headers = self.hpack_decoder.decode(encoded_headers)?;

        // Apply redaction to header values
        for value in headers.values_mut() {
            // Redact sensitive values (passwords, tokens, api keys, etc.)
            let result = self.redaction_engine.redact(value);
            if !result.warnings.is_empty() {
                debug!("Redacted {} secrets in header value", result.warnings.len());
                *value = result.redacted;
            }
        }

        // Buffer headers for upstream forwarding
        self.upstream_wiring.buffer_request_headers(stream_id, headers.clone());

        Ok(headers)
    }

    /// Process incoming DATA frame from client
    ///
    /// Flow:
    /// 1. Apply redaction to body chunk
    /// 2. Buffer for upstream forwarding
    pub async fn process_client_data(
        &mut self,
        stream_id: u32,
        data: &[u8],
        end_stream: bool,
    ) -> Result<Vec<u8>> {
        debug!(
            "Processing client DATA for stream {}, {} bytes, end_stream={}",
            stream_id,
            data.len(),
            end_stream
        );

        // Apply redaction to body chunk (convert to string for redaction)
        let data_str = String::from_utf8_lossy(data);
        let result = self.redaction_engine.redact(&data_str);
        if !result.warnings.is_empty() {
            debug!("Redacted {} secrets in body chunk", result.warnings.len());
        }
        let redacted_data = result.redacted.into_bytes();

        // Buffer body for upstream forwarding
        self.upstream_wiring.buffer_request_body(stream_id, redacted_data.clone())?;

        // Mark request complete if END_STREAM received
        if end_stream {
            self.upstream_wiring.mark_request_complete(stream_id)?;
            debug!("Stream {} request complete, ready for forwarding", stream_id);
        }

        Ok(redacted_data)
    }

    /// Check if stream ready to forward to upstream
    pub fn is_ready_to_forward(&self, stream_id: u32) -> bool {
        self.upstream_wiring.is_request_ready(stream_id)
    }

    /// Get complete request for forwarding (headers + body)
    pub fn get_request_for_forwarding(
        &self,
        stream_id: u32,
    ) -> Result<(HashMap<String, String>, Vec<u8>)> {
        self.upstream_wiring.get_complete_request(stream_id)
    }

    /// Receive response from upstream and process
    ///
    /// Flow:
    /// 1. Apply redaction to response headers
    /// 2. Buffer response
    /// 3. Return encoded H2 frame ready to send to client
    pub async fn process_upstream_response_headers(
        &mut self,
        stream_id: u32,
        headers: HashMap<String, String>,
    ) -> Result<Vec<u8>> {
        debug!("Processing upstream response HEADERS for stream {}", stream_id);

        // Apply redaction to response header values
        let mut redacted_headers = headers.clone();
        for value in redacted_headers.values_mut() {
            let result = self.redaction_engine.redact(value);
            if !result.warnings.is_empty() {
                debug!("Redacted {} secrets in response header", result.warnings.len());
                *value = result.redacted;
            }
        }

        // Buffer response
        let encoded_headers = FrameEncoder::encode_headers_frame(stream_id, &redacted_headers, false)?;
        self.upstream_wiring
            .buffer_response_data(stream_id, encoded_headers.clone())?;

        Ok(encoded_headers)
    }

    /// Receive response body from upstream and process
    ///
    /// Flow:
    /// 1. Apply redaction to response body
    /// 2. Buffer response
    /// 3. Return encoded H2 frame ready to send to client
    pub async fn process_upstream_response_data(
        &mut self,
        stream_id: u32,
        data: &[u8],
        end_stream: bool,
    ) -> Result<Vec<u8>> {
        debug!(
            "Processing upstream response DATA for stream {}, {} bytes, end_stream={}",
            data.len(),
            stream_id,
            end_stream
        );

        // Apply redaction to response body
        let data_str = String::from_utf8_lossy(data);
        let result = self.redaction_engine.redact(&data_str);
        if !result.warnings.is_empty() {
            debug!("Redacted {} secrets in response body", result.warnings.len());
        }
        let redacted_data = result.redacted.into_bytes();

        // Mark response complete if END_STREAM
        if end_stream {
            self.upstream_wiring.mark_response_complete(stream_id)?;
            self.total_responses_sent += 1;
        }

        // Encode as H2 DATA frame
        let frame = FrameEncoder::encode_data_frame(stream_id, &redacted_data, end_stream)?;
        self.upstream_wiring
            .buffer_response_data(stream_id, frame.clone())?;

        Ok(frame)
    }

    /// Send response to client
    ///
    /// Writes encoded frames back to client connection
    pub async fn send_response_to_client<W: AsyncWriteExt + Unpin>(
        &mut self,
        stream_id: u32,
        writer: &mut W,
    ) -> Result<()> {
        debug!("Sending response for stream {} to client", stream_id);

        if !self.upstream_wiring.is_response_ready(stream_id) {
            return Err(anyhow!(
                "Response for stream {} not ready (incomplete)",
                stream_id
            ));
        }

        // Get complete response (already in H2 frame format)
        let response = self.upstream_wiring.get_complete_response(stream_id)?;

        // Send to client
        writer.write_all(&response).await?;
        writer.flush().await?;

        self.total_responses_sent += 1;

        // Clean up stream state
        self.upstream_wiring.cleanup_stream(stream_id);
        debug!("Response sent to client, stream {} cleaned up", stream_id);

        Ok(())
    }

    /// Statistics for monitoring
    pub fn stats(&self) -> IntegrationStats {
        let wiring_stats = self.upstream_wiring.stats();
        IntegrationStats {
            active_streams: wiring_stats.active_streams,
            buffered_requests: wiring_stats.buffered_requests,
            buffered_responses: wiring_stats.buffered_responses,
            total_requests_forwarded: wiring_stats.total_requests_forwarded,
            total_responses_sent: self.total_responses_sent,
        }
    }
}

/// Statistics for Phase 3 integration
#[derive(Debug, Clone)]
pub struct IntegrationStats {
    pub active_streams: u64,
    pub buffered_requests: u64,
    pub buffered_responses: u64,
    pub total_requests_forwarded: u64,
    pub total_responses_sent: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_h2_multiplexer_with_upstream_creation() {
        let config = scred_redactor::RedactionConfig { enabled: true };
        let redaction_engine = Arc::new(RedactionEngine::new(config));
        let multiplexer = H2MultiplexerWithUpstream::new(redaction_engine);

        assert_eq!(multiplexer.total_requests_forwarded, 0);
        assert_eq!(multiplexer.total_responses_sent, 0);
    }

    #[tokio::test]
    async fn test_process_client_headers() {
        let config = scred_redactor::RedactionConfig { enabled: true };
        let redaction_engine = Arc::new(RedactionEngine::new(config));
        let mut multiplexer = H2MultiplexerWithUpstream::new(redaction_engine);

        // Empty header block (valid, just no headers)
        let encoded_headers = vec![];

        let result = multiplexer
            .process_client_headers(1, &encoded_headers)
            .await;

        // Should succeed even with empty headers
        assert!(result.is_ok());
        
        // Should have an empty header map
        if let Ok(headers) = result {
            assert!(headers.is_empty());
        }
    }

    #[tokio::test]
    async fn test_is_ready_to_forward() {
        let config = scred_redactor::RedactionConfig { enabled: true };
        let redaction_engine = Arc::new(RedactionEngine::new(config));
        let mut multiplexer = H2MultiplexerWithUpstream::new(redaction_engine);

        // Stream not registered yet
        assert!(!multiplexer.is_ready_to_forward(1));

        // Register stream
        multiplexer.upstream_wiring.register_stream(1);
        assert!(!multiplexer.is_ready_to_forward(1)); // Still not ready without headers

        // Add headers
        let mut headers = HashMap::new();
        headers.insert(":method".to_string(), "GET".to_string());
        multiplexer
            .upstream_wiring
            .buffer_request_headers(1, headers);

        // Still not ready (need END_STREAM)
        assert!(!multiplexer.is_ready_to_forward(1));

        // Mark complete
        multiplexer.upstream_wiring.mark_request_complete(1).unwrap();
        assert!(multiplexer.is_ready_to_forward(1));
    }

    #[test]
    fn test_stats() {
        let config = scred_redactor::RedactionConfig { enabled: true };
        let redaction_engine = Arc::new(RedactionEngine::new(config));
        let multiplexer = H2MultiplexerWithUpstream::new(redaction_engine);

        let stats = multiplexer.stats();
        assert_eq!(stats.active_streams, 0);
        assert_eq!(stats.total_responses_sent, 0);
    }
}
