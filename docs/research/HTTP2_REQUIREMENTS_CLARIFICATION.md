# HTTP/2 Requirements Clarification & Architecture Pivot

**Date**: 2026-03-19 (Session 4, Critical Change)
**Status**: Requirements finalized, new architecture documented

## Requirement Statement

### MUST NOT
- ❌ Downgrade HTTP/2 clients to HTTP/1.1
- ❌ Transcode HTTP/2 responses to HTTP/1.1
- ❌ Lose h2 multiplexing benefits

### MUST
- ✅ Support HTTP/2 natively for clients
- ✅ Support HTTP/2 natively for upstream servers
- ✅ Redact secrets in HTTP/2 streams properly
- ✅ Work for both MITM and proxy modes

## Previous Approach (DEPRECATED)

The previously implemented approach was transparent fallback:
```
Client h2 ALPN → SCRED downgrades to HTTP/1.1 → redaction → HTTP/1.1 response
Upstream h2 → SCRED transcodes to HTTP/1.1 → redaction → HTTP/1.1 response
```

**Status**: This approach does NOT meet requirements (no downgrade allowed)
**Action**: Must pivot to full HTTP/2 native support

## New Architecture: Full HTTP/2 Native Support

### Core Principle: Per-Stream Redaction State

Instead of downgrade/transcode, maintain HTTP/2 natively:

```
HTTP/2 Client (multiplexed)
    ↓
SCRED H2 Router
├─ Stream 1 → StreamRedactionState → Redactor 1 → Redacted response
├─ Stream 3 → StreamRedactionState → Redactor 2 → Redacted response  
└─ Stream 5 → StreamRedactionState → Redactor 3 → Redacted response
    ↓
HTTP/2 Upstream (multiplexed)
```

### Key Insights

1. **No Transcode Needed**
   - Redaction works on HTTP/1.1 header format
   - HTTP/2 headers are just pseudo-headers (same format)
   - Body chunks redacted identically
   - Response stays as h2 frames (no conversion)

2. **Per-Stream Isolation**
   - Each stream gets independent redactor context
   - Stream 1's secrets don't leak to Stream 3
   - Patterns matched per stream
   - Concurrent streams handled safely

3. **Simpler Than Previous Approach**
   - No complex transcode logic
   - Just demultiplex → redact → remultiplex
   - Uses same redaction engine (StreamingRedactor)
   - Same complexity as HTTP/1.1 (~2-4ms per stream)

## Implementation Architecture

### Modules to Create

