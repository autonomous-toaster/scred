# HTTP/2 Support for SCRED - Design & Implementation Plan

**Status**: Phase 1 in progress (stream_state.rs complete)  
**Effort**: 50-70 hours total, 2-3 weeks  
**Goal**: Full HTTP/2 native support (no downgrade) with proper secret redaction

## Architecture: Per-Stream Redaction

SCRED uses per-stream redaction state to support HTTP/2 multiplexing natively:

```
HTTP/2 Client (multi-stream)
    ↓ Stream 1 → StreamRedactionState { redactor_1, headers_1, body_1 }
    ↓ Stream 3 → StreamRedactionState { redactor_2, headers_2, body_2 }
    ↓ Stream 5 → StreamRedactionState { redactor_3, headers_3, body_3 }
    ↓
Demultiplex → Apply per-stream redaction → Remultiplex
    ↓
HTTP/2 Response (multiplexed, redacted)
```

**Key Principle**: Each stream has independent redaction context. No cross-stream contamination.

## Phase 1: Client-Side HTTP/2 Handler (15-20h)

### Completed

- [x] `stream_state.rs` (200 LOC, 3 tests)
  - `StreamRedactionState` struct: Independent redaction for each stream
  - `add_headers()`: Buffer HTTP/2 headers
  - `add_body_chunk()`: Buffer body data
  - `apply_redaction()`: Redact when stream complete
  - `stats()`: Logging and monitoring

### In Progress

- [ ] `server.rs` (200 LOC) - H2ServerConnection wrapper
  - Handle h2 client connections
  - Accept streams from client
  - Forward to upstream
  
- [ ] `h2_mitm.rs` (400 LOC) - Main demux/redact/remux logic
  - Demultiplex client streams
  - Route to StreamRedactionState
  - Remultiplex responses back
  - Error handling and cleanup

- [ ] Integration in `tls_acceptor.rs`
  - Route h2 clients to h2_mitm (not downgrade)

### Tests
- Unit tests for stream state isolation
- Stream completion detection
- Redaction application

## Phase 2: Upstream HTTP/2 Support (15-20h)

- [ ] `client.rs` (200 LOC) - H2ClientConnection wrapper
  - Connect to HTTP/2 upstream servers
  - Send requests, receive responses
  
- [ ] `stream_mapper.rs` (100 LOC) - Stream ID mapping
  - Map client stream IDs to upstream stream IDs
  - Handle stream creation/cleanup

### Tests
- Upstream connection pooling
- Stream reuse
- Concurrent request handling

## Phase 3: Proxy Multiplexing (10-15h)

- [ ] Connection pooling
- [ ] Stream optimization
- [ ] Integration with proxy mode (not just MITM)

## Key Design Decisions

### 1. No Downgrade (User Requirement)
- HTTP/2 clients stay on HTTP/2
- No transparent fallback to HTTP/1.1
- Simplifies architecture vs transcode approach

### 2. Redaction Invariant
```
Regardless of protocol (h1 or h2):
  Redaction Input  = HTTP/1.1 text format
  Redaction Output = HTTP/1.1 text format
  Redaction Logic  = Completely unchanged
```

This means:
- Convert h2 headers → http/1.1 text
- Apply existing redaction patterns
- Convert back to h2 frame format

### 3. All HTTP/2 Code in scred-http
- Reusable for both MITM and proxy modes
- Easy to find and maintain
- Clear separation of concerns

## Testing Strategy

### Unit Tests
- Stream state creation and isolation
- Header buffering
- Body chunk accumulation
- Redaction application
- Error cases (duplicate headers, data after END_STREAM)

### Integration Tests
- Multiple concurrent streams
- Per-stream redaction isolation
- Upstream h2 server connectivity
- End-to-end redaction verification

### Manual Testing
```bash
curl --http2 -x http://localhost:9999 https://example.com/api?key=secret
```

## Performance

- **Buffering overhead**: ~2-3ms per stream (acceptable for HTTP/2)
- **Multiplexing**: No regression (single connection, multiple streams)
- **Memory**: Bounded by stream limit (typically 100-1000 concurrent streams)

## Success Criteria

✅ HTTP/2 clients work without downgrade  
✅ Secrets redacted in all h2 streams  
✅ Multiple concurrent streams isolated  
✅ Upstream h2 servers supported  
✅ <5ms overhead per stream  
✅ Zero HTTP/1.1 regression  

## File Organization

```
scred-http/src/h2/
├── alpn.rs (110 LOC) - ALPN detection
├── frame.rs (360 LOC) - Frame parsing (reference)
├── stream_state.rs (200 LOC) - Per-stream redaction state
├── server.rs (200 LOC) - H2 client handler
├── client.rs (200 LOC) - H2 upstream handler
├── stream_mapper.rs (100 LOC) - Stream ID mapping
└── mod.rs - Exports

scred-mitm/src/mitm/
├── h2_mitm.rs (400 LOC) - Main demux/remux logic
└── tls_acceptor.rs - Route h2 to h2_mitm
```

## Timeline

- **Week 1**: Phase 1 (client handler + integration) - 15-20h
- **Week 2**: Phase 2 (upstream support) - 15-20h
- **Week 3**: Phase 3 (proxy mode + optimization) - 10-15h
- **Buffer**: Testing, docs, bug fixes - 10-15h

**Total**: 2-3 weeks for full production-ready support

## References

- [HTTP/2 RFC 7540](https://tools.ietf.org/html/rfc7540)
- [HPACK RFC 7541](https://tools.ietf.org/html/rfc7541)
- [Hyperium h2 crate](https://github.com/hyperium/h2)
