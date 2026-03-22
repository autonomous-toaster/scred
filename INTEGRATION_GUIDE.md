# SCRED Pattern Detector - Integration Guide

## Overview

The detector is designed to integrate into three SCRED components, each with different streaming contexts:

| Component | Context | Use Case | Streaming Model |
|-----------|---------|----------|-----------------|
| **scred-redactor** | CLI tool | Local file/stdin redaction | Chunked file reads |
| **scred-mitm** | Network proxy | TLS stream interception | Live TCP stream chunks |
| **scred-proxy** | HTTP proxy | HTTP/2 stream redaction | Frame-by-frame streaming |

---

## Architecture: Universal Streaming Interface

### Core Pattern

```rust
pub struct Detector {
    ptr: *mut PatternDetector,
}

impl Detector {
    pub fn process(&mut self, chunk: &[u8], is_eof: bool) 
        -> Result<ProcessResult, &'static str>
    {
        // Process independent chunk
        // Accumulate events
        // Maintain state across chunks
    }
}

pub struct ProcessResult {
    pub events: Vec<DetectionEvent>,  // Matches in this chunk
    pub bytes_processed: usize,
}

pub struct DetectionEvent {
    pub pattern_id: u16,
    pub pattern_name: [c_char; 64],
    pub position: usize,              // Offset in chunk
    pub length: u16,                  // Match length
}
```

### Key Properties

✅ **Stateful**: Maintains detector state across chunks  
✅ **Independent chunks**: Each chunk processed independently  
✅ **Event accumulation**: Get events from each chunk  
✅ **EOF handling**: Flush logic on is_eof=true  
✅ **Zero-copy**: Works with borrowed slices  

---

## 1. Integration into scred-redactor (CLI)

### Purpose
Redact secrets in files before upload/deployment

### Streaming Model: File Chunking
```
Read 4KB chunk → Detect → Redact → Write
Read 4KB chunk → Detect → Redact → Write
... repeat until EOF
```

### Implementation

#### File: `crates/scred-redactor/src/lib.rs`

```rust
use scred_pattern_detector_zig::{Detector, DetectionEvent};
use std::io::{Read, Write};

pub struct ZigRedactor {
    detector: Detector,
    input: Box<dyn Read>,
    output: Box<dyn Write>,
}

impl ZigRedactor {
    pub fn new(input: Box<dyn Read>, output: Box<dyn Write>) 
        -> Result<Self, Box<dyn std::error::Error>> 
    {
        Ok(Self {
            detector: Detector::new()?,
            input,
            output,
        })
    }

    pub fn redact_streaming(&mut self) 
        -> Result<RedactionStats, Box<dyn std::error::Error>> 
    {
        let mut stats = RedactionStats::default();
        let mut buffer = vec![0u8; 4096]; // 4KB chunks
        let mut position: usize = 0;

        loop {
            // Read chunk
            let n = self.input.read(&mut buffer)?;
            if n == 0 {
                break; // EOF
            }

            let chunk = &buffer[..n];
            let is_eof = n < buffer.len();

            // Process chunk through detector
            let result = self.detector.process(chunk, is_eof)?;
            
            // Collect events
            stats.total_matches += result.events.len();
            for event in &result.events {
                stats.by_pattern
                    .entry(event.pattern_name())
                    .or_insert(0)
                    += 1;
            }

            // Redact matches
            let redacted = Self::redact_chunk(chunk, &result.events);

            // Write redacted chunk
            self.output.write_all(&redacted)?;
            position += n;
        }

        stats.total_bytes = position;
        Ok(stats)
    }

    fn redact_chunk(chunk: &[u8], events: &[DetectionEvent]) -> Vec<u8> {
        let mut redacted = chunk.to_vec();
        
        for event in events {
            let end = event.position + event.length as usize;
            if end <= redacted.len() {
                for i in event.position..end {
                    redacted[i] = b'x';
                }
            }
        }
        
        redacted
    }
}

#[derive(Default, Debug)]
pub struct RedactionStats {
    pub total_bytes: usize,
    pub total_matches: usize,
    pub by_pattern: std::collections::HashMap<String, usize>,
}
```

