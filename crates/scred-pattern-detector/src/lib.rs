/// Rust FFI wrapper for Zig pattern detector with streaming support
use std::ffi::c_char;
use std::os::raw::c_int;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct DetectionEvent {
    pub pattern_id: u16,
    pub pattern_name: [c_char; 64],
    pub name_len: u8,
    pub position: usize,
    pub length: u16,
}

/// PatternType classification - based on PERFORMANCE characteristics
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Fast prefix-only matching (O(1) per pattern)
    FastPrefix = 0,
    /// Structured format validation (e.g., JWT parsing)
    StructuredFormat = 1,
    /// Full regex-based matching (O(n) with backtracking)
    RegexBased = 2,
}

impl PatternType {
    pub fn name(&self) -> &'static str {
        match self {
            PatternType::FastPrefix => "FastPrefix",
            PatternType::StructuredFormat => "StructuredFormat",
            PatternType::RegexBased => "RegexBased",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PatternType::FastPrefix => "Fast prefix-only matching (<5ms for all)",
            PatternType::StructuredFormat => "Structured format validation (JWT, etc.)",
            PatternType::RegexBased => "Full regex matching (~1000ms for all)",
        }
    }

    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(PatternType::FastPrefix),
            1 => Some(PatternType::StructuredFormat),
            2 => Some(PatternType::RegexBased),
            _ => None,
        }
    }
}

/// Get pattern type name by u8 value
pub fn pattern_type_name(pattern_type: u8) -> &'static str {
    match pattern_type {
        0 => "FastPrefix",
        1 => "StructuredFormat",
        2 => "RegexBased",
        _ => "Unknown",
    }
}

/// Get pattern type description by u8 value
pub fn pattern_type_description(pattern_type: u8) -> &'static str {
    match pattern_type {
        0 => "Fast prefix-only matching",
        1 => "Structured format (JWT)",
        2 => "Full regex matching",
        _ => "Unknown type",
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExportedPattern {
    pub name: [u8; 128],
    pub prefix: [u8; 256],
    pub min_len: usize,
    pub pattern_type: u8,  // 0=FastPrefix, 1=StructuredFormat, 2=RegexBased
}

#[repr(C)]
pub struct PatternDetector;

extern "C" {
    pub fn scred_detector_new() -> *mut PatternDetector;
    pub fn scred_detector_process(
        detector: *mut PatternDetector,
        input: *const u8,
        input_len: usize,
        is_eof: bool,
    ) -> *mut u8;
    pub fn scred_detector_get_events(detector: *mut PatternDetector) -> *const DetectionEvent;
    pub fn scred_detector_get_event_count(detector: *mut PatternDetector) -> usize;
    pub fn scred_detector_free(detector: *mut PatternDetector);
    pub fn scred_detector_get_redacted_output(detector: *const PatternDetector) -> *const u8;
    pub fn scred_detector_get_output_length(detector: *const PatternDetector) -> usize;

    // Pattern export functions - source of truth from Zig
    pub fn scred_detector_get_pattern_count() -> usize;
    pub fn scred_detector_get_pattern(index: usize, exported: *mut ExportedPattern) -> c_int;
}

/// Safe wrapper for pattern detection
pub struct Detector {
    ptr: *mut PatternDetector,
}

impl Detector {
    pub fn new() -> Result<Self, &'static str> {
        unsafe {
            let ptr = scred_detector_new();
            if ptr.is_null() {
                Err("Failed to create detector")
            } else {
                Ok(Detector { ptr })
            }
        }
    }

    /// Process data chunk and get detection events
    pub fn process(&mut self, input: &[u8], is_eof: bool) -> Result<ProcessResult, &'static str> {
        unsafe {
            let output_ptr = scred_detector_process(
                self.ptr,
                input.as_ptr(),
                input.len(),
                is_eof,
            );

            if output_ptr.is_null() {
                return Err("Detection failed");
            }

            let event_count = scred_detector_get_event_count(self.ptr);
            let mut events = Vec::new();

            if event_count > 0 {
                let events_ptr = scred_detector_get_events(self.ptr);
                if !events_ptr.is_null() {
                    events = std::slice::from_raw_parts(events_ptr, event_count).to_vec();
                }
            }

            Ok(ProcessResult {
                events,
                bytes_processed: input.len(),
            })
        }
    }
}

impl Drop for Detector {
    fn drop(&mut self) {
        unsafe {
            scred_detector_free(self.ptr);
        }
    }
}

