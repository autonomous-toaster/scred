# h2 Migration Implementation - Phase 1 Progress

**Status**: Phase 1.1 Complete ✅ | Phase 1.2 In Progress  
**Timeline**: 2026-03-22  
**Goal**: Migrate MITM and Proxy to h2 crate with shared redaction adapter

---

## What We've Done (Phase 1.1)

### ✅ H2MitmAdapter Module Created (250 LOC)

Located: `crates/scred-http/src/h2_adapter/mod.rs`

**Key Components**:

1. **H2MitmAdapter Struct** - Main adapter for both MITM and Proxy
   - Holds redaction engine
   - Manages per-stream redaction state
   - Provides clean API for header/body redaction

2. **StreamRedactionState** - Per-stream tracking
   - Individual `StreamingRedactor` per stream
   - Lookahead buffer for streaming processing
   - Headers redacted flags
   
3. **Public API Methods**:
   - `new()` - Create adapter
   - `redact_request_headers()` - Intercept request headers
   - `redact_response_headers()` - Intercept response headers
   - `redact_request_body()` - Intercept request body chunks
   - `redact_response_body()` - Intercept response body chunks
   - `cleanup_stream()` - Clean up when stream ends
   - `get_stats()` / `reset_stats()` - Monitoring

4. **AdapterStats** - Built-in monitoring
   - total_streams - Cumulative stream count
   - total_headers_redacted - Headers that were modified
   - total_bytes_redacted - Bytes processed with modifications
   - active_streams - Current open streams

### ✅ Tests Passing (3/3)
```
test_adapter_creation         ✅ OK
test_stream_lifecycle         ✅ OK
test_header_redaction         ✅ OK
```

### ✅ h2 Dependency Added
```toml
h2 = "0.4"  # RFC 7540 HTTP/2 implementation
```

---

## Architecture: Shared Adapter Layer

```
┌──────────────────────────────────────────────────────────┐
│         MITM Proxy & Forward Proxy                       │
├──────────────────────────────────────────────────────────┤
│                                                          │
│    ┌────────────────────────────────────────────────┐   │
│    │      H2MitmAdapter (250 LOC)                   │   │
│    │  • Per-stream redaction management             │   │
│    │  • Header/body interception                    │   │
│    │  • Streaming redaction support                 │   │
│    │  • Statistics/monitoring                       │   │
│    └────────────────────────────────────────────────┘   │
│                      ▲                                   │
│                      │                                   │
│        Used by both MITM and Proxy                      │
│                                                          │
├──────────────────────────────────────────────────────────┤
│                                                          │
│    MITM: h2 server + h2 client                          │
│    Proxy: h2 client for upstream                        │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

**Benefits**:
- Single source of truth for HTTP/2 redaction logic
- Both MITM and Proxy get same redaction behavior
- Easy to maintain and extend
- Proven pattern for both use cases

---

## What's Coming (Phase 1.2+)

### Phase 1.2: MITM Integration (Next)
- [ ] Port `crates/scred-mitm/src/mitm/tls_mitm.rs` to use H2MitmAdapter
- [ ] Replace custom HTTP/2 handling with h2 + adapter
- [ ] Test with basic curl HTTP/2 request
- [ ] Verify redaction works

### Phase 1.3: Proxy Integration
- [ ] Port `crates/scred-proxy/src/main.rs` to use H2MitmAdapter
- [ ] Forward proxy mode with H2MitmAdapter
- [ ] Test proxy connectivity

### Phase 2: Full Redaction Integration
- [ ] Stream-aware header redaction
- [ ] Body chunk redaction with lookahead
- [ ] Streaming performance optimization

### Phase 3: Testing & Validation
- [ ] Integration tests (curl + httpbin)
- [ ] Both MITM and Proxy modes
- [ ] Performance comparison
- [ ] Edge case coverage

### Phase 4: Cleanup
- [ ] Remove 14 custom HTTP/2 modules
- [ ] Remove custom Huffman decoder
- [ ] Remove custom HPACK implementation
- [ ] Update documentation
- [ ] Final validation

---

## H2MitmAdapter API Reference

### Creating Adapter
```rust
use scred_http::h2_adapter::H2MitmAdapter;
use scred_redactor::{RedactionEngine, RedactionConfig};
use std::sync::Arc;

let config = RedactionConfig { enabled: true };
let engine = Arc::new(RedactionEngine::new(config));
let adapter = H2MitmAdapter::new(engine);
```

### Redacting Headers
```rust
use http::Request;

let mut request = Request::builder()
    .method("GET")
    .uri("http://example.com")
    .body(())
    .unwrap();