#### Usage

```rust
// CLI: scred redact --input secret.txt --output redacted.txt
let input = Box::new(File::open("secret.txt")?);
let output = Box::new(File::create("redacted.txt")?);

let mut redactor = ZigRedactor::new(input, output)?;
let stats = redactor.redact_streaming()?;

println!("Redacted {} secrets across {} bytes", 
    stats.total_matches, stats.total_bytes);
```

### Integration Checklist

- [ ] Add `scred-pattern-detector-zig` as dependency in `Cargo.toml`
- [ ] Implement `ZigRedactor` with 4KB chunk size
- [ ] Update `scred redact` command to use `ZigRedactor`
- [ ] Migrate from regex-based detector
- [ ] Run test suite: `cargo test --lib`
- [ ] Benchmark: `cargo bench --bench redaction`
- [ ] Verify output compatibility with existing tools

---

## 2. Integration into scred-mitm (Network Proxy)

### Purpose
Intercept TLS streams, redact secrets, re-encrypt

### Streaming Model: TCP Packet Streaming
```
Receive TCP packet → TLS decrypt → Detect → Redact → TLS encrypt → Send
... continuous stream processing
```

### Implementation

#### File: `crates/scred-mitm/src/mitm/detector_stream.rs` (new)

```rust
use scred_pattern_detector_zig::Detector;
use std::collections::VecDeque;

/// Stateful detector for continuous TLS streams
pub struct StreamingDetector {
    detector: Detector,
    position: usize,  // Absolute position in stream
    event_queue: VecDeque<StreamEvent>,
}

pub struct StreamEvent {
    pub absolute_position: usize,  // Position in full stream
    pub pattern_name: String,
    pub length: u16,
}

impl StreamingDetector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            detector: Detector::new()?,
            position: 0,
            event_queue: VecDeque::new(),
        })
    }

    /// Process a TCP packet (may be partial secret)
    pub fn process_packet(&mut self, packet: &[u8]) 
        -> Result<Vec<StreamEvent>, Box<dyn std::error::Error>> 
    {
        // is_eof only true when TLS connection closes
        let is_eof = false;

        let result = self.detector.process(packet, is_eof)?;

        // Convert local positions to absolute stream positions
        let mut events = Vec::new();
        for event in result.events {
            let abs_pos = self.position + event.position;
            self.event_queue.push_back(StreamEvent {
                absolute_position: abs_pos,
                pattern_name: event.pattern_name(),
                length: event.length,
            });
            events.push(StreamEvent {
                absolute_position: abs_pos,
                pattern_name: event.pattern_name(),
                length: event.length,
            });
        }

        self.position += packet.len();
        Ok(events)
    }

    /// Signal end of stream
    pub fn close_stream(&mut self) 
        -> Result<Vec<StreamEvent>, Box<dyn std::error::Error>> 
    {
        // Process empty chunk with is_eof=true for cleanup
        let result = self.detector.process(&[], true)?;

        let mut events = Vec::new();
        for event in result.events {
            let abs_pos = self.position + event.position;
            events.push(StreamEvent {
                absolute_position: abs_pos,
                pattern_name: event.pattern_name(),
                length: event.length,
            });
        }

        Ok(events)
    }

    /// Get all pending events
    pub fn drain_events(&mut self) -> Vec<StreamEvent> {
        self.event_queue.drain(..).collect()
    }
}
```

#### Integration with TLS proxy