pub struct ProcessResult {
    pub events: Vec<DetectionEvent>,
    pub bytes_processed: usize,
}

impl DetectionEvent {
    pub fn pattern_name(&self) -> String {
        unsafe {
            let bytes = std::slice::from_raw_parts(
                self.pattern_name.as_ptr() as *const u8,
                self.name_len as usize,
            );
            String::from_utf8_lossy(bytes).to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[test]
    #[ignore]  // Old Detector API - replaced with FFI direct calls
    fn test_detector_creation() {
        let detector = Detector::new();
        assert!(detector.is_ok());
    }

    #[test]
    #[ignore]  // Old Detector API - use ZigAnalyzer instead
    fn test_aws_detection() {
        let mut detector = Detector::new().unwrap();
        let input = b"api_key=AKIAIOSFODNN7TESTEXAMPLE";
        let result = detector.process(input, true).unwrap();

        println!("AWS test: {} events detected", result.events.len());
        if result.events.len() > 0 {
            for event in &result.events {
                println!("  Pattern: {} at pos {} len {}",
                    event.pattern_name(), event.position, event.length);
            }
        }
        assert!(result.events.len() > 0);
    }

    #[test]
    #[ignore]  // Old Detector API - use ZigAnalyzer instead
    fn test_github_token_detection() {
        let mut detector = Detector::new().unwrap();
        let input = b"token=ghp_TESTKEYabcdefghijklmnopqrstuvwxyz0123456789";
        let result = detector.process(input, true).unwrap();

        println!("GitHub test: {} events detected", result.events.len());
        assert!(result.events.len() > 0);
    }

    #[test]
    #[ignore]  // Old Detector API - use ZigAnalyzer instead
    fn test_multiple_patterns() {
        let mut detector = Detector::new().unwrap();

        let tests = vec![
            ("AKIAIOSFODNN7TESTEXAMPLE", "AWS Access Token", 1),
            ("ghp_abcdefghijklmnopqrstuvwxyz0123456789", "GitHub PAT", 1),
            ("dummy_stripe_test_key", "Stripe Live", 1),
            ("dummy_openai_project_key", "OpenAI Project", 1),
            ("dummy_openai_svc_key", "OpenAI Service", 1),
            ("sk-abc123defghijklmnopqrstuvwxyz0123456789abcdefghijk", "OpenAI Org", 1),
            ("postgres://user:pass@host:5432/database", "PostgreSQL", 1),
        ];

        for (input, label, expected) in tests {
            let result = detector.process(input.as_bytes(), true).unwrap();
            println!("{}: {} events", label, result.events.len());
            assert!(result.events.len() >= expected, "{} should detect at least {} patterns", label, expected);
        }
    }

    #[test]
    #[ignore]  // Old Detector API - use ZigAnalyzer instead
    fn test_streaming_mode() {
        let mut detector = Detector::new().unwrap();

        // Stream: chunk1 has no pattern, chunk2 has AWS token
        let chunk1 = b"prefix=";
        let chunk2 = b"AKIAIOSFODNN7EXAMPLE and more";

        let result1 = detector.process(chunk1, false).unwrap();
        println!("Chunk 1: {} events", result1.events.len());

        let result2 = detector.process(chunk2, true).unwrap();
        println!("Chunk 2: {} events", result2.events.len());

        assert!(result2.events.len() > 0);
    }

    #[test]
    #[ignore]  // Old Detector API - use ZigAnalyzer instead
    fn test_event_details() {
        let mut detector = Detector::new().unwrap();
        let input = b"key=AKIAIOSFODNN7EXAMPLE";
        let result = detector.process(input, true).unwrap();

        assert!(result.events.len() > 0);
        let event = &result.events[0];

        println!("Event Details:");
        println!("  Pattern: {}", event.pattern_name());
        println!("  ID: {}", event.pattern_id);
        println!("  Position: {}", event.position);
        println!("  Length: {}", event.length);

        assert!(event.position < input.len() as usize);
        assert!(event.length > 0);
        assert!(!event.pattern_name().is_empty());
    }
}

#[cfg(test)]
mod throughput_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn bench_throughput_baseline() {
        let mut detector = Detector::new().unwrap();
        let chunk_size = 1_000_000; // 1MB
        let chunks = 10;
        let data = vec![b'x'; chunk_size];

        let start = Instant::now();
        for i in 0..chunks {
            let _ = detector.process(&data, i == chunks - 1);
        }
        let elapsed = start.elapsed();
        let throughput_mbs = (chunk_size * chunks) as f64 / 1_000_000.0 / elapsed.as_secs_f64();

        println!("\n📊 Baseline throughput: {:.1} MB/s", throughput_mbs);
        assert!(throughput_mbs > 50.0, "Throughput too low: {} MB/s", throughput_mbs);
    }

    #[test]
    #[ignore]
    fn bench_throughput_with_matches() {
        let mut detector = Detector::new().unwrap();
        let chunk_size = 1_000_000;
        let chunks = 10;
        let mut data = vec![b'x'; chunk_size];

        // Embed AWS keys
        let aws_key = b"AKIAIOSFODNN7EXAMPLE";
        for i in 0..5 {
            let offset = i * 100000;
            if offset + aws_key.len() < chunk_size {
                data[offset..offset + aws_key.len()].copy_from_slice(aws_key);
            }
        }

        let start = Instant::now();
        let mut total_events = 0;
        for i in 0..chunks {
            let result = detector.process(&data, i == chunks - 1).unwrap();
            total_events += result.events.len();
        }
        let elapsed = start.elapsed();
        let throughput_mbs = (chunk_size * chunks) as f64 / 1_000_000.0 / elapsed.as_secs_f64();

        println!("\n📊 With matches ({} events) throughput: {:.1} MB/s", total_events, throughput_mbs);
        assert!(throughput_mbs > 50.0);
        assert!(total_events > 0);
    }

    #[test]
    #[ignore]
    fn bench_throughput_large_file() {
        let mut detector = Detector::new().unwrap();
        let chunk_size = 10_000_000; // 10MB
        let chunks = 10; // 100MB total
        let mut data = vec![b'x'; chunk_size];

        // Embed various patterns
        let patterns = [
            b"AKIAIOSFODNN7EXAMPLE".as_ref(),
            b"ghp_abcdefghijklmnopqrstuvwxyz01234567",
            b"dummy_stripe_key",
        ];

        for i in 0..chunk_size / 50000 {
            let offset = i * 50000;
            if offset < chunk_size {
                let pattern = patterns[i % patterns.len()];
                if offset + pattern.len() < chunk_size {
                    data[offset..offset + pattern.len()].copy_from_slice(pattern);
                }
            }
        }

        let start = Instant::now();
        let mut total_events = 0;
        for i in 0..chunks {
            let result = detector.process(&data, i == chunks - 1).unwrap();
            total_events += result.events.len();
        }
        let elapsed = start.elapsed();
        let total_mb = (chunk_size * chunks) as f64 / 1_000_000.0;
        let throughput_mbs = total_mb / elapsed.as_secs_f64();

        println!("\n📊 Large file ({}MB, {} events) throughput: {:.1} MB/s",
            total_mb as usize, total_events, throughput_mbs);
        println!("   Average per 10MB: {:.2} ms", elapsed.as_secs_f64() * 1000.0 / chunks as f64);
        assert!(throughput_mbs > 50.0);
    }
}

#[cfg(test)]
mod realistic_throughput_tests {
    use super::*;
    use std::time::Instant;

