# HTTP/2 Full Support Strategy for SCRED (No Downgrade)

**Status**: Architecture Design  
**Requirement**: Full HTTP/2 support with secret redaction (no downgrade)  
**Target**: Both MITM and proxy

## Requirement Clarification

**MUST NOT**: Downgrade HTTP/2 to HTTP/1.1
**MUST**: Support HTTP/2 natively for clients and upstream servers
**MUST**: Redact secrets in HTTP/2 streams properly

## Architecture: Full HTTP/2 Native Support

### Core Principle: Per-Stream Redaction State

Instead of transcoding to HTTP/1.1, we maintain HTTP/2 natively:

```
┌──────────────────────────────┐
│ HTTP/2 Client                │
│ Multiple concurrent streams  │
└──────────┬───────────────────┘
           │ h2 multiplexed
           ↓
┌──────────────────────────────────────────────────┐
│ SCRED H2 Router (NEW)                            │
│ ┌────────────────────────────────────────────┐   │
│ │ Per-Stream State Map:                      │   │
│ │  Stream 1: Headers + Body Buffer + Redactor│   │
│ │  Stream 3: Headers + Body Buffer + Redactor│   │
│ │  Stream 5: Headers + Body Buffer + Redactor│   │
│ └────────────────────────────────────────────┘   │
│ Demultiplex → Process each stream → Remultiplex │
└──────────┬───────────────────────────────────────┘
           │ h2 multiplexed (redacted)
           ↓
┌──────────────────────────────┐
│ Upstream (h2 or h1)          │
└──────────────────────────────┘
```

### Key Design Decisions

1. **Stream-ID Based Demultiplexing**
   - http2 crate handles frame parsing and multiplexing
   - We maintain HashMap<StreamId, StreamRedactionState>
   - Each stream gets independent redaction context

2. **Redaction Per Stream**
   ```rust
   struct StreamRedactionState {
       headers: HttpHeaders,
       body_chunks: Vec<Vec<u8>>,  // buffered
       redactor: StreamingRedactor,
       end_stream: bool,
   }
   ```
   - Redaction applied to headers and body per stream
   - Patterns matched across stream boundaries (stateful)
   - Result: h2-encoded redacted response

3. **No Transcoding Required**
   - Redaction works on HTTP/1.1 headers (same format)
   - Body bytes redacted as chunks (same logic)
   - Response stays as HTTP/2 frames
   - Zero complexity added vs transcode approach

4. **HTTP/2 → HTTP/2 Flow**
   - Client h2 connection accepted directly (no downgrade)
   - Upstream h2 connection negotiated (ALPN)
   - Streams demultiplexed → individually redacted → remultiplexed
   - Result: Full h2 support with redaction

## Implementation Plan: Full H2 Support

### Phase 1: Client-Side HTTP/2 Handler (Week 1, 15-20h)

**Goal**: Accept h2 clients without downgrading

Files to create:
- `crates/scred-http/src/h2/server.rs` (NEW)
  - H2ServerConnection wrapper
  - accept_stream() - handle incoming h2 streams
  - send_response() - send h2 response back to client
  
- `crates/scred-http/src/h2/stream_state.rs` (NEW)
  - StreamRedactionState struct
  - Per-stream redaction context
  - Body buffer + redactor

- `crates/scred-mitm/src/mitm/h2_mitm.rs` (NEW)
  - Main h2 MITM handler (replaces tls_mitm.rs downgrade logic)
  - Demultiplex streams
  - Route to redaction engine
  - Remultiplex responses

Files to modify:
- `crates/scred-mitm/src/mitm/tls_acceptor.rs`
  - Detect h2 ALPN
  - Route to h2_mitm handler (no downgrade)
  
- `crates/scred-mitm/src/mitm/http_handler.rs`
  - Conditional: if h2, use h2_mitm; if h1, use existing path

### Phase 2: Upstream HTTP/2 Support (Week 2, 15-20h)

**Goal**: Connect to upstream h2 servers with multiplexing

Files to create:
- `crates/scred-http/src/h2/client.rs` (NEW)
  - H2ClientConnection wrapper
  - send_request() - send h2 request upstream
  - receive_response() - receive h2 response
  
- `crates/scred-http/src/h2/stream_mapper.rs` (NEW)
  - Map client stream IDs → upstream stream IDs
  - Handle stream ID rewriting (if needed)

Files to modify:
- `crates/scred-mitm/src/mitm/h2_mitm.rs`
  - Detect upstream h2 ALPN
  - Create upstream h2 connection
  - Forward client streams to upstream
  - Receive upstream responses
  - Remultiplex back to client (redacted)

### Phase 3: Proxy Upstream Multiplexing (Week 3, 10-15h)

**Goal**: Proxy multiple clients to single upstream h2 connection

- Connection pooling for upstream h2
- Stream reuse optimization
- Concurrent request handling

## Architecture: scred-http Module Organization

```
scred-http/src/h2/
├── alpn.rs                      (existing, 110 LOC)
├── frame.rs                     (existing, 360 LOC)
├── h2_reader.rs                 (existing, 150 LOC)
├── transcode.rs                 (existing, 140 LOC)
├── server.rs                    (NEW, 200 LOC)
│   ├── H2ServerConnection
│   ├── accept_stream()
│   └── send_response()
├── stream_state.rs              (NEW, 150 LOC)
│   ├── StreamRedactionState
│   ├── add_header()
│   ├── add_body_chunk()
│   └── apply_redaction()
├── client.rs                    (NEW, 200 LOC)
│   ├── H2ClientConnection
│   ├── send_request()
│   └── receive_response()
├── stream_mapper.rs             (NEW, 100 LOC)
│   ├── StreamIdMap
│   └── map_stream_id()
└── mod.rs                       (updated exports)

scred-mitm/src/mitm/
├── h2_mitm.rs                   (NEW, 400 LOC)
│   ├── handle_h2_mitm()
│   ├── demultiplex_stream()
│   └── remultiplex_response()
├── tls_acceptor.rs              (updated)
│   └── Route to h2_mitm if h2 ALPN
└── tls_mitm.rs                  (existing, keep for h1)
```

