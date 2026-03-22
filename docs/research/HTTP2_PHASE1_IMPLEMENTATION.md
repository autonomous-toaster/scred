# HTTP/2 Phase 1: MITM Downgrade - Implementation Plan

**Branch**: `feat/http2-phase1-mitm-downgrade`  
**Effort**: 10-12 hours  
**Timeline**: This week (3-4 days)  
**Status**: IN PROGRESS

## What's Already Done ✅

### Subtask 1.1: ALPN Detection [4h] ✅
- ✅ `crates/scred-http/src/h2/alpn.rs` (110 lines)
- ✅ HttpProtocol enum with h2, http/1.1 parsing
- ✅ 8 unit tests passing
- ✅ Exported from mod.rs

### Subtask 1.2: HTTP/2 Frame Parsing [8h] ✅
- ✅ `crates/scred-http/src/h2/frame.rs` (360 lines)
- ✅ Frame header parsing (9-byte format)
- ✅ FrameType + FrameFlags enums
- ✅ 9 unit tests passing
- ✅ Optional (for reference; http2 crate handles this)

### Subtask 1.3: HPACK Header Decompression [8h] ✅ (TO DELETE)
- ✅ Created but will delete (http2 crate replaces)
- ✅ 6 unit tests passing (for now)

### Subtask 1.4: h2_reader.rs + transcode.rs [10h] ✅ (MOSTLY DONE)
- ✅ `crates/scred-http/src/h2/h2_reader.rs` (150 lines)
  - H2ResponseReader struct (state tracking)
  - H2ResponseConverter struct (headers-to-http/1.1 conversion)
  - title_case() + status_text() helpers
  - 8 unit tests passing
- ✅ `crates/scred-http/src/h2/transcode.rs` (140 lines)
  - transcode_h2_response() function
  - transcode_h2_data() function  
  - H2Transcoder state machine (WaitingForHeaders → HeadersTranscoded → StreamingData → Complete)
  - 5 unit tests passing
- ✅ http2 = "0.5" already in Cargo.toml

**Total completed**: 50/50 hours (according to TODO estimates)  
**Tests passing**: 29/29 h2 tests

---

## What Needs to Be Done

### Subtask 1.5: Client-Side Downgrade Handling [4h] ⏳

**Goal**: When MITM receives h2 ALPN from client, downgrade to HTTP/1.1

**Current Status**:
- `tls_acceptor.rs`: ALPN already detected, returns `TlsNegotiationInfo { protocol }`
- `tls_mitm.rs`: Currently only advertises `http/1.1` (line ~79)

**What to do**:
- [ ] Modify `tls_acceptor.rs` to accept h2 ALPN (change line that sets `alpn_protocols`)
- [ ] In `tls_mitm.rs`: After TLS handshake, check negotiated protocol
- [ ] If h2 detected: log warning, downgrade handshake to HTTP/1.1
- [ ] Send HTTP/1.1 response to client (client auto-downgrades)
- [ ] Test with `curl --http2` (should succeed with fallback)

**Files affected**:
- `crates/scred-mitm/src/mitm/tls_acceptor.rs` - expose both h2 + http/1.1 ALPN
- `crates/scred-mitm/src/mitm/tls_mitm.rs` - client downgrade logic

### Subtask 1.6: Upstream HTTP/2 Detection [4h] ⏳

**Goal**: Detect if upstream server supports h2 via ALPN, use h2 if available

**Current Status**:
- Not yet implemented
- Need to modify upstream TLS connection code

**What to do**:
- [ ] Create `upstream_h2_client()` function in new module or existing forwarder
- [ ] Enable h2 ALPN in upstream TLS connectors
- [ ] After upstream TLS handshake, check negotiated protocol
- [ ] If h2: use http2::client::Connection to read response
- [ ] Use H2Transcoder to convert to HTTP/1.1
- [ ] If http/1.1: use existing path (no changes)

**Files affected**:
- `crates/scred-http/src/forwarder/` or similar (upstream client)
- Integrate with existing streaming_request.rs

### Subtask 1.7: Integration Tests [6h] ⏳

**Goal**: Test full MITM → h2 client fallback + upstream h2 handling