```rust
// File: crates/scred-mitm/src/mitm/tls_mitm.rs

use crate::detector_stream::StreamingDetector;

pub struct TlsForwarder {
    detector: StreamingDetector,
    // ... existing fields
}

impl TlsForwarder {
    pub async fn handle_client_data(&mut self, data: &[u8]) 
        -> Result<Vec<u8>, Box<dyn std::error::Error>> 
    {
        // Decrypt from client
        let decrypted = self.decrypt_from_client(data)?;

        // Detect secrets
        let events = self.detector.process_packet(&decrypted)?;
        
        // Log detections for audit trail
        for event in &events {
            warn!("Detected {} at stream offset {}", 
                event.pattern_name, event.absolute_position);
        }

        // Redact in-place
        let mut redacted = decrypted.clone();
        for event in events {
            // Map event to position in decrypted buffer
            self.redact_range(&mut redacted, event.absolute_position, event.length);
        }

        // Encrypt redacted data
        let encrypted = self.encrypt_to_server(&redacted)?;
        Ok(encrypted)
    }

    pub async fn close(&mut self) 
        -> Result<(), Box<dyn std::error::Error>> 
    {
        // Flush any pending detections
        let _final_events = self.detector.close_stream()?;
        Ok(())
    }

    fn redact_range(&self, data: &mut [u8], start: usize, len: u16) {
        let end = start + len as usize;
        if end <= data.len() {
            data[start..end].fill(b'x');
        }
    }
}
```

### Key Properties for MITM

✅ **Position tracking**: Absolute stream position across packets  
✅ **Stateful**: Detector maintains context across TCP packets  
✅ **Event batching**: Process multiple events per packet  
✅ **Stream lifecycle**: `process_packet()` → `close_stream()`  
✅ **Audit trail**: Log all detections with absolute position  

### Integration Checklist

- [ ] Add `scred-pattern-detector-zig` dependency
- [ ] Create `detector_stream.rs` with `StreamingDetector`
- [ ] Update `TlsForwarder::handle_client_data()` for detection
- [ ] Add position tracking across packets
- [ ] Create audit log for detected secrets
- [ ] Test with real TLS connections
- [ ] Benchmark: measure overhead per packet

---

## 3. Integration into scred-proxy (HTTP/2 Proxy)

### Purpose
Redact secrets in HTTP/2 streams, headers, and payloads

### Streaming Model: Frame-by-Frame
```
HTTP/2 HEADERS frame → Detect headers → Redact
HTTP/2 DATA frame 1 → Detect payload → Redact
HTTP/2 DATA frame 2 → Detect payload → Redact
... per-frame processing
```

### Implementation

#### File: `crates/scred-http/src/h2/detector_integration.rs` (new)

```rust
use scred_pattern_detector_zig::{Detector, DetectionEvent};
use http::HeaderMap;

pub struct Http2SecretDetector {
    detector: Detector,
    frame_buffer: Vec<u8>,  // Buffer for header/payload data
}

impl Http2SecretDetector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            detector: Detector::new()?,
            frame_buffer: Vec::new(),
        })
    }

    /// Process HTTP/2 headers frame
    pub fn process_headers(&mut self, headers: &HeaderMap) 
        -> Result<HeaderRedactionMap, Box<dyn std::error::Error>> 
    {
        let mut redactions = HeaderRedactionMap::new();

        for (name, value) in headers.iter() {
            let header_bytes = format!("{}: {}", 
                name.as_str(), 
                String::from_utf8_lossy(value.as_bytes())
            );

            let result = self.detector.process(header_bytes.as_bytes(), false)?;

            for event in result.events {
                redactions.insert(
                    name.as_str().to_string(),
                    HeaderRedaction {
                        pattern: event.pattern_name(),
                        position: event.position,
                        length: event.length,
                    }
                );
            }
        }

        Ok(redactions)
    }

    /// Process HTTP/2 DATA frame
    pub fn process_body_frame(&mut self, data: &[u8], is_last: bool) 
        -> Result<Vec<DetectionEvent>, Box<dyn std::error::Error>> 
    {
        let result = self.detector.process(data, is_last)?;
        Ok(result.events)
    }

    /// Process complete request/response body
    pub fn process_complete_body(&mut self, body: &[u8]) 
        -> Result<Vec<DetectionEvent>, Box<dyn std::error::Error>> 
    {
        let result = self.detector.process(body, true)?;
        Ok(result.events)
    }

    /// Redact detected secrets in headers
    pub fn redact_headers(&self, headers: &mut HeaderMap, 
        redactions: &HeaderRedactionMap) 
    {
        for (name, redaction) in redactions.iter() {
            if let Ok(value) = headers.get(name) {
                let mut bytes = value.as_bytes().to_vec();
                let end = redaction.position + redaction.length as usize;
                if end <= bytes.len() {
                    bytes[redaction.position..end].fill(b'x');
                }
                // Update header with redacted value
                if let Ok(redacted_value) = http::HeaderValue::from_bytes(&bytes) {
                    headers.insert(name.parse().unwrap(), redacted_value);
                }
            }
        }
    }

    /// Redact detected secrets in body
    pub fn redact_body(&self, body: &mut Vec<u8>, 
        events: &[DetectionEvent]) 
    {
        for event in events {
            let end = event.position + event.length as usize;
            if end <= body.len() {
                body[event.position..end].fill(b'x');
            }
        }
    }
}

pub type HeaderRedactionMap = std::collections::HashMap<String, HeaderRedaction>;

pub struct HeaderRedaction {
    pub pattern: String,
    pub position: usize,
    pub length: u16,
}
```