    /// Test with patterns at START of chunks (worst case for streaming)
    #[test]
    #[ignore]
    fn bench_patterns_at_start() {
        let mut detector = Detector::new().unwrap();
        let chunk_size = 1_000_000;
        let chunks = 10;
        let mut data = vec![b'x'; chunk_size];

        // Embed patterns at START of each chunk (streaming boundary stress)
        let aws_key = b"AKIAIOSFODNN7EXAMPLE";
        data[0..aws_key.len()].copy_from_slice(aws_key);

        let start = Instant::now();
        let mut total_events = 0;
        for i in 0..chunks {
            let result = detector.process(&data, i == chunks - 1).unwrap();
            total_events += result.events.len();
        }
        let elapsed = start.elapsed();
        let throughput_mbs = (chunk_size * chunks) as f64 / 1_000_000.0 / elapsed.as_secs_f64();

        println!("\n📊 Patterns at START (streaming boundary):");
        println!("  Throughput: {:.1} MB/s", throughput_mbs);
        println!("  Matches: {}", total_events);
        println!("  Time per 1MB: {:.2} ms", elapsed.as_secs_f64() * 1000.0 / chunks as f64);
        assert!(throughput_mbs > 30.0);  // Realistic threshold for this scenario
    }

    /// Test with patterns at END of chunks (potential lookahead needed)
    #[test]
    #[ignore]
    fn bench_patterns_at_end() {
        let mut detector = Detector::new().unwrap();
        let chunk_size = 1_000_000;
        let chunks = 10;
        let mut data = vec![b'x'; chunk_size];

        // Embed patterns at END of each chunk
        let aws_key = b"AKIAIOSFODNN7EXAMPLE";
        let end_pos = chunk_size - aws_key.len();
        data[end_pos..chunk_size].copy_from_slice(aws_key);

        let start = Instant::now();
        let mut total_events = 0;
        for i in 0..chunks {
            let result = detector.process(&data, i == chunks - 1).unwrap();
            total_events += result.events.len();
        }
        let elapsed = start.elapsed();
        let throughput_mbs = (chunk_size * chunks) as f64 / 1_000_000.0 / elapsed.as_secs_f64();

        println!("\n📊 Patterns at END (lookahead stress):");
        println!("  Throughput: {:.1} MB/s", throughput_mbs);
        println!("  Matches: {}", total_events);
        println!("  Time per 1MB: {:.2} ms", elapsed.as_secs_f64() * 1000.0 / chunks as f64);
        assert!(throughput_mbs > 30.0);  // Realistic threshold for boundary stress test
    }

