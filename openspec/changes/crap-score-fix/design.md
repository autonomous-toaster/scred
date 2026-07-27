## Architecture

### Current State: God Functions
```
┌────────────────────────────────────────────┐
│  handle_h2_client_transcoding (662 lines)  │
│  ┌──────────────────────────────────────┐  │
│  │ • HTTP/2 frame parsing loop          │  │
│  │ • Frame type dispatch (8+ types)     │  │
│  │ • H2 ↔ H1.1 transcoding              │  │
│  │ • Chunked encoding handling          │  │
│  │ • Content-Length handling            │  │
│  │ • HPACK encoding/decoding            │  │
│  │ • Error handling per path            │  │
│  │ • Network I/O throughout             │  │
│  └──────────────────────────────────────┘  │
│         ↓ CC=87, Coverage=0%, CRAP=7656    │
└────────────────────────────────────────────┘
```

### Target State: Extracted Layers
```
┌─────────────────────────────────────────────┐
│  handle_h2_client_transcoding (refactored)  │
│  ┌───────────────────────────────────────┐  │
│  │ Orchestrates:                         │  │
│  │ • read_h2_frames()                    │  │
│  │ • dispatch_frame_type()               │  │
│  │ • transcode_headers()                 │  │
│  │ • transcode_body()                    │  │
│  └───────────────────────────────────────┘  │
│         ↓ CC=15, Coverage=60%, CRAP=22      │
└─────────────────────────────────────────────┘
           ↓ calls
┌─────────────────────────────────────────────┐
│  Pure Helper Functions (testable)           │
│  • parse_frame_type()                       │
│  • decode_hpack_headers()                   │
│  • encode_chunked_body()                    │
│  • determine_frame_strategy()               │
│         ↓ CC=5-10, Coverage=90%             │
└─────────────────────────────────────────────┘
```

## Extraction Strategy

### Pattern: Separate I/O from Logic
```rust
// BEFORE: Tightly coupled
async fn handle_h2_client_transcoding<S>(conn: S, ...) {
    // I/O: Read frame
    let mut header = [0u8; 9];
    conn.read_exact(&mut header).await?;
    
    // Logic: Parse frame
    let frame = Frame::parse(&header)?;
    
    // I/O: Write response
    conn.write_all(&response).await?;
}

// AFTER: Separated
async fn handle_h2_client_transcoding<S>(conn: S, ...) {
    let header = read_frame_header(&mut conn).await?;
    let frame = parse_frame(&header)?;  // Pure function
    let response = transcode_frame(&frame)?;  // Pure function
    write_response(&mut conn, &response).await?;
}

// Testable pure functions
fn parse_frame(header: &[u8; 9]) -> Result<Frame> { ... }
fn transcode_frame(frame: &Frame) -> Result<TranscodedFrame> { ... }
```

### Extraction Candidates (Top 10)

#### 1. handle_h2_client_transcoding (CC: 87 → target: 15)
**Extract:**
- `parse_h2_frame()` - Parse frame header and payload
- `dispatch_frame_type()` - Route to frame-specific handler
- `transcode_headers_frame()` - H2 headers → H1.1 headers
- `transcode_data_frame()` - H2 data → H1.1 body
- `handle_settings_frame()` - SETTINGS frame logic
- `handle_goaway_frame()` - GOAWAY frame logic

#### 2. handle_single_request (CC: 61 → target: 12)
**Extract:**
- `extract_host_from_request()` - Host extraction logic
- `determine_upstream_addr()` - Upstream address resolution
- `select_protocol()` - H2 vs H1.1 selection
- `select_redaction_mode()` - Redaction strategy

#### 3. handle_http_proxy (CC: 35 → target: 10)
**Extract:**
- `parse_proxy_request()` - Request parsing
- `forward_to_upstream()` - Proxy forwarding logic

## Testing Strategy

### Integration Tests (Characterization)
```rust
#[tokio::test]
async fn test_h2_transcoding_headers_frame() {
    // Set up mock H2 client and server
    // Send HEADERS frame
    // Verify transcoded H1.1 request
    // Ensures behavior preserved during refactoring
}
```

### Unit Tests (Extracted Functions)
```rust
#[test]
fn test_parse_frame_headers() {
    let header = [0x00, 0x00, 0x10, 0x01, ...];
    let frame = parse_frame(&header).unwrap();
    assert_eq!(frame.frame_type, FrameType::Headers);
}

#[test]
fn test_transcode_headers_to_h11() {
    let h2_headers = vec![(":method", "GET"), (":path", "/")];
    let h11 = transcode_headers(h2_headers);
    assert!(h11.starts_with("GET / HTTP/1.1"));
}
```

## CRAP Reduction Math

**Current:** CC=87, Coverage=0% → CRAP = 87² × 1³ = 7569

**Target:** CC=15, Coverage=60% → CRAP = 15² × (1-0.6)³ = 225 × 0.064 = 14.4 ✅

**How:**
- Extract 10-12 helper functions (each CC=5-8)
- Main function becomes orchestrator (CC=10-15)
- Write tests for extracted functions (60%+ coverage)
- Integration tests cover main function paths

## Implementation Phases

### Phase 1: Compilation Fixes (1-2 days)
- Fix all 66 compilation errors
- Verify `cargo check` passes
- No refactoring yet, just make it compile

### Phase 2: Test Infrastructure (1 day)
- Create test utilities for async I/O mocking
- Set up integration test framework
- Create mock HTTP/2 servers

### Phase 3: Top 10 Refactoring (5-7 days)
For each function:
1. Write characterization test (2-3 hours)
2. Identify extraction points (1 hour)
3. Extract helper functions (2-3 hours)
4. Write unit tests for helpers (2 hours)
5. Verify CRAP score drops (30 min)

### Phase 4: Verification (1 day)
- Run `cargo crap --workspace`
- Verify all top 10 below threshold
- Run full test suite
- Run `just ci`

## Risks

**High Risk:**
- Extracting from 662-line function without breaking behavior
- **Mitigation:** Characterization tests first

**Medium Risk:**
- Async I/O mocking complexity
- **Mitigation:** Use existing mock frameworks, keep mocks simple

**Low Risk:**
- Compilation fixes are straightforward
- **Mitigation:** Fix one error at a time, verify after each
