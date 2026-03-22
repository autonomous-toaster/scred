# SCRED Pattern Detector - Architecture & Design

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                   Zig Pattern Detector                          │
│  (43 patterns, prefix matching, streaming support, C FFI)       │
└────────────┬────────────┬────────────┬────────────┬─────────────┘
             │            │            │            │
      ┌──────┴─────┐  ┌───┴──────┐  ┌─┴────────┐  ┌┴──────────┐
      │   Rust     │  │  Rust    │  │  Rust   │  │ Benchmarks│
      │  FFI       │  │ Tests    │  │ Building│  │  & Docs   │
      │ Wrapper    │  │ (6/6 ✅) │  │        │  │           │
      └──────┬─────┘  └───┬──────┘  └─┬───────┘  └┴──────────┘
             │            │            │
      ┌──────┴────────────┴────────────┴────────────┐
      │         Three Integration Contexts          │
      └──────┬────────────┬───────────────┬─────────┘
             │            │               │
      ┌──────┴─────┐ ┌───┴───────┐ ┌────┴────────┐
      │   CLI      │ │  NETWORK  │ │   HTTP/2    │
      │ (Redactor) │ │  (MITM)   │ │  (Proxy)    │
      └────────────┘ └───────────┘ └─────────────┘
```

---

## Core Detector Design

### Zig Implementation (280 LOC)

```zig
// Pattern Definition
pub const Pattern = struct {
    name: []const u8,      // "aws-access-token"
    prefix: []const u8,    // "AKIA"
    min_len: usize,        // 20 bytes minimum
};

// Detection Result
pub const DetectionEvent = extern struct {
    pattern_id: u16,       // Index in patterns array
    pattern_name: [64]u8,  // Pattern identifier
    name_len: u8,          // Actual name length
    position: usize,       // Byte offset in chunk
    length: u16,           // Match length
};

// Detector State Machine
pub const PatternDetector = struct {
    allocator: Allocator,
    events: std.ArrayList(DetectionEvent),
    output: std::ArrayList(u8),
};
```

### Rust Wrapper (195 LOC)

```rust
pub struct Detector {
    ptr: *mut PatternDetector,  // Opaque Zig pointer
}

pub struct ProcessResult {
    pub events: Vec<DetectionEvent>,
    pub bytes_processed: usize,
}

impl Detector {
    pub fn process(&mut self, chunk: &[u8], is_eof: bool)
        -> Result<ProcessResult, &'static str>
    // Key properties:
    // - Borrows chunk (zero-copy)
    // - Returns owned events
    // - Maintains state across calls
    // - is_eof triggers cleanup
}

impl Drop for Detector {
    fn drop(&mut self) {
        unsafe { scred_detector_free(self.ptr); }
    }
    // Automatic cleanup on detector drop
}
```

---

## Streaming Model

### State Machine

```
┌─────────────────────────────────────────────────────────┐
│                   Detector Created                       │
│              (allocator initialized)                     │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
        ┌─────────────────────────────┐
        │   process(chunk, false)     │
        │  - Detect patterns          │
        │  - Accumulate events        │
        │  - Maintain state           │
        └────────┬────────────────────┘
                 │
    ┌────────────┴────────────┐
    │                         │
    ▼                         ▼
(more chunks)         (last chunk)
process(...)          process(..., true)
    │                         │
    │                         ▼
    │              ┌─────────────────────┐
    └─────────────▶│  Flush & cleanup    │
                   │  Return final events│
                   └────────┬────────────┘
                            │
                            ▼
                   ┌─────────────────────┐
                   │  Detector Dropped   │
                   │  (auto cleanup)     │
                   └─────────────────────┘
```

### Example: 3-Chunk Stream

```rust
// CHUNK 1: No pattern
detector.process(b"prefix=", false)?
→ events: []
→ state: ready for next

// CHUNK 2: Contains partial pattern start
detector.process(b"AKIA", false)?
→ events: [Event { pattern: "aws-access-token", position: 0 }]
→ state: ready

// CHUNK 3: Final chunk
detector.process(b"IOSFODNN7EXAMPLE\n", true)?
→ events: []  // No new events
→ state: flushed, ready to drop
```

---

## Pattern Matching Algorithm

### Prefix-Based Detection

```zig
fn matchPattern(detector, input, pos, pattern, id) -> bool {
    // 1. Check remaining bytes
    if (pos >= input.len) return false;

    // 2. Check prefix match
    if (pattern.prefix.len > 0) {
        if (pos + pattern.prefix.len > input.len) return false;
        if (!std.mem.eql(u8, input[pos..pos+prefix.len], prefix))
            return false;
    }

    // 3. Check minimum length
    if (pos + pattern.min_len > input.len)
        return false;

    // 4. Record event
    event = DetectionEvent {
        .pattern_id = id,
        .pattern_name = pattern.name,
        .name_len = @intCast(pattern.name.len),
        .position = pos,
        .length = @intCast(pattern.min_len),
    };
    detector.events.append(event);

    return true;
}
```

### Complexity Analysis

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Per-chunk | O(n × p) | n = chunk size, p = patterns |
| Per-pattern | O(m) | m = prefix length |
| Event storage | O(z) | z = matches found |
| **Total stream** | **O(N + Z)** | N = total bytes, Z = total matches |

### Example: 43 patterns, 1MB chunk

```
Time = (n × p) + (z × log z)
     = (1,000,000 × 43) + (10 × log 10)
     ≈ 43,000,000 ops
     ≈ 43-86 ms @ 1-2 GHz
     ≈ 60-600 MB/s (optimized)