    /// Test with patterns scattered throughout
    #[test]
    #[ignore]
    fn bench_patterns_scattered() {
        let mut detector = Detector::new().unwrap();
        let chunk_size = 1_000_000;
        let chunks = 10;
        let mut data = vec![b'x'; chunk_size];

        // Scatter 20 patterns throughout chunk
        let patterns = [
            (b"AKIAIOSFODNN7EXAMPLE".as_ref(), 0),
            (b"ghp_abcdefghijklmnopqrstuvwxyz01234567".as_ref(), 50000),
            (b"dummy_stripe_key".as_ref(), 100000),
            (b"sk-proj-1234567890abcdefghijk".as_ref(), 150000),
            (b"postgres://user:pass@host/db".as_ref(), 200000),
            (b"AKIAIOSFODNN7EXAMPLE".as_ref(), 250000),
            (b"ghp_abcdefghijklmnopqrstuvwxyz01234567".as_ref(), 300000),
            (b"dummy_stripe_key".as_ref(), 350000),
            (b"sk-proj-1234567890abcdefghijk".as_ref(), 400000),
            (b"Bearer sk-proj-test".as_ref(), 450000),
            (b"Authorization: Bearer sk-proj-key".as_ref(), 500000),
            (b"AKIAIOSFODNN7EXAMPLE".as_ref(), 550000),
            (b"ghp_abcdefghijklmnopqrstuvwxyz01234567".as_ref(), 600000),
            (b"xoxb-TESTSLACKTOKEN123456789-987654321-abcdefghijklmnopqrst".as_ref(), 650000),
            (b"eyJhbGciOiJIUzI1NiJ9abc".as_ref(), 700000),
            (b"dummy_stripe_key".as_ref(), 750000),
            (b"mongodb://user:AKIAIOSFODNN7@host".as_ref(), 800000),
            (b"sk-proj-1234567890abcdefghijk".as_ref(), 850000),
            (b"postgres://AKIAIOSFODNN7@host".as_ref(), 900000),
            (b"ghp_abcdefghijklmnopqrstuvwxyz01234567".as_ref(), 950000),
        ];

        for (pattern, offset) in &patterns {
            if offset + pattern.len() < chunk_size {
                data[*offset..*offset + pattern.len()].copy_from_slice(pattern);
            }
        }

        let start = Instant::now();
        let mut total_events = 0;
        for i in 0..chunks {
            let result = detector.process(&data, i == chunks - 1).unwrap();
            total_events += result.events.len();
        }
        let elapsed = start.elapsed();
        let throughput_mbs = (chunk_size * chunks) as f64 / 1_000_000.0 / elapsed.as_secs_f64();

        println!("\n📊 Patterns scattered (20 per MB):");
        println!("  Throughput: {:.1} MB/s", throughput_mbs);
        println!("  Total matches: {}", total_events);
        println!("  Matches per MB: {}", total_events / chunks);
        println!("  Time per 1MB: {:.2} ms", elapsed.as_secs_f64() * 1000.0 / chunks as f64);
        assert!(throughput_mbs > 200.0);  // Scattered patterns should be fast
    }