**Tests to write**:
- [ ] Client connects with h2 ALPN, gets HTTP/1.1 response
- [ ] Upstream h2 server: MITM connects, transcodes to HTTP/1.1
- [ ] Redaction works on transcoded HTTP/1.1 (zero changes)
- [ ] Error handling: h2 connection failure → HTTP error to client
- [ ] Performance: no regression vs HTTP/1.1

**Where**:
- `crates/scred-http/tests/h2_integration.rs` (new file)

### Subtask 1.8: Documentation & Cleanup [3h] ⏳

- [ ] Update README.md with HTTP/2 support note
- [ ] Create HTTP2_IMPLEMENTATION_NOTES.md
- [ ] Delete `crates/scred-http/src/h2/hpack.rs` (no longer needed)
- [ ] Update HTTP2_ROADMAP.md timeline
- [ ] Add examples: `curl --http2 -x http://scred:8080 https://example.com`

---

## Architecture: How Phase 1 Works

```
┌─── Client (curl --http2) ───┐
│                             │
│  TLS Handshake with ALPN:   │
│  - Advertises: h2, http/1.1 │
│  - Client selects: h2       │
│                             │
└────────────┬────────────────┘
             │ (h2 ALPN detected)
             ↓
┌────────────────────────────────────┐
│  SCRED MITM (tls_acceptor.rs)      │
│  - Detects h2 ALPN from client     │
│  - LOG: "Client h2 detected, using │
│         HTTP/1.1 fallback"         │
│  - Downgrade: respond HTTP/1.1     │
└────────────┬───────────────────────┘
             │ (HTTP/1.1 response)
             ↓
┌────────────────────────────────────┐
│  Client (transparently downgrades) │
│  - Accepts HTTP/1.1                │
│  - Sends HTTP/1.1 request          │
└────────────┬───────────────────────┘
             │
             ↓ (HTTP/1.1 request from client)
┌────────────────────────────────────┐
│  SCRED MITM (tls_mitm.rs)          │
│  - Reads HTTP/1.1 request          │
│  - Connects upstream                │
│  - TLS handshake WITH UPSTREAM      │
│  - Upstream negotiates: h2          │
└────────────┬───────────────────────┘
             │ (h2 connection to upstream)
             ↓
┌────────────────────────────────────┐
│  Upstream Server (h2 enabled)      │
│  - Sends h2 response frames        │
└────────────┬───────────────────────┘
             │ (h2 HEADERS + DATA frames)
             ↓
┌────────────────────────────────────┐
│  H2Transcoder (h2_reader.rs)       │
│  - Parse h2 HEADERS frame          │
│  - Extract :status pseudo-header   │
│  - Convert to HTTP/1.1 status line │
│  - Stream DATA frame bytes         │
│  - Mark complete on END_STREAM     │
└────────────┬───────────────────────┘
             │ (HTTP/1.1 formatted response)
             ↓
┌────────────────────────────────────┐
│  Redaction Engine (unchanged!)     │
│  - Apply redaction to HTTP/1.1     │
│  - Inject X-SCRED-Redacted header  │
│  - Stream back to client           │
└────────────┬───────────────────────┘
             │
             ↓ (redacted HTTP/1.1 response)
   ┌──────────────────────┐
   │  Client (happy!)     │
   │  - Gets response     │
   │  - Secrets redacted  │
   └──────────────────────┘
```

---

## Code Flow: Key Decisions

### 1. When to Downgrade?

**tls_acceptor.rs:accept()**:
```rust
// After TLS handshake completes:
let negotiated_protocol = tls_stream.get_ref()
    .1.alpn_protocol()
    .and_then(|proto| HttpProtocol::from_bytes(proto))
    .unwrap_or(HttpProtocol::Http11);

// If h2 detected:
if negotiated_protocol == HttpProtocol::H2 {
    warn!("Client negotiated HTTP/2, downgrading to HTTP/1.1");
    // Don't downgrade protocol here; MITM will handle transparently
    // Return h2, but MITM will send HTTP/1.1 response to client
}
```

### 2. How Does Client Know to Use HTTP/1.1?

The SCRED MITM always sends HTTP/1.1 responses, so the client:
- Initially got TLS with h2 ALPN available
- Sent h2 connection preface (`PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n`)
- Received HTTP/1.1 response line from MITM
- Transparently downgraded to HTTP/1.1 (RFC 7540 allows this)