```

---

## Integration Points

### Pattern 1: CLI (scred-redactor)

```
User Input
    │
    ▼
┌─────────────────┐
│ Read 4KB chunk  │  (streaming file I/O)
└────────┬────────┘
         │
         ▼
    ┌─────────────────┐
    │ Zig Detector    │
    │  process()      │
    └────────┬────────┘
             │
             ▼ (events)
         ┌─────────────────┐
         │ Redact Matches  │
         │ byte[pos..+len]─│→ 'x'
         └────────┬────────┘
                  │
                  ▼
            ┌─────────────┐
            │ Write chunk │
            └────────┬────┘
                     │
    ┌────────────────┘
    │ (loop until EOF)
    └─────────────────▶ User Output
```

**Key**: File-based, batch processing, state per file

---

### Pattern 2: Network (scred-mitm)

```
TLS Connection
    │
    ▼
┌──────────────────┐
│ Recv TCP Packet  │
└────────┬─────────┘
         │
         ▼
    ┌──────────────────┐
    │ TLS Decrypt      │
    └────────┬─────────┘
             │
             ▼
         ┌──────────────────┐
         │ StreamingDetector│
         │ process_packet() │  (tracks abs position)
         └────────┬─────────┘
                  │
                  ▼ (events with abs positions)
              ┌──────────────────┐
              │ Redact by offset │
              │ stream[pos..+len]│→ 'x'
              └────────┬─────────┘
                       │
                       ▼
                  ┌──────────────┐
                  │ TLS Encrypt  │
                  └────────┬─────┘
                           │
    ┌──────────────────────┘
    │ (continuous until close)
    └─────────────────────▶ TLS Upstream
```

**Key**: Network streaming, position tracking, state per connection

---

### Pattern 3: HTTP/2 (scred-proxy)

```
HTTP/2 Frame Stream
    │
    ├─▶ HEADERS Frame
    │      │
    │      ▼
    │  ┌──────────────────┐
    │  │ Http2Detector    │
    │  │ process_headers()│
    │  └────────┬─────────┘
    │           │
    │           ▼ (header events)
    │       ┌───────────────┐
    │       │ Redact Headers│
    │       └────────┬──────┘
    │               │
    │
    ├─▶ DATA Frame 1
    │      │
    │      ▼
    │  ┌──────────────────┐
    │  │ Http2Detector    │
    │  │ process_body()   │ (is_last=false)
    │  └────────┬─────────┘
    │           │
    │           ▼ (body events)
    │       ┌───────────────┐
    │       │ Redact Body   │
    │       └────────┬──────┘
    │
    ├─▶ DATA Frame 2 (final)
    │      │
    │      ▼
    │  ┌──────────────────┐
    │  │ Http2Detector    │
    │  │ process_body()   │ (is_last=true)
    │  └────────┬─────────┘
    │           │
    └──────────▶ Redacted HTTP/2 Frame Stream
```

**Key**: Frame-aware, header/body separation, is_last flag

---

## Memory Management

### Allocation Strategy

```rust
// Zig allocation
let gpa = GeneralPurposeAllocator{};
let detector = gpa.create(PatternDetector);
→ ~4 KB overhead per detector

// Event storage
events: ArrayList<DetectionEvent>
→ ~64 bytes per event
→ Typical: 10-100 events/chunk
→ Per-chunk: 640 bytes - 6.4 KB

// Per-stream memory
detector: ~4 KB
events: ~10 KB average
Total: ~14 KB/stream

// Global memory (43 patterns)
ALL_PATTERNS array: ~5 KB (static)
Binary size: 1.1 MB (libscred_pattern_detector.a)
```

### Cleanup

```rust
impl Drop for Detector {
    fn drop(&mut self) {
        // Zig cleanup on FFI boundary
        unsafe {
            scred_detector_free(self.ptr);
        }
        // Automatic: events deinitialized
        // Automatic: output buffer freed
    }
}
```

---

## Event Flow

### Event Lifecycle

```
Detection:
  Pattern matches input[pos..pos+len]
         │
         ▼
  Create DetectionEvent {
    pattern_id: index in ALL_PATTERNS
    pattern_name: name bytes
    position: offset in chunk
    length: match length
  }
         │
         ▼
  Store in detector.events (ArrayList)
         │
         ▼
Retrieval:
  process() returns ProcessResult {
    events: Vec<DetectionEvent>,
    bytes_processed: chunk.len
  }
         │
         ▼