    /// Test with realistic HTTP request payloads (headers + body)
    #[test]
    #[ignore]
    fn bench_http_request_payloads() {
        let mut detector = Detector::new().unwrap();
        let mut total_data = Vec::new();

        // Create realistic HTTP request with mixed secret locations
        let request = "POST /api/v1/models HTTP/2\r\n\
                       Host: api.openai.com\r\n\
                       Authorization: Bearer dummy_openai_key\r\n\
                       X-API-Key: AKIAIOSFODNN7TESTEXAMPLE\r\n\
                       User-Agent: curl/7.64.1\r\n\
                       Accept: application/json\r\n\
                       Content-Type: application/json\r\n\
                       Content-Length: 284\r\n\
                       \r\n\
                       {\r\n\
                         \"model\": \"gpt-4\",\r\n\
                         \"api_key\": \"dummy_openai_key\",\r\n\
                         \"messages\": [\r\n\
                           {\"role\": \"user\", \"content\": \"Generate code\"},\r\n\
                           {\"role\": \"system\", \"content\": \"token=ghp_TESTKEYabcdefghijklmnopqrstuvwxyz01234567\"}\r\n\
                         ],\r\n\
                         \"temperature\": 0.7\r\n\
                       }\r\n";

        // Repeat request 100 times to simulate stream of requests
        for _ in 0..100 {
            total_data.extend_from_slice(request.as_bytes());
        }

        let chunk_size = 65536; // 64KB chunks like typical network packets
        let start = Instant::now();
        let mut pos = 0;
        while pos < total_data.len() {
            let end = std::cmp::min(pos + chunk_size, total_data.len());
            let is_last = (end >= total_data.len());
            let result = detector.process(&total_data[pos..end], is_last).unwrap();
            // Note: we can't accumulate events across calls in the old API
            let _ = result.events.len();  // Use result to prevent unused warning
            pos = end;
        }
        let elapsed = start.elapsed();

        let total_mb = total_data.len() as f64 / 1_000_000.0;
        let throughput_mbs = total_mb / elapsed.as_secs_f64();

        println!("\n📊 HTTP Request Payloads (realistic):");
        println!("  Data size: {:.1} MB", total_mb);
        println!("  Chunk size: {} KB", chunk_size / 1024);
        println!("  Throughput: {:.1} MB/s", throughput_mbs);
        println!("  Expected ~2000 matches: (not counted in old API)");
        println!("  Time per request (~600B): {:.3} ms",
                 elapsed.as_secs_f64() * 1000.0 / 100.0);
        assert!(throughput_mbs > 50.0);  // Realistic threshold for HTTP payload processing
    }

    /// Test with database connection strings + credentials
    #[test]
    #[ignore]
    fn bench_database_connection_strings() {
        let mut detector = Detector::new().unwrap();
        let mut data = Vec::new();

        // Create realistic database logs
        let log_lines = [
            "2024-03-20 10:15:23 [INFO] Connecting to postgres://user:AKIAIOSFODNN7EXAMPLE@db.example.com:5432/mydb",
            "2024-03-20 10:15:24 [INFO] Connection established",
            "2024-03-20 10:15:25 [DEBUG] Query: SELECT * FROM users WHERE id = $1",
            "2024-03-20 10:15:26 [INFO] Connecting to mysql://admin:sk_live_test@mysql.internal/prod",
            "2024-03-20 10:15:27 [INFO] MySQL connection OK",
            "2024-03-20 10:15:28 [DEBUG] Executing backup to mongodb://backup:ghp_abcdef@mongo.backup/archive",
            "2024-03-20 10:15:29 [INFO] Backup started",
            "2024-03-20 10:15:30 [ERROR] Failed auth: postgres://AKIAIOSFODNN7@replica:5432/failover",
            "2024-03-20 10:15:31 [INFO] Retrying connection",
            "2024-03-20 10:15:32 [INFO] Cache: redis://default:sk-proj-key@cache.local:6379",
        ];

        // Repeat 1000 times to create realistic log volume
        for _ in 0..1000 {
            for line in &log_lines {
                data.extend_from_slice(line.as_bytes());
                data.push(b'\n');
            }
        }

        let chunk_size = 4_000_000; // 4MB chunks like typical file reads
        let start = Instant::now();
        let mut total_events = 0;
        let mut pos = 0;

        while pos < data.len() {
            let end = std::cmp::min(pos + chunk_size, data.len());
            let is_last = (end >= data.len());
            let result = detector.process(&data[pos..end], is_last).unwrap();
            total_events += result.events.len();
            pos = end;
        }
        let elapsed = start.elapsed();

        let total_mb = data.len() as f64 / 1_000_000.0;
        let throughput_mbs = total_mb / elapsed.as_secs_f64();

        println!("\n📊 Database Connection Logs:");
        println!("  Data size: {:.1} MB", total_mb);
        println!("  Log lines: {}", 10000);
        println!("  Throughput: {:.1} MB/s", throughput_mbs);
        println!("  Total matches: {}", total_events);
        println!("  Matches per 10 lines: {}", total_events / 1000);
        assert!(throughput_mbs > 50.0);  // Lower bar for logs
    }

