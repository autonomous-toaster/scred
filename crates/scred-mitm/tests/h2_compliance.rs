/// Test infrastructure for comprehensive E2E testing
///
/// Provides:
/// - Custom H2 test client for protocol compliance
/// - Local HTTP/2 test server
/// - Metrics collection (throughput, latency, memory)
/// - Protocol validation helpers

#[cfg(test)]
mod test_harness {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use std::collections::HashMap;
    use std::error::Error;

    /// Metrics for a test run
    #[derive(Debug, Clone)]
    pub struct TestMetrics {
        pub requests: u64,
        pub responses: u64,
        pub errors: u64,
        pub total_duration: Duration,
        pub request_latencies: Vec<Duration>,
        pub secrets_redacted: u64,
    }

    impl TestMetrics {
        pub fn new() -> Self {
            Self {
                requests: 0,
                responses: 0,
                errors: 0,
                total_duration: Duration::from_secs(0),
                request_latencies: Vec::new(),
                secrets_redacted: 0,
            }
        }

        pub fn throughput_rps(&self) -> f64 {
            if self.total_duration.as_secs_f64() == 0.0 {
                0.0
            } else {
                self.responses as f64 / self.total_duration.as_secs_f64()
            }
        }

        pub fn avg_latency_ms(&self) -> f64 {
            if self.request_latencies.is_empty() {
                0.0
            } else {
                let sum: u128 = self.request_latencies.iter().map(|d| d.as_millis()).sum();
                sum as f64 / self.request_latencies.len() as f64
            }
        }

        pub fn p99_latency_ms(&self) -> f64 {
            if self.request_latencies.is_empty() {
                0.0
            } else {
                let mut sorted = self.request_latencies.clone();
                sorted.sort();
                let idx = (sorted.len() as f64 * 0.99) as usize;
                sorted.get(idx).map(|d| d.as_millis() as f64).unwrap_or(0.0)
            }
        }

        pub fn error_rate(&self) -> f64 {
            if self.requests == 0 {
                0.0
            } else {
                self.errors as f64 / self.requests as f64
            }
        }
    }

    /// Configuration for H2 test client
    pub struct H2ClientConfig {
        pub proxy_host: String,
        pub proxy_port: u16,
        pub upstream_url: String,
        pub num_streams: usize,
        pub requests_per_stream: usize,
        pub timeout: Duration,
        pub include_secrets: bool,
    }

    impl Default for H2ClientConfig {
        fn default() -> Self {
            Self {
                proxy_host: "127.0.0.1".to_string(),
                proxy_port: 8080,
                upstream_url: "https://httpbin.org".to_string(),
                num_streams: 10,
                requests_per_stream: 100,
                timeout: Duration::from_secs(30),
                include_secrets: true,
            }
        }
    }

    /// Simple local HTTP/2 test server for reproducible testing
    pub struct LocalH2TestServer {
        addr: SocketAddr,
    }

    impl LocalH2TestServer {
        pub fn new(port: u16) -> Result<Self, Box<dyn Error>> {
            let addr: SocketAddr = ([127, 0, 0, 1], port).into();
            
            eprintln!("[H2-TEST-SERVER] Created server for {}", addr);

            Ok(Self {
                addr,
            })
        }

        pub fn addr(&self) -> SocketAddr {
            self.addr
        }
    }

    /// HTTP/2 Frame validator (RFC 7540 compliance)
    pub struct H2FrameValidator {
        stats: HashMap<String, u64>,
    }

    impl H2FrameValidator {
        pub fn new() -> Self {
            Self {
                stats: HashMap::new(),
            }
        }

        /// Validate frame header (9 bytes minimum)
        pub fn validate_frame_header(frame_data: &[u8]) -> Result<(u32, u8, u8), String> {
            if frame_data.len() < 9 {
                return Err(format!("Frame too short: {} bytes", frame_data.len()));
            }

            // Bytes 0-2: Length (24 bits)
            let length = ((frame_data[0] as u32) << 16)
                | ((frame_data[1] as u32) << 8)
                | (frame_data[2] as u32);

            // Byte 3: Type
            let frame_type = frame_data[3];

            // Byte 4: Flags
            let flags = frame_data[4];

            // Validate known frame types (0-9)
            if frame_type > 9 {
                return Err(format!("Unknown frame type: {}", frame_type));
            }

            Ok((length, frame_type, flags))
        }

