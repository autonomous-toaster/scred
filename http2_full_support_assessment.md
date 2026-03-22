# Full HTTP/2 Support Assessment for SCRED

## Current State
- **Phase 1**: HTTP/2 transparent downgrade to HTTP/1.1 ✅ WORKING
  - Clients see HTTP/1.1 only
  - No multiplexing, no frame handling
  - Works but not optimal

## Requirement: FULL HTTP/2 SUPPORT
Must support HTTP/2 natively with:
- Stream multiplexing (concurrent requests over single connection)
- Header compression (HPACK)
- Server push (optional)
- Flow control
- Stream prioritization

## Key Gotchas & Challenges

### 1. Per-Stream Redaction State (CRITICAL)
**Problem**: HTTP/2 multiplexes multiple requests on same connection
- Each stream has independent headers + body
- Redaction state CANNOT be shared between streams
- Current: Single `StreamingRedactor` per connection ❌

**Solution Required**:
```
Connection-level multiplexer:
├─ Stream 1: Independent StreamingRedactor + state
├─ Stream 3: Independent StreamingRedactor + state
├─ Stream 5: Independent StreamingRedactor + state
└─ Stream 7: Independent StreamingRedactor + state
```

### 2. Stream State Machine
**Problem**: HTTP/2 streams have complex lifecycle
```
IDLE → OPEN (after receiving headers) → HALF_CLOSED_LOCAL/REMOTE → CLOSED
```

**Issue**: Must track:
- Which headers are received
- Which body chunks are received
- When stream ends (END_STREAM flag)
- Stream window size (flow control)
- Stream priority

### 3. Connection Pooling
**Problem**: Need to maintain HTTP/2 connections to upstream
- One connection per upstream host (if keep-alive)
- Multiplex multiple downstream requests to single upstream connection
- Handle connection close/reset gracefully

**Current**: One upstream connection per CONNECT tunnel ❌
**Need**: Connection pool by hostname

### 4. Flow Control & Backpressure
**Problem**: HTTP/2 has window-based flow control
- Connection-level window (default 65535 bytes)
- Stream-level window (default 65535 bytes)
- Each DATA frame consumes window
- Must send WINDOW_UPDATE to resume flow

**Issue for redaction**:
- Can't buffer entire stream (may be 1GB+)
- Must process & redact as chunks arrive
- Must respect window sizes when forwarding

### 5. Header Compression (HPACK)
**Problem**: Headers compressed with HPACK context
- Decoder state accumulates across entire connection
- Decompression context must be maintained
- Header ordering matters for decompression

**Current**: Using `http2` crate handles this ✅
**Caveat**: Must use same decoder for all frames on connection

### 6. Stream Prioritization
**Problem**: Streams can have priority/weight
- Affects how upstream multiplexes sending
- Lower priority streams may be delayed
- Affects perceived latency

**Impact**: Medium (can ignore initially)

### 7. Server Push
**Problem**: Server can push responses for streams client didn't request
- Client must accept or reject with RST_STREAM
- Complicates redaction (unexpected body)

**Impact**: Low (rarely used, can disable with SETTINGS)

## Architecture Options

### Option A: Full Streaming Multiplexer (RECOMMENDED)
```rust
pub struct H2Multiplexer {
    connection: h2::server::Connection,
    streams: HashMap<u32, StreamRedactionState>,
    upstream_pool: ConnectionPool,
}

struct StreamRedactionState {
    stream_id: u32,
    headers_buffer: Vec<u8>,
    body_redactor: StreamingRedactor,
    window_size: u32,
    state: StreamState,
}
```

**Pros**:
- True multiplexing
- Independent per-stream redaction
- Optimal throughput

**Cons**:
- Complex state management (~500-800 LOC)
- Flow control implementation needed
- More edge cases

**Effort**: 30-40 hours (2-3 days at 8h/day)
**Risk**: Medium (many edge cases)

### Option B: Sequential Stream Processing
```rust
// Handle one stream at a time, decode to HTTP/1.1
// Send to redaction, re-encode to HTTP/2
// Wait for stream completion before next
```

**Pros**:
- Simpler implementation
- Reuses existing redaction logic
- Less state management

**Cons**:
- NO multiplexing (defeats purpose of h2)
- Throughput limited to sequential processing
- Client sees latency

**Effort**: 15-20 hours
**Risk**: Low (simple, known patterns)

**Note**: This defeats the purpose of HTTP/2. NOT RECOMMENDED.

### Option C: Hybrid (Initial Implementation)
```rust
// Phase 2a: Accept h2 frames, buffer headers/body per stream
// Send batches to per-stream redactors (no true multiplexing yet)
// Re-encode and send back
// 
// Phase 2b: Add true multiplexing with concurrent processing
```

**Pros**:
- Smaller initial scope
- Paths exist for optimization
- Can start shipping

**Cons**:
- Intermediate state (not full h2)
- Needs refactoring later

**Effort**: 20-25 hours initial, then +15-20 more

## Comparison with rust-rpxy

rust-rpxy approach:
- Uses `hyper` HTTP library
- `hyper::Client` handles h2 automatically
- Separate client for h2c vs h1
- Selection based on `req.version()`
- No custom multiplexer needed (hyper does it)