#### Integration with HTTP/2 stream handler

```rust
// File: crates/scred-http/src/h2/mod.rs

use crate::h2::detector_integration::Http2SecretDetector;

pub struct Http2StreamRedactor {
    detector: Http2SecretDetector,
    // ... existing fields
}

impl Http2StreamRedactor {
    pub async fn redact_request(&mut self, 
        req: &mut Request<Body>) 
        -> Result<(), Box<dyn std::error::Error>> 
    {
        // Detect secrets in headers
        let header_redactions = self.detector.process_headers(req.headers())?;
        self.detector.redact_headers(req.headers_mut(), &header_redactions);

        // Detect secrets in body
        let body_bytes = hyper::body::to_bytes(req.body_mut()).await?;
        let events = self.detector.process_complete_body(&body_bytes)?;

        // Create redacted body
        let mut redacted = body_bytes.to_vec();
        self.detector.redact_body(&mut redacted, &events);

        // Update request with redacted body
        *req.body_mut() = Body::from(redacted);

        Ok(())
    }

    pub async fn redact_response(&mut self, 
        resp: &mut Response<Body>) 
        -> Result<(), Box<dyn std::error::Error>> 
    {
        // Same logic for response
        let header_redactions = self.detector.process_headers(resp.headers())?;
        self.detector.redact_headers(resp.headers_mut(), &header_redactions);

        let body_bytes = hyper::body::to_bytes(resp.body_mut()).await?;
        let events = self.detector.process_complete_body(&body_bytes)?;

        let mut redacted = body_bytes.to_vec();
        self.detector.redact_body(&mut redacted, &events);

        *resp.body_mut() = Body::from(redacted);

        Ok(())
    }
}
```

### Key Properties for HTTP/2

✅ **Frame-aware**: Separate header and body processing  
✅ **Header redaction**: Target-specific header masking  
✅ **Streaming bodies**: Handle chunked encoding  
✅ **Complete bodies**: Process full payloads for pattern context  
✅ **Zero-copy**: Works with borrowed data  

### Integration Checklist

- [ ] Add `scred-pattern-detector-zig` dependency
- [ ] Create `detector_integration.rs` module
- [ ] Implement header detection/redaction
- [ ] Implement body detection/redaction
- [ ] Update `Http2StreamRedactor` for detection
- [ ] Test with real HTTP/2 streams
- [ ] Benchmark: measure latency per frame
- [ ] Verify header encoding not affected

---

## Comparative Analysis

### Chunk Sizes (Recommendations)

| Component | Chunk Size | Rationale |
|-----------|-----------|-----------|
| **scred-redactor** | 4-64 KB | File I/O optimal |
| **scred-mitm** | 4-16 KB | TCP MSS, latency |
| **scred-proxy** | Frame-size | HTTP/2 frame boundary |