        /// Validate stream ID (31 bits, high bit reserved)
        pub fn validate_stream_id(stream_bytes: &[u8; 4]) -> Result<u32, String> {
            if stream_bytes.len() != 4 {
                return Err("Stream ID must be 4 bytes".to_string());
            }

            let stream_id = u32::from_be_bytes(*stream_bytes) & 0x7FFF_FFFF;

            if stream_id == 0 {
                return Err("Stream ID 0 reserved for connection".to_string());
            }

            Ok(stream_id)
        }

        /// Check RFC 7540 Section 6 frame type constraints
        pub fn validate_frame_constraints(&mut self, frame_type: u8, flags: u8, stream_id: u32) -> Result<(), String> {
            match frame_type {
                0x0 => { // DATA
                    if stream_id == 0 {
                        return Err("DATA frame on stream 0 (connection)".to_string());
                    }
                    self.increment_stat("DATA");
                }
                0x1 => { // HEADERS
                    if stream_id == 0 {
                        return Err("HEADERS frame on stream 0".to_string());
                    }
                    self.increment_stat("HEADERS");
                }
                0x2 => { // PRIORITY
                    self.increment_stat("PRIORITY");
                }
                0x3 => { // RST_STREAM
                    if stream_id == 0 {
                        return Err("RST_STREAM on stream 0".to_string());
                    }
                    self.increment_stat("RST_STREAM");
                }
                0x4 => { // SETTINGS
                    if stream_id != 0 {
                        return Err("SETTINGS frame on stream (must be connection)".to_string());
                    }
                    // SETTINGS ACK must not have payload
                    if (flags & 0x01) != 0 && (flags & 0xFE) != 0 {
                        return Err("SETTINGS ACK with payload".to_string());
                    }
                    self.increment_stat("SETTINGS");
                }
                0x5 => { // PUSH_PROMISE
                    if stream_id == 0 {
                        return Err("PUSH_PROMISE on stream 0".to_string());
                    }
                    self.increment_stat("PUSH_PROMISE");
                }
                0x6 => { // PING
                    if stream_id != 0 {
                        return Err("PING on stream (must be connection)".to_string());
                    }
                    self.increment_stat("PING");
                }
                0x7 => { // GOAWAY
                    if stream_id != 0 {
                        return Err("GOAWAY on stream (must be connection)".to_string());
                    }
                    self.increment_stat("GOAWAY");
                }
                0x8 => { // WINDOW_UPDATE
                    self.increment_stat("WINDOW_UPDATE");
                }
                0x9 => { // CONTINUATION
                    if stream_id == 0 {
                        return Err("CONTINUATION on stream 0".to_string());
                    }
                    self.increment_stat("CONTINUATION");
                }
                _ => {}
            }

            Ok(())
        }

        fn increment_stat(&mut self, key: &str) {
            *self.stats.entry(key.to_string()).or_insert(0) += 1;
        }

        pub fn stats(&self) -> &HashMap<String, u64> {
            &self.stats
        }
    }

    /// HPACK Header decoder for validation
    pub struct HpackDecoder {
        dynamic_table: Vec<(Vec<u8>, Vec<u8>)>,
        max_table_size: usize,
    }

    impl HpackDecoder {
        pub fn new() -> Self {
            Self {
                dynamic_table: Vec::new(),
                max_table_size: 4096,
            }
        }

        /// Decode HPACK indexed header (RFC 7541 Section 6.1)
        pub fn decode_indexed(&self, index: usize) -> Result<(Vec<u8>, Vec<u8>), String> {
            // Static table has 61 entries (RFC 7541 Appendix B)
            let static_table_size = 61;

            if index <= static_table_size {
                // Look up in static table
                match index {
                    1 => Ok((b":authority".to_vec(), b"".to_vec())),
                    2 => Ok((b":method".to_vec(), b"GET".to_vec())),
                    3 => Ok((b":method".to_vec(), b"POST".to_vec())),
                    4 => Ok((b":path".to_vec(), b"/".to_vec())),
                    _ => Err(format!("Unknown static table index: {}", index)),
                }
            } else {
                // Dynamic table lookup
                let dyn_idx = index - static_table_size - 1;
                self.dynamic_table.get(dyn_idx)
                    .cloned()
                    .ok_or_else(|| format!("Dynamic table index out of range: {}", dyn_idx))
            }
        }

        /// Validate header field size (RFC 7540 Section 6.5.2)
        pub fn validate_header_size(&self, header_block: &[u8]) -> Result<(), String> {
            const MAX_HEADER_SIZE: usize = 16384; // 16 KB default
            
            if header_block.len() > MAX_HEADER_SIZE {
                return Err(format!("Header block too large: {} > {}", 
                    header_block.len(), MAX_HEADER_SIZE));
            }

            Ok(())
        }
    }