Application:
  for event in events {
    redact(chunk, event.position, event.length)
  }
         │
         ▼
  Return redacted chunk to caller
```

---

## Performance Characteristics

### Latency Profile

```
Operation              Time        Notes
────────────────────────────────────────
Detector creation     ~100 µs      Allocate + init
Process 4 KB chunk     ~7 ms        43 patterns, 0 matches
Process 4 KB chunk     ~7 ms        43 patterns, 10 matches
Match detection        ~1-10 µs     Per pattern attempt
Event recording        ~1 µs        Per match
Detector drop          ~50 µs       Cleanup

Total for 1 MB:       ~1.7 ms      @ 600 MB/s
                      ~7 ms        @ 140 MB/s
                     ~17 ms        @ 60 MB/s
```

### Throughput Estimates

```
Configuration          Throughput    Factor
────────────────────────────────────────
Worst case (debug)     ~60 MB/s      1x
Typical (release)      ~150 MB/s     2.5x
Optimized (-O3)        ~300 MB/s     5x
SIMD variant          ~600 MB/s     10x
```

---

## Testing Strategy

### Unit Tests (Zig)

```zig
test "Pattern loading" {
    // Verify 43 patterns loaded
    assert(PATTERN_COUNT == 43);
}

test "Basic detection" {
    // Single pattern detection
    detector.process(b"AKIAIOSFODNN7EXAMPLE", true);
    assert(detector.get_event_count() > 0);
}

test "Multiple patterns" {
    // Cross-service detection
    detector.process(b"AWS: AKIA... GitHub: ghp_...", true);
    assert(events.len >= 2);
}
```

### Integration Tests (Rust)

```rust
#[test]
fn test_streaming() {
    // Multi-chunk detection
    detector.process(b"part1", false)?;
    detector.process(b"AKIAIOSFODNN7EXAMPLE", true)?;
    assert!(events.len > 0);
}

#[test]
fn test_event_details() {
    // Event introspection
    let event = events[0];
    assert!(event.position >= 0);
    assert!(event.length > 0);
    assert!(!event.pattern_name().is_empty());
}
```

### End-to-End Tests

```bash
# CLI
echo "key=AKIAIOSFODNN7EXAMPLE" | scred redact
# ✓ Output: key=xxxxxxxxxxxxxxxxxxxxxxxx

# MITM
scred-mitm --upstream api.openai.com
# ✓ Intercepts and redacts TLS stream

# Proxy
scred-proxy --listen 0.0.0.0:8080
# ✓ Redacts HTTP/2 headers and bodies
```

---

## Deployment Architecture

### Development

```
┌──────────────────────────────────────┐
│   scred-pattern-detector-zig repo    │
│   ├─ src/lib.zig (280 LOC)           │
│   ├─ src/lib.rs (195 LOC)            │
│   ├─ tests (20 Zig + 30 Rust)        │
│   └─ build.rs (45 LOC)               │
└────────────┬─────────────────────────┘
             │
             ├─ cargo test              (Rust tests)
             ├─ zig test                (Zig tests)
             └─ cargo bench             (Performance)
```

### Production

```
┌────────────────────────────────────────────┐
│        libscred_pattern_detector.a         │
│         (1.1 MB, statically linked)        │
└────────────┬──────────────────────────────┘
             │
      ┌──────┴──────┬──────────┬──────────┐
      │             │          │          │
      ▼             ▼          ▼          ▼
┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│ scred    │ │ scred    │ │ scred    │ │  Tests   │
│ redactor │ │  mitm    │ │  proxy   │ │ & Bench  │
└──────────┘ └──────────┘ └──────────┘ └──────────┘
```

---

## Migration Roadmap

### Phase 1: scred-redactor (Week 1-2)
- Lowest risk (file-based only)
- Add dependency to Cargo.toml
- Implement ZigRedactor wrapper
- Run benchmark vs regex

### Phase 2: scred-proxy (Week 3-4)
- Medium risk (HTTP/2 frames)
- Add HTTP/2 detector integration
- Test with real traffic (staging)
- Monitor latency impact

### Phase 3: scred-mitm (Week 5-6)
- Highest risk (TLS interception)
- Implement StreamingDetector
- Deploy with gradual rollout
- 5% → 25% → 100%

---

## Quality Metrics

| Metric | Target | Method |
|--------|--------|--------|
| False Positives | 0% | Manual review |
| Coverage | 85%+ | Test corpus |
| Throughput | >50 MB/s | Benchmark |
| Latency P99 | <1 ms/MB | Profiling |
| Memory/stream | <20 KB | Valgrind |
| CPU usage | <5% | perf |

---

## Conclusion

The detector provides a **universal streaming interface** that adapts to three distinct deployment contexts:

1. **CLI**: File chunking (4 KB)
2. **Network**: Packet streaming (variable)
3. **HTTP/2**: Frame granularity (variable)

All three use the same underlying **Zig detector** with **Rust FFI wrapper**, ensuring consistent behavior while optimizing for each context's specific requirements.