    /// Test with NO patterns (baseline clean data)
    #[test]
    #[ignore]
    fn bench_no_patterns_clean_data() {
        let mut detector = Detector::new().unwrap();
        let mut data = Vec::new();

        // Create realistic log lines WITHOUT secrets
        let clean_lines = [
            "2024-03-20 10:15:23 [INFO] Server started on port 8080",
            "2024-03-20 10:15:24 [INFO] Database connection pool initialized",
            "2024-03-20 10:15:25 [DEBUG] Cache layer activated",
            "2024-03-20 10:15:26 [INFO] Health check passed",
            "2024-03-20 10:15:27 [INFO] Request received from client",
            "2024-03-20 10:15:28 [DEBUG] Processing request id=12345",
            "2024-03-20 10:15:29 [INFO] Response sent (200 OK)",
            "2024-03-20 10:15:30 [INFO] Metrics: requests=1000, latency_ms=45",
            "2024-03-20 10:15:31 [DEBUG] Garbage collection: freed 512MB",
            "2024-03-20 10:15:32 [INFO] All systems operational",
        ];

        // Repeat 10000 times to create 100MB+ clean data
        for _ in 0..10000 {
            for line in &clean_lines {
                data.extend_from_slice(line.as_bytes());
                data.push(b'\n');
            }
        }

        let chunk_size = 10_000_000; // 10MB chunks
        let start = Instant::now();
        let mut total_events = 0;
        let mut pos = 0;

        while pos < data.len() {
            let end = std::cmp::min(pos + chunk_size, data.len());
            let is_last = (end >= data.len());
            let result = detector.process(&data[pos..end], is_last).unwrap();
            total_events += result.events.len();
            pos = end;
        }
        let elapsed = start.elapsed();

        let total_mb = data.len() as f64 / 1_000_000.0;
        let throughput_mbs = total_mb / elapsed.as_secs_f64();

        println!("\n📊 Clean Data (NO patterns):");
        println!("  Data size: {:.1} MB", total_mb);
        println!("  Throughput: {:.1} MB/s", throughput_mbs);
        println!("  False positives: {}", total_events);
        println!("  Expected: 0");
        assert_eq!(total_events, 0, "Should have zero matches");
        assert!(throughput_mbs > 150.0);  // Should be fastest (no event overhead)
    }