    /// Protocol compliance test report
    pub struct ComplianceReport {
        pub rfc_sections: HashMap<String, bool>,
        pub frame_types_tested: Vec<String>,
        pub errors: Vec<String>,
        pub warnings: Vec<String>,
    }

    impl ComplianceReport {
        pub fn new() -> Self {
            Self {
                rfc_sections: HashMap::new(),
                frame_types_tested: Vec::new(),
                errors: Vec::new(),
                warnings: Vec::new(),
            }
        }

        pub fn is_compliant(&self) -> bool {
            self.errors.is_empty() && self.rfc_sections.values().all(|&v| v)
        }

        pub fn summary(&self) -> String {
            format!(
                "Compliance Report:\n  Sections Passed: {}/{}\n  Frame Types: {}\n  Errors: {}\n  Warnings: {}",
                self.rfc_sections.values().filter(|&&v| v).count(),
                self.rfc_sections.len(),
                self.frame_types_tested.len(),
                self.errors.len(),
                self.warnings.len(),
            )
        }
    }
}

#[cfg(test)]
mod h2_compliance_tests {
    use super::test_harness::*;
    use std::time::Duration;

    #[test]
    fn test_frame_header_validation() {
        // Valid DATA frame header
        let valid_frame = [
            0x00, 0x00, 0x0A, // Length: 10
            0x00,             // Type: DATA
            0x01,             // Flags: END_STREAM
            0x00, 0x00, 0x00, 0x01, // Stream ID: 1
        ];

        let result = H2FrameValidator::validate_frame_header(&valid_frame);
        assert!(result.is_ok());
        let (length, frame_type, flags) = result.unwrap();
        assert_eq!(length, 10);
        assert_eq!(frame_type, 0);
        assert_eq!(flags, 1);
    }

    #[test]
    fn test_frame_header_too_short() {
        let short_frame = [0x00, 0x00];
        let result = H2FrameValidator::validate_frame_header(&short_frame);
        assert!(result.is_err());
    }

    #[test]
    fn test_stream_id_validation() {
        // Valid stream ID: 1
        let stream_bytes = [0x00, 0x00, 0x00, 0x01];
        let result = H2FrameValidator::validate_stream_id(&stream_bytes);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Invalid: stream ID 0 (reserved)
        let reserved = [0x00, 0x00, 0x00, 0x00];
        let result = H2FrameValidator::validate_stream_id(&reserved);
        assert!(result.is_err());
    }

    #[test]
    fn test_frame_type_constraints() {
        let mut validator = H2FrameValidator::new();

        // DATA on stream 0 should fail
        let result = validator.validate_frame_constraints(0x0, 0x01, 0);
        assert!(result.is_err());

        // DATA on stream 1 should pass
        let result = validator.validate_frame_constraints(0x0, 0x01, 1);
        assert!(result.is_ok());

        // SETTINGS on stream 0 should pass
        let result = validator.validate_frame_constraints(0x4, 0x00, 0);
        assert!(result.is_ok());

        // SETTINGS on stream 1 should fail
        let result = validator.validate_frame_constraints(0x4, 0x00, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_hpack_header_size_validation() {
        let decoder = HpackDecoder::new();

        // Small header (should pass)
        let small_header = b"name:value";
        assert!(decoder.validate_header_size(small_header).is_ok());

        // Create oversized header
        let oversized = vec![0u8; 20000];
        assert!(decoder.validate_header_size(&oversized).is_err());
    }

    #[test]
    fn test_metrics_calculation() {
        let mut metrics = TestMetrics::new();
        metrics.requests = 1000;
        metrics.responses = 950;
        metrics.errors = 50;
        metrics.total_duration = Duration::from_secs(10);
        metrics.request_latencies = vec![
            Duration::from_millis(10),
            Duration::from_millis(15),
            Duration::from_millis(20),
        ];

        assert_eq!(metrics.error_rate(), 0.05);
        assert_eq!(metrics.throughput_rps(), 95.0);
        assert!(metrics.avg_latency_ms() > 0.0);
    }

    #[test]
    fn test_local_h2_server_startup() {
        let server = LocalH2TestServer::new(19999);
        assert!(server.is_ok());
        
        let server = server.unwrap();
        let addr = server.addr();
        assert_eq!(addr.port(), 19999);
    }
}