**Why we can't copy this**:
- We need to REDACT streams (they don't)
- We need PER-STREAM redaction state (they need per-connection routing)
- hyper's h2 is transparent but doesn't expose stream handles for redaction

## What We Need to Build

### 1. H2 Stream Decoder
- Use `h2` crate for frame handling ✅
- Demultiplex frames by stream_id
- Reconstruct headers + body per stream
- Track flow control windows

**Crate**: `h2` v0.4+ (RFC 9113 compliant)

### 2. Per-Stream Redaction State Manager
```rust
pub struct StreamRedactionManager {
    streams: Arc<Mutex<HashMap<u32, StreamRedactionState>>>,
    upstream_connector: Arc<UpstreamConnector>,
}

impl StreamRedactionManager {
    async fn create_stream(&self, stream_id: u32, headers: &[HeaderField]) -> Result<()>;
    async fn append_data(&self, stream_id: u32, data: &[u8]) -> Result<()>;
    async fn end_stream(&self, stream_id: u32) -> Result<()>;
    async fn get_response(&self, stream_id: u32) -> Result<ResponseFrames>;
}
```

### 3. Upstream Connection Pool
```rust
pub struct UpstreamH2Pool {
    pools: HashMap<String, Vec<h2::SendRequest>>,
}

impl UpstreamH2Pool {
    async fn get_or_create(&self, host: &str) -> Result<h2::SendRequest>;
    async fn send_request(&self, host: &str, req: Request) -> Result<Response>;
}
```

### 4. Flow Control Handler
```rust
pub struct FlowController {
    connection_window: u32,
    stream_windows: HashMap<u32, u32>,
}

impl FlowController {
    fn consume_window(&mut self, stream_id: u32, bytes: usize) -> Result<()>;
    fn restore_window(&mut self, stream_id: u32) -> Result<Vec<WindowUpdate>>;
}
```

## Files to Create/Modify

```
crates/scred-http/src/h2/
├── mod.rs (already exists)
├── alpn.rs (already exists, MODIFY - re-enable h2)
├── frame.rs (already exists)
├── stream_state.rs (CREATE - per-stream state machine)
├── stream_manager.rs (CREATE - manages all streams)
├── flow_control.rs (CREATE - window management)
└── upstream_pool.rs (CREATE - connection pooling)

crates/scred-mitm/src/mitm/
├── tls_mitm.rs (MODIFY - route to h2 multiplexer)
├── h2_mitm.rs (CREATE - main h2 handler)
└── h2_upstream.rs (CREATE - upstream h2 client)

crates/scred-redactor/src/
└── (NO CHANGES - redaction logic unchanged)
```

## Implementation Phases

### Phase 2a: Stream Demultiplexing (15h)
- Accept h2 frames
- Demultiplex by stream_id
- Buffer headers + body per stream
- Implement state machine (IDLE → OPEN → CLOSED)

**Tests**: Frame routing, state transitions, ordering guarantees

### Phase 2b: Per-Stream Redaction (20h)
- Create StreamingRedactor per stream
- Apply redaction to each stream independently
- Track stream lifecycle (headers → body → END_STREAM)
- Collect responses per stream

**Tests**: Independent streams, stream isolation, concurrent redaction

### Phase 2c: Upstream Pooling (15h)
- H2 client pool by hostname
- Connection reuse
- Handling connection close/reset
- Fallback to new connection if needed

**Tests**: Pool creation, connection reuse, error handling

### Phase 2d: Flow Control (10h)
- Window tracking (connection + stream)
- WINDOW_UPDATE handling
- Backpressure when windows full
- Buffer management under flow control

**Tests**: Window exhaustion, UPDATE frames, backpressure scenarios

### Phase 2e: Integration & Testing (10h)
- End-to-end h2 tests with curl --http2
- Concurrent streams (curl --parallel)
- Large file transfers
- Error scenarios (closed streams, RST_STREAM)

**Tests**: Real httpbin.org requests, concurrency, stress

## Total Effort: 70-80 hours (1-1.5 weeks at 8h/day)

## Gotchas & Caveats

1. **Stream Ordering**: Responses must match upstream, but can arrive out-of-order due to multiplexing
   - Solution: Queue responses, return in order

2. **Error Handling**: Stream reset (RST_STREAM) vs connection close have different semantics
   - Solution: Per-stream error states + connection-level recovery

3. **Flow Control**: Easy to deadlock if not careful with window management
   - Solution: Monitor window sizes, send UPDATE proactively

4. **Memory**: Buffering multiple streams can consume significant memory
   - Solution: Stream-level backpressure, configurable limits

5. **Latency**: Multiplexing reduces latency only if streams are truly concurrent
   - Solution: Ensure async processing, don't block on one stream

6. **Dependencies**: http2 crate is relatively stable but less battle-tested than hyper
   - Solution: Comprehensive testing, gradual rollout

7. **HPACK Context**: Decompression state is cumulative across connection
   - Solution: Maintain single decoder per connection, decode headers eagerly

## Decision Points

### Should We Implement Phase 2?
- **YES if**: Users need high-throughput http2 multiplexing
- **NO if**: Phase 1 downgrade is sufficient for 99% of use cases

### Should We Use `http2` Crate vs `hyper`?
- **http2**: Lower-level control, required for custom per-stream redaction
- **hyper**: Simpler but less control over streams
- **Decision**: Use `http2` crate (we need fine-grained control)

### Connection Pooling: Per-Host or Global?
- **Per-host**: Follows HTTP/2 semantics, cleaner isolation
- **Global**: Simpler, higher connection reuse
- **Decision**: Per-host (safer, matches HTTP semantics)