## Redaction Flow: Full H2

### Example: API Key in H2 Response

```
1. Client: curl --http2 https://example.com/api?key=secret123
2. SCRED receives: HTTP/2 stream 1 (h2 HEADERS + h2 DATA)
3. Demultiplex: Extract stream 1 headers and body
4. Redaction context: Create StreamRedactionState for stream 1
5. Apply redaction:
   - Headers checked for patterns
   - Body chunks checked and redacted
   - Pattern found: "secret123" → "[REDACTED]"
6. Remultiplex: Encode redacted headers + body as h2 frames
7. Send h2 response to client
8. Client sees: redacted response in h2 frames
```

**Result**: Secrets redacted while maintaining h2 multiplexing!

## Per-Stream Redaction Implementation

```rust
pub struct StreamRedactionState {
    stream_id: u32,
    headers: HttpHeaders,
    body_chunks: Vec<Vec<u8>>,
    redactor: Arc<StreamingRedactor>,
    end_stream: bool,
}

impl StreamRedactionState {
    pub async fn apply_redaction(&mut self) -> Result<(HttpHeaders, Vec<u8>)> {
        // 1. Redact headers
        let redacted_headers = self.redactor.redact_headers(&self.headers);
        
        // 2. Combine and redact body chunks
        let body = self.body_chunks.join(&b""[..]);
        let (redacted_body, _stats) = self.redactor.redact_body(&body);
        
        Ok((redacted_headers, redacted_body))
    }
}
```

## No Transcode Needed!

Key insight: **Redaction works on HTTP/1.1 header format** regardless of protocol:
- HTTP/1.1: `GET /path?key=value HTTP/1.1\r\n`
- HTTP/2: `:method: GET`, `:path: /path?key=value` (pseudo-headers)

Both are text-based secret patterns. HTTP/2 headers are just formatted differently internally by http2 crate.

```
HTTP/1.1 headers → Redaction Engine → Redacted
HTTP/2 headers → Convert to text → Redaction Engine → Redacted → Convert back to h2
```

The conversion is **trivial** (just formatting), not like transcode.

## Testing Strategy

### Unit Tests
- Stream state management (add header, add chunk, apply redaction)
- Per-stream redaction isolation (stream 1 secret ≠ stream 3 secret)
- Stream ID mapping

### Integration Tests
- Client h2 → SCRED h2 MITM → redacted
- Upstream h2 → SCRED proxies to client h2 → redacted
- Concurrent streams (1, 3, 5) all redacted independently
- Error handling (stream reset, connection close)

### End-to-End Tests
- `curl --http2 -x http://scred:8080 https://example.com/api?key=secret`
- Verify response has `[REDACTED]` in place of `secret`
- Verify h2 frames properly formatted

## Performance Impact

- **Demultiplexing**: ~0.5ms per stream
- **Per-stream redaction**: ~1-2ms per stream (same as http/1.1)
- **Remultiplexing**: ~0.5ms per stream
- **Total overhead**: ~2-4ms per stream (acceptable)
- **Benefit**: True multiplexing (no h1 connection per stream)

## Migration Path from Current Downgrade

If current code is downgrading:

1. Keep existing h2 module infrastructure
   - alpn.rs (detection) ✅
   - transcode.rs (reference, remove later) 
   - h2_reader.rs (convert to stream_state.rs)

2. Add new h2 native support
   - server.rs (h2 client handler)
   - client.rs (h2 upstream connector)
   - h2_mitm.rs (main demux/remux logic)

3. Update MITM entry points
   - tls_acceptor.rs: Route h2 to h2_mitm (not downgrade)
   - http_handler.rs: Conditional routing

4. Remove downgrade paths
   - Delete transcode.rs (no longer needed)
   - Remove downgrade logging
   - Update docs

## Success Criteria

- ✅ HTTP/2 clients connect without downgrade
- ✅ Secrets redacted in h2 streams
- ✅ Multiple concurrent streams work correctly
- ✅ Upstream h2 servers supported
- ✅ Zero HTTP/1.1 regression
- ✅ Performance acceptable (<5ms overhead per stream)

## Effort Estimate

| Phase | Component | Hours | Status |
|-------|-----------|-------|--------|
| 1 | Client h2 handler (server.rs, stream_state.rs, h2_mitm.rs) | 15-20 | TODO |
| 2 | Upstream h2 (client.rs, stream_mapper.rs) | 15-20 | TODO |
| 3 | Proxy multiplexing (connection pooling) | 10-15 | TODO |
| - | Tests + docs | 10-15 | TODO |
| **Total** | **Full h2 support (no downgrade)** | **50-70** | **TODO** |

**Timeline**: 2-3 weeks for full h2 support (no downgrade)

## References

- RFC 7540: HTTP/2 Specification (multiplexing)
- RFC 7541: HPACK (header compression)
- http2 crate: ServerConnection + ClientConnection
- SCRED redaction engine: StreamingRedactor interface

---

**Next**: Should I implement this full H2 native support? Or clarify requirements further?