### Memory Usage

| Component | Per-Detector | Per-Stream | Total |
|-----------|-------------|-----------|--------|
| **scred-redactor** | ~4 KB | N/A (single) | ~4 KB |
| **scred-mitm** | ~4 KB | Events buffer | ~20 KB/stream |
| **scred-proxy** | ~4 KB | Events buffer | ~20 KB/request |

### Latency Impact

| Component | Overhead | Why |
|-----------|----------|-----|
| **scred-redactor** | <1 ms/4KB | Batch processing |
| **scred-mitm** | <100 µs/packet | Per-packet streaming |
| **scred-proxy** | <50 µs/frame | Frame-level processing |

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scred_redactor_streaming() {
        let input = "api_key=AKIAIOSFODNN7EXAMPLE\n".as_bytes();
        let mut redactor = ZigRedactor::new(...).unwrap();
        let stats = redactor.redact_streaming().unwrap();
        
        assert_eq!(stats.total_matches, 1);
        assert_eq!(stats.by_pattern["aws-access-token"], 1);
    }

    #[test]
    fn test_scred_mitm_position_tracking() {
        let mut detector = StreamingDetector::new().unwrap();
        
        detector.process_packet(b"part1").unwrap();
        detector.process_packet(b"AKIAIOSFODNN7EXAMPLE").unwrap();
        
        let events = detector.drain_events();
        assert_eq!(events[0].absolute_position, 5); // After "part1"
    }

    #[test]
    fn test_scred_proxy_header_redaction() {
        let mut detector = Http2SecretDetector::new().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Bearer sk-proj-abc123...".parse().unwrap());
        
        let redactions = detector.process_headers(&headers).unwrap();
        assert!(redactions.contains_key("Authorization"));
    }
}
```

### Integration Tests

```bash
# Test scred-redactor
cargo test --test redactor_streaming -- --nocapture

# Test scred-mitm
cargo test --test mitm_detector -- --nocapture

# Test scred-proxy
cargo test --test http2_redaction -- --nocapture
```

### End-to-End Tests

```bash
# CLI: Redact a file
echo 'api_key=AKIAIOSFODNN7EXAMPLE' | scred redact
# Expected: api_key=xxxxxxxxxxxxxxxxxxxxxxxx

# MITM: Intercept and redact traffic
scred-mitm --listen 0.0.0.0:8443 --upstream api.openai.com

# HTTP/2 Proxy: Redact proxied requests
scred-proxy --listen 0.0.0.0:8080
```

---

## Performance Benchmarks

### Expected Results

```
Detector throughput: 60-600 MB/s
Per-chunk overhead: <1 ms

scred-redactor:  100 MB file → 150-1600 ms
scred-mitm:      1 Gbps stream → <1.7 µs/packet
scred-proxy:     10k req/s → <50 µs/request
```

---

## Migration Path

### Phase 1: scred-redactor (Lowest Risk)
- No network concerns
- Batch processing only
- Easy to rollback

### Phase 2: scred-proxy (Medium Risk)
- HTTP/2 only (controlled)
- Frame-level granularity
- Can toggle per-handler

### Phase 3: scred-mitm (Highest Impact)
- TLS interception
- Continuous streams
- Production traffic

---

## Troubleshooting

### Detector crashes on specific input
- Check pattern prefix for buffer overflow
- Validate min_len constraints
- Run with ASAN: `ASAN_OPTIONS=detect_leaks=1`

### High false positive rate
- Verify min_len settings
- Check for overlapping patterns
- Review recent pattern additions

### Performance degradation
- Profile with perf: `perf record -F 99 ./detector`
- Check chunk size optimization
- Monitor event buffer growth

---

## Deployment Checklist

- [ ] All components use latest detector version
- [ ] Streaming integration tested with real data
- [ ] Performance benchmarks meet SLA
- [ ] Audit logging configured
- [ ] Rollback plan documented
- [ ] Team trained on new patterns
- [ ] Monitoring alerts set up
- [ ] Customer communication planned