    /// Test with MIXED realistic data (secrets + clean)
    #[test]
    #[ignore]
    fn bench_mixed_realistic_data() {
        let mut detector = Detector::new().unwrap();
        let mut data = Vec::new();

        // 90% clean data, 10% with secrets (realistic log mix)
        let clean_lines = [
            "2024-03-20 10:15:23 [INFO] Request received",
            "2024-03-20 10:15:24 [DEBUG] Processing payload",
            "2024-03-20 10:15:25 [INFO] Database query executed",
        ];

        let secret_lines = [
            "2024-03-20 10:15:23 [INFO] API token: dummy_openai_test",
            "2024-03-20 10:15:24 [DEBUG] AWS credentials: AKIAIOSFODNN7EXAMPLE",
        ];

        // Create mix: 9 clean lines for every 1 secret line
        for i in 0..10000 {
            if i % 10 == 0 {
                for line in &secret_lines {
                    data.extend_from_slice(line.as_bytes());
                    data.push(b'\n');
                }
            } else {
                for line in &clean_lines {
                    data.extend_from_slice(line.as_bytes());
                    data.push(b'\n');
                }
            }
        }

        let chunk_size = 10_000_000; // 10MB chunks
        let start = Instant::now();
        let mut total_events = 0;
        let mut pos = 0;

        while pos < data.len() {
            let end = std::cmp::min(pos + chunk_size, data.len());
            let is_last = (end >= data.len());
            let result = detector.process(&data[pos..end], is_last).unwrap();
            total_events += result.events.len();
            pos = end;
        }
        let elapsed = start.elapsed();

        let total_mb = data.len() as f64 / 1_000_000.0;
        let throughput_mbs = total_mb / elapsed.as_secs_f64();

        println!("\n📊 Mixed Data (90% clean, 10% secrets):");
        println!("  Data size: {:.1} MB", total_mb);
        println!("  Throughput: {:.1} MB/s", throughput_mbs);
        println!("  Total matches: {}", total_events);
        println!("  Expected ~2000 matches: {}", 
                 if total_events > 1800 && total_events < 2200 { "✓ Correct" } else { "✗ Wrong" });
        assert!(throughput_mbs > 80.0);  // Realistic target: 80-100 MB/s with variance
        assert!(total_events > 1800 && total_events < 2200);
    }
}

use std::sync::Arc;

// ============================================================================
// Public API: Get patterns from Zig (source of truth)
// ============================================================================

#[derive(Debug, Clone)]
pub struct PatternInfo {
    pub name: String,
    pub prefix: String,
    pub min_len: usize,
    pub pattern_type: u8,  // 0=FastPrefix, 1=StructuredFormat, 2=RegexBased
}

// ============================================================================
// NEW: Conversion Functions for Phase 2 Classification Architecture
// ============================================================================

impl ExportedPattern {
    /// Get pattern type as string
    pub fn pattern_type_name(&self) -> &'static str {
        match self.pattern_type {
            0 => "FastPrefix",
            1 => "StructuredFormat",
            2 => "RegexBased",
            _ => "Unknown",
        }
    }

    /// Get pattern type description
    pub fn pattern_type_description(&self) -> &'static str {
        match self.pattern_type {
            0 => "Fast prefix-only matching (<5ms for all)",
            1 => "Structured format validation (JWT, etc.)",
            2 => "Full regex matching (~1000ms for all)",
            _ => "Unknown",
        }
    }

    /// Get pattern type as enum
    pub fn as_pattern_type(&self) -> Option<PatternType> {
        PatternType::from_u8(self.pattern_type)
    }
}

/// Get all patterns from the Zig detector (source of truth)
pub fn get_all_patterns() -> Vec<PatternInfo> {
    use std::ffi::CStr;
    
    let mut patterns = Vec::new();
    
    unsafe {
        let count = scred_detector_get_pattern_count();
        for i in 0..count {
            let mut exported = std::mem::zeroed::<ExportedPattern>();
            let result = scred_detector_get_pattern(i, &mut exported);
            if result != 0 {
                // Use CStr to read null-terminated strings correctly
                let name = CStr::from_bytes_until_nul(&exported.name)
                    .ok()
                    .and_then(|s| s.to_str().ok())
                    .unwrap_or("INVALID")
                    .to_string();
                    
                let prefix = CStr::from_bytes_until_nul(&exported.prefix)
                    .ok()
                    .and_then(|s| s.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                
                patterns.push(PatternInfo {
                    name,
                    prefix,
                    min_len: exported.min_len,
                    pattern_type: exported.pattern_type,
                });
            }
        }
    }
    
    patterns
}
// ============================================================================
// PHASE 2: METADATA FFI BINDINGS (Task 3)
// ============================================================================

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PatternMetadataFFI {
    // Identity fields
    pub name: *const u8,
    pub name_len: usize,
    
    // Classification
    pub tier: u8,           // 0-4: critical, api_keys, infrastructure, services, patterns
    pub category: u8,       // 0-5: simple_prefix, fixed, minlen, variable, jwt, regex
    pub risk_score: u8,     // 0-100
    pub ffi_path: u8,       // 0-6: match_prefix, charset, length, minlen, variable, jwt, regex
    
    // Prefix information
    pub prefix: *const u8,
    pub prefix_len: u16,
    
    // Charset type
    pub charset_type: u8,   // 0-5: alphanumeric, hex, base64, base64url, numeric, any
    
    // Length constraints
    pub min_length: u16,
    pub max_length: u16,
    pub fixed_length: u16,  // 0 if variable
    
    // Regex pattern
    pub regex_pattern: *const u8,
    pub regex_len: usize,
    
    // Example secret
    pub example_secret: *const u8,
    pub example_len: usize,
    
    // Tags (comma-separated)
    pub tags: *const u8,
    pub tags_len: usize,
}

extern "C" {
    /// Get pattern metadata by name
    pub fn scred_pattern_get_metadata_by_name(
        name: *const u8,
        name_len: usize,
    ) -> PatternMetadataFFI;
    
    /// Get pattern metadata by index
    pub fn scred_pattern_get_metadata_by_index(index: usize) -> PatternMetadataFFI;
    
    /// Get pattern tier by name
    pub fn scred_pattern_get_tier(
        name: *const u8,
        name_len: usize,
    ) -> u8;
    
    /// Get total pattern count
    pub fn scred_pattern_count() -> usize;
    
    /// Check if pattern is in tier
    pub fn scred_pattern_in_tier(
        name: *const u8,
        name_len: usize,
        tier: u8,
    ) -> bool;
}

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_metadata_pattern_count() {
        unsafe {
            let count = scred_pattern_count();
            println!("Total patterns: {}", count);
            assert_eq!(count, 274, "Expected 274 patterns");
        }
    }

    #[test]
    #[ignore]
    fn test_metadata_by_index() {
        unsafe {
            let metadata = scred_pattern_get_metadata_by_index(0);
            println!("First pattern name length: {}", metadata.name_len);
            assert!(!metadata.name.is_null());
            assert!(metadata.name_len > 0);
        }
    }

    #[test]
    #[ignore]
    fn test_get_tier() {
        unsafe {
            let name = b"aws-access-key";
            let tier = scred_pattern_get_tier(name.as_ptr(), name.len());
            println!("AWS pattern tier: {}", tier);
            assert_ne!(tier, 255);
        }
    }

    #[test]
    #[ignore]
    fn test_pattern_in_tier() {
        unsafe {
            let name = b"github-token";
            let in_tier = scred_pattern_in_tier(name.as_ptr(), name.len(), 1);
            println!("GitHub token in tier 1: {}", in_tier);
            assert!(in_tier);
        }
    }
}