// Add headers with potential secrets
request.headers_mut().insert(
    "authorization",
    "Bearer sk_live_secret_token".parse().unwrap(),
);

// Redact headers
adapter.redact_request_headers(stream_id, &mut request).await?;
// Authorization header is now redacted
```

### Redacting Body Chunks
```rust
// Process streaming body chunks
let body_chunk = b"customer_api_key: sk_live_secret";

// Not final chunk (more data coming)
let redacted = adapter.redact_request_body(stream_id, body_chunk, false).await?;

// Final chunk
let redacted = adapter.redact_request_body(stream_id, body_chunk, true).await?;
```

### Stream Cleanup
```rust
// When stream closes
adapter.cleanup_stream(stream_id).await;
```

### Monitoring
```rust
let stats = adapter.get_stats().await;
println!("Total streams: {}", stats.total_streams);
println!("Active streams: {}", stats.active_streams);
println!("Headers redacted: {}", stats.total_headers_redacted);
println!("Bytes redacted: {}", stats.total_bytes_redacted);
```

---

## Key Design Decisions

### 1. Async Throughout
- All methods are `async`
- Uses `tokio::sync::Mutex` for thread-safe state
- Compatible with tokio runtime

### 2. Per-Stream State
- Each stream has its own `StreamingRedactor`
- Lookahead buffer for streaming processing
- Automatic cleanup on stream end

### 3. Streaming Redaction
- Uses `StreamingRedactor::process_chunk()` API
- Maintains lookahead buffer between chunks
- Returns tuple: (redacted_text, pattern_count, bytes_redacted)

### 4. Statistics Tracking
- Built-in stats for monitoring
- Useful for debugging and performance analysis
- Can be reset independently

### 5. Error Handling
- Graceful fallback: if redaction fails, return original data
- All methods return `Result<T>` for propagation
- Structured logging via tracing

---

## Test Coverage

All adapter tests pass:

```
✅ test_adapter_creation
   - Verifies adapter initializes correctly
   - Checks initial stats are zero

✅ test_stream_lifecycle
   - Tests stream creation
   - Verifies stats update
   - Tests stream cleanup
   - Verifies stats updated again

✅ test_header_redaction
   - Creates request with secret header
   - Applies redaction
   - Verifies header is modified
```

---

## Compilation Status

- [x] h2 dependency added
- [x] H2MitmAdapter module created
- [x] Tests compile and pass
- [x] No compilation errors
- [x] Ready for MITM integration

---

## Next Immediate Steps

1. **Phase 1.2 - MITM Integration**
   - Port tls_mitm.rs to use H2MitmAdapter
   - Replace custom HTTP/2 with h2 + adapter
   - Test with curl

2. **Phase 1.3 - Proxy Integration**
   - Port scred-proxy/main.rs
   - Test forward proxy mode

3. **Phase 2 - Full Redaction**
   - Stream-aware redaction hooks
   - Performance optimization

---

## Files Created/Modified

**New Files**:
- ✅ `crates/scred-http/src/h2_adapter/mod.rs` (250 LOC)

**Modified Files**:
- ✅ `crates/scred-http/Cargo.toml` (added h2 = "0.4")
- ✅ `crates/scred-http/src/lib.rs` (exported h2_adapter module)

**Commits**:
- ✅ 917ce80 - Phase 1.1: Add h2 dependency and create H2MitmAdapter

---

## Success Criteria Met So Far

- ✅ h2 dependency integrated
- ✅ H2MitmAdapter module created
- ✅ Per-stream redaction state management
- ✅ Header/body redaction methods
- ✅ Stream lifecycle management
- ✅ Statistics tracking
- ✅ Unit tests passing
- ✅ No compilation errors

---

## Assessment Completed ✅

From TODO-7d4d5202:

- ✅ Assessment complete - h2 is 100% compatible
- ✅ Decision made - migrate to h2 with adapter layer
- ✅ Architecture designed - shared H2MitmAdapter
- ✅ Phase 1.1 implemented - adapter module working
- ▶️ Phase 1.2 next - MITM integration
- ⏳ Phase 1.3 - Proxy integration
- ⏳ Phase 2 - Full redaction
- ⏳ Phase 3 - Testing
- ⏳ Phase 4 - Cleanup

---

## Status Summary

**What works**: H2MitmAdapter is a solid, tested foundation for both MITM and Proxy to use for HTTP/2 redaction.

**What's next**: Integrate adapter into MITM handler, then Proxy, then full redaction pipeline.

**Timeline**: 2-3 days to complete full migration (from assessment).