**scred-http/src/h2/**
```
server.rs (200 LOC)
├─ H2ServerConnection wrapper
├─ accept_stream() - handle incoming h2 streams from client
└─ send_response() - send h2 response back to client

stream_state.rs (150 LOC)
├─ StreamRedactionState struct
├─ add_header() - accumulate request headers
├─ add_body_chunk() - buffer body bytes
└─ apply_redaction() - apply redactor to headers + body

client.rs (200 LOC)
├─ H2ClientConnection wrapper
├─ send_request() - send h2 request to upstream
└─ receive_response() - receive h2 response from upstream

stream_mapper.rs (100 LOC)
├─ StreamIdMap for mapping client stream IDs → upstream
└─ handle_stream_id_rewriting()
```

**scred-mitm/src/mitm/**
```
h2_mitm.rs (400 LOC)
├─ handle_h2_mitm() - main entry point (no downgrade!)
├─ accept_client_h2_streams() - accept from client
├─ connect_upstream_h2() - connect to upstream (ALPN)
├─ forward_stream_to_upstream() - forward client stream
├─ receive_upstream_response() - receive redacted response
└─ send_to_client_h2() - send back to client
```

### Integration Points

**tls_acceptor.rs**
```rust
// Detect h2 ALPN
let protocol = tls_connection.alpn_protocol();

// Route BASED ON PROTOCOL (no downgrade!)
if protocol == Some(b"h2") {
    // NEW: Route to h2_mitm (native h2 handler)
    handle_h2_mitm(client, host, upstream).await?
} else {
    // EXISTING: Route to tls_mitm (http/1.1)
    handle_tls_mitm(client, host, upstream).await?
}
```

## Redaction Flow: Full HTTP/2 Example

```
1. Client: curl --http2 https://api.example.com/user?api_key=secret123

2. SCRED receives:
   - h2 Stream 1 HEADERS frame:
     :method: GET
     :path: /user?api_key=secret123
     :authority: api.example.com
   - h2 Stream 1 DATA frame:
     (body bytes, if any)

3. Demultiplex:
   - Extract stream_id = 1
   - Parse headers
   - Buffer body chunks

4. Create StreamRedactionState:
   - stream_id: 1
   - headers: HttpHeaders { ... }
   - redactor: StreamingRedactor::new()

5. Apply Redaction:
   - Check headers for patterns
   - Pattern found: "secret123" (matches configured pattern)
   - Redact: "secret123" → "[REDACTED]"
   - Result: {
       :path: /user?api_key=[REDACTED]
     }

6. Upstream Connection:
   - TLS handshake (ALPN negotiation)
   - Upstream selected: h2
   - Create H2ClientConnection
   - Send redacted request to upstream

7. Receive Upstream Response:
   - h2 HEADERS frame with :status 200
   - h2 DATA frames with response body
   - Body contains API response with secrets
   - Apply redaction to response body

8. Remultiplex:
   - Encode redacted headers + body as h2 frames
   - Send back to client via h2 connection
   - Client receives: redacted response (still h2!)

9. Client sees:
   - HTTP/2 response
   - API key redacted: "secret123" → "[REDACTED]"
   - All multiplexing benefits preserved
```

## Migration from Downgrade Approach

1. **Phase 1: Keep Infrastructure**
   - alpn.rs (already done, reuse for h2 detection)
   - frame.rs (useful as reference, optional)
   - Keep h2_reader.rs for now (convert to stream_state.rs)

2. **Phase 2: Create New Modules**
   - server.rs, stream_state.rs (client-side h2 handling)
   - client.rs, stream_mapper.rs (upstream h2 handling)
   - h2_mitm.rs (main routing logic)

3. **Phase 3: Update MITM**
   - tls_acceptor.rs: Route h2 → h2_mitm (NOT downgrade!)
   - Keep tls_mitm.rs for HTTP/1.1 (no regressions)

4. **Phase 4: Remove Downgrade Code**
   - Delete transcode.rs (no longer needed)
   - Remove downgrade logging (no longer happens)
   - Update documentation

5. **Phase 5: Extend to Proxy**
   - Add connection pooling for upstream h2
   - Implement stream multiplexing optimization
   - Support multiple clients → single upstream h2

## Code Example: Per-Stream Redaction

```rust
// StreamRedactionState: One per client stream
pub struct StreamRedactionState {
    stream_id: u32,
    headers: HttpHeaders,
    body_chunks: Vec<Vec<u8>>,
    redactor: Arc<StreamingRedactor>,
    end_stream: bool,
}

impl StreamRedactionState {
    pub async fn apply_redaction(&mut self) 
        -> Result<(HttpHeaders, Vec<u8>)> {
        
        // 1. Redact headers (pattern matching)
        let redacted_headers = self.redactor.redact_headers(&self.headers)?;
        
        // 2. Combine body chunks
        let mut body = Vec::new();
        for chunk in &self.body_chunks {
            body.extend_from_slice(chunk);
        }
        
        // 3. Redact body (streaming redaction)
        let (redacted_body, _stats) = self.redactor.redact_body(&body)?;
        
        Ok((redacted_headers, redacted_body))
    }
}

// Main MITM handler: Manages multiple streams
pub struct H2MitmRouter {
    streams: HashMap<u32, StreamRedactionState>,
    redaction_engine: Arc<RedactionEngine>,
}

impl H2MitmRouter {
    pub async fn handle_stream_data(
        &mut self,
        stream_id: u32,
        frame: h2::Frame,
    ) -> Result<()> {
        // Get or create stream state
        let stream = self.streams
            .entry(stream_id)
            .or_insert_with(|| StreamRedactionState::new(stream_id));
        
        // Process frame based on type
        match frame {
            h2::Frame::Headers(headers) => {
                stream.add_headers(headers);
            }
            h2::Frame::Data(data) => {
                stream.add_body_chunk(data.clone());
                if data.is_end_stream() {
                    stream.end_stream = true;
                }
            }
        }
        
        Ok(())
    }
}
```

## Performance Impact

| Operation | Overhead | Per-Stream |
|-----------|----------|------------|
| Demultiplex | ~0.5ms | Per stream |
| Create StreamRedactionState | ~0.1ms | Per stream |
| Apply redaction (headers) | ~0.5ms | Per stream |
| Apply redaction (body) | ~1-2ms | Per stream |
| Remultiplex | ~0.5ms | Per stream |
| **Total** | **~3-4ms** | **Per stream** |

**Benefit**: Multiple streams on single connection (true multiplexing)

## Success Criteria

- ✅ HTTP/2 clients accepted without downgrade
- ✅ Secrets redacted in h2 request headers
- ✅ Secrets redacted in h2 response bodies
- ✅ Multiple concurrent streams work correctly
- ✅ Each stream's redaction isolated (no cross-leakage)
- ✅ Upstream h2 servers supported
- ✅ Zero HTTP/1.1 regression
- ✅ Performance acceptable (<5ms overhead per stream)
- ✅ Works for both MITM and proxy modes

## Timeline & Effort

| Phase | Component | Hours | Week |
|-------|-----------|-------|------|
| 1 | Client h2 handler + h2_mitm | 15-20 | 1 |
| 2 | Upstream h2 + stream mapping | 15-20 | 2 |
| 3 | Proxy multiplexing + pooling | 10-15 | 3 |
| - | Tests + documentation | 10-15 | 2-3 |
| **Total** | **Full h2 native support** | **50-70** | **2-3 weeks** |

## Comparison: Downgrade vs Native

| Aspect | Downgrade (Deprecated) | Native (NEW) |
|--------|----------|---------|
| **H2 clients** | ❌ Downgrade to h1 | ✅ Full h2 support |
| **H2 upstreams** | ⚠️ Transcode to h1 | ✅ Full h2 support |
| **Multiplexing** | ❌ Single stream per connection | ✅ True h2 multiplexing |
| **Redaction** | ✅ Works (on h1 transcode) | ✅ Works (native h2) |
| **Complexity** | Low | Medium |
| **Performance** | Good (no transcode) | Good (no transcode) |
| **Meets Requirements** | ❌ NO (downgrade disallowed) | ✅ YES (native h2) |

## References

- RFC 7540: HTTP/2 Specification (streams, multiplexing)
- RFC 7541: HPACK (header compression)
- http2 crate: ServerConnection + ClientConnection
- SCRED redaction engine: StreamingRedactor interface

---

## DECISION

✅ **Full HTTP/2 Native Support** (no downgrade)
- Requirement: Must support h2 natively
- Approach: Per-stream redaction state
- Implementation: 50-70 hours, 2-3 weeks
- Risk: Medium (new code, but bounded scope)
- Benefit: Production-ready h2 support for both MITM and proxy

**Next**: Begin Phase 1 implementation (client-side h2 handler)