// ============================================================================
// WAVE 3: SIMD-OPTIMIZED VALIDATORS - FFI EXPORTS
// ============================================================================

extern "C" {
    /// WAVE 3: Bearer Token OAuth2 Validator (ROI: 90, Target: 15-20x)
    pub fn validate_bearer_token_simd(
        data: *const u8,
        data_len: usize,
    ) -> bool;

    /// WAVE 3: IPv4 Address Validator (ROI: 85, Target: 15-25x)
    pub fn validate_ipv4_simd(
        data: *const u8,
        data_len: usize,
    ) -> bool;

    /// WAVE 3: Credit Card Number Validator (ROI: 80, Target: 20-30x)
    pub fn validate_credit_card_simd(
        data: *const u8,
        data_len: usize,
    ) -> bool;

    /// WAVE 3: AWS Secret Access Key Validator (ROI: 75, Target: 6-10x)
    pub fn validate_aws_secret_key_simd(
        data: *const u8,
        data_len: usize,
    ) -> bool;

    /// WAVE 3: Email Address Validator (ROI: 60, Target: 12-18x)
    pub fn validate_email_simd(
        data: *const u8,
        data_len: usize,
    ) -> bool;

    /// WAVE 3: Phone Number Validator (ROI: 65, Target: 10-15x)
    pub fn validate_phone_number_simd(
        data: *const u8,
        data_len: usize,
    ) -> bool;

    /// WAVE 3: Git Repository URL Validator (ROI: 70, Target: 6-10x)
    pub fn validate_git_repo_url_simd(
        data: *const u8,
        data_len: usize,
    ) -> bool;

    /// WAVE 3: API Key Generic Validator (ROI: 55, Target: 8-12x)
    pub fn validate_api_key_generic_simd(
        prefix_type: u8,
        data: *const u8,
        data_len: usize,
    ) -> bool;
}

// ============================================================================
// C FFI: Redaction Engine
// ============================================================================

/// Result from Zig redaction engine
#[repr(C)]
pub struct ZigRedactionResult {
    pub output: Option<*mut u8>,
    pub output_len: usize,
    pub match_count: u32,
}

extern "C" {
    /// Redact text using Zig's optimized pattern detection
    /// 
    /// Returns a RedactionResult with:
    /// - output: allocated by Zig (must call free_redaction_result)
    /// - output_len: length of redacted output
    /// - match_count: number of patterns found
    pub fn scred_redact_text_optimized_stub(
        text: *const u8,
        text_len: usize,
    ) -> ZigRedactionResult;

    /// Free the output buffer from ZigRedactionResult
    pub fn free_redaction_result(result: ZigRedactionResult);
}