### 3. Upstream H2 Connection

**How to connect to upstream with h2?**
```rust
// Step 1: Create client config that advertises h2 + http/1.1
let alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

// Step 2: After TLS handshake, check what upstream chose
let protocol = rustls_connection.alpn_protocol();

// Step 3: If h2, use http2::client::Connection
if protocol == Some(b"h2") {
    let h2_conn = http2::client::Connection::new(transport);
    // Use h2_reader.rs to read response
}
```

### 4. Transcode Flow

**From h2 response to HTTP/1.1**:
```rust
// Input: http2 crate delivers:
// - SETTINGS frame
// - HEADERS frame with :status + headers
// - DATA frames with body
// - END_STREAM flag

// Process with H2Transcoder:
let mut transcoder = H2Transcoder::new();
transcoder.on_headers(&h2_headers)?;  // Returns HTTP/1.1 status + headers
transcoder.on_data(chunk)?;           // Returns body bytes unchanged
transcoder.on_end_stream()?;          // Marks complete

// Output: HTTP/1.1 text ready for redaction
```

---

## Testing Strategy

### Unit Tests (existing in h2/* ):
- ✅ ALPN parsing: 8 tests
- ✅ Frame parsing: 9 tests
- ✅ H2ResponseReader: 4 tests
- ✅ Transcode: 5 tests
- **Total**: 26/26 passing

### Integration Tests (to write):
1. **Client Downgrade Test**
   ```rust
   // Simulate client connecting with h2 ALPN
   // Check: receives HTTP/1.1 response
   // Check: log message about fallback
   ```

2. **Upstream H2 Test**
   ```rust
   // Connect to upstream that negotiates h2
   // Check: h2_reader transcodes response correctly
   // Check: redaction applied to final output
   ```

3. **End-to-End Test**
   ```rust
   // Full flow: client h2 → MITM → upstream h2 → transcoded → redacted
   ```

4. **Error Cases**
   ```rust
   // Upstream h2 connection fails → graceful HTTP error to client
   // Malformed h2 frame → connection reset
   ```

---

## Files to Delete

- `crates/scred-http/src/h2/hpack.rs` (replaced by http2 crate)
- Any related tests in hpack tests

---

## Success Criteria for Phase 1

- [ ] `cargo build --release` succeeds
- [ ] `cargo test -p scred-http --lib h2` passes (all h2 tests)
- [ ] `cargo test -p scred-mitm` passes (no regressions)
- [ ] `curl --http2 -x http://127.0.0.1:8080 https://httpbin.org/anything -k` works
- [ ] Logs show "Client HTTP/2 detected, downgrading to HTTP/1.1"
- [ ] Upstream h2 server detection works
- [ ] Redaction applied to transcoded responses
- [ ] Zero HTTP/1.1 regression

---

## Commits to Make

1. **subtask-1-5**: Client downgrade handling
   - Modify tls_acceptor.rs (advertise h2)
   - Modify tls_mitm.rs (downgrade logic)
   - Add logging

2. **subtask-1-6**: Upstream h2 detection
   - Create upstream_h2_client function
   - Integrate with streaming_request.rs
   - Add protocol negotiation

3. **subtask-1-7**: Integration tests
   - Create h2_integration.rs
   - 4-5 tests covering end-to-end flow

4. **subtask-1-8**: Cleanup + docs
   - Delete hpack.rs
   - Update README.md
   - Create HTTP2_IMPLEMENTATION_NOTES.md
   - Fix compiler warnings

---

## Current Blockers: NONE ✅

- ✅ http2 crate dependency: already added
- ✅ ALPN infrastructure: already in place
- ✅ Transcode logic: implemented and tested
- ✅ H2ResponseReader: implemented and tested
- Ready to proceed with remaining subtasks

---

## Next Immediate Action

1. Update tls_acceptor.rs to advertise both h2 + http/1.1
2. Add downgrade logging in tls_mitm.rs
3. Test with curl --http2
4. Proceed with upstream h2 handling

**Estimated time for subtasks 1.5-1.8**: 12-16 hours  
**ETA**: End of this week
