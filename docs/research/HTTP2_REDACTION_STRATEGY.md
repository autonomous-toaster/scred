# HTTP/2 Redaction Strategy for SCRED

**Status**: Phase 1 Implementation  
**Branch**: `feat/http2-phase1-mitm-downgrade`

## Executive Summary

SCRED's HTTP/2 support in Phase 1 uses **transparent HTTP/1.1 fallback** for both client and upstream connections. This maintains compatibility with existing redaction logic while enabling HTTP/2 server support through transcode-to-HTTP/1.1.

**Key Design**: Redaction always operates on HTTP/1.1 text, regardless of whether the client or upstream uses HTTP/2.

---

## Why HTTP/2 is Tricky for Redaction

### Problem 1: Multiplexing Breaks Single-Request Context
**HTTP/1.1 Model**:
```
Client Request 1 → SCRED → Upstream
             ↓ (parse, check redaction, stream)
           Response 1 ← SCRED ← Upstream
Client Request 2 → SCRED → Upstream
```

**HTTP/2 Model**:
```
Client Stream 1 ┐
Client Stream 2 ├─→ SCRED (demultiplex, check redaction per stream, remultiplex)
Client Stream 3 ┘
```

SCRED's redaction engine is designed for **single request context**:
- Parse once
- Check patterns once
- Stream once
- Redact once

HTTP/2 multiplexing requires per-stream redaction state, which is Phase 2+ territory.

### Problem 2: HPACK Decompression is Complex
- HTTP/2 uses HPACK (RFC 7541) for header compression
- Non-trivial to decompress (~200 LOC) and pattern-match
- Solution: Use `http2` crate which handles this correctly

### Problem 3: MITM ALPN Chicken-Egg
- SCRED must present certificate before knowing which protocol client wants
- Solution: Advertise both h2 + http/1.1, let client choose
- If client chooses h2: respond with HTTP/1.1 (transparent downgrade)

---

## Phase 1 Architecture: Transparent Fallback

### Client Flow (h2 → HTTP/1.1)

```
┌──────────────┐
│ HTTP/2 Client│
│ Sends: h2    │
└──────┬───────┘
       │ TLS with ALPN h2
       ↓
┌──────────────────────────────┐
│ SCRED TLS Acceptor           │
│ Advertise: [h2, http/1.1]    │
│ Detect: client chose h2      │
│ Action: Log + downgrade      │
└──────┬───────────────────────┘
       │ Respond: HTTP/1.1 response line
       ↓
┌──────────────┐
│ HTTP/2 Client│
│ Auto-downgrade (RFC 7540)    │
│ Connection as: HTTP/1.1      │
└──────┬───────────────────────┘
       │ HTTP/1.1 request (per RFC 7540 §3.4)
       ↓
┌──────────────────────────────┐
│ SCRED Redaction Engine       │
│ Input: HTTP/1.1 request      │
│ Process: Existing logic      │
│ Output: Redacted HTTP/1.1    │
└──────────────────────────────┘
       │
       ↓ (to upstream)
```

**Why this works**:
- Client RFC 7540 compliant → accepts HTTP/1.1 downgrade
- Redaction: unchanged (works on HTTP/1.1)
- Implementation: ~50 lines (ALPN + downgrade logging)
- Risk: minimal (proven HTTP/1.1 path)

### Upstream Flow (h2 → HTTP/1.1)

```
┌──────────────────────────────┐
│ SCRED MITM                   │
│ Upstream TLS with ALPN       │
│ [h2, http/1.1]              │
└──────┬───────────────────────┘
       │ Upstream negotiates: h2
       ↓
┌──────────────────────────────┐
│ HTTP/2 Upstream Server       │
│ Sends: h2 HEADERS + DATA     │
└──────┬───────────────────────┘
       │ (h2 binary frames)
       ↓
┌──────────────────────────────┐
│ H2Transcoder                 │
│ Input: h2 frames (HEADERS)   │
│ Parse: :status pseudo-header │
│ Output: HTTP/1.1 status line │
└──────┬───────────────────────┘
       │ HTTP/1.1 response
       ↓
┌──────────────────────────────┐
│ SCRED Redaction Engine       │
│ Input: HTTP/1.1 response     │
│ Process: Existing logic      │
│ Output: Redacted HTTP/1.1    │
└──────┬───────────────────────┘
       │
       ↓ (to client)
```

**Why this works**:
- Upstream h2 is transparent (no spec requirement for downgrade)
- HPACK decompressed by `http2` crate (RFC 7541 compliant)
- Transcode: ~80 LOC (headers → HTTP/1.1 text, data → bytes)
- Redaction: unchanged (works on HTTP/1.1)

---

## Redaction: The Key Invariant

**Phase 1 Redaction Invariant**:
```rust
// Regardless of protocol client or upstream uses:
// Input redaction source = HTTP/1.1 text
// Output redaction target = HTTP/1.1 text
// Redaction logic = unchanged
```

### Example Flow End-to-End

```
1. Client: curl --http2 https://example.com/path?api_key=secret123
2. TLS Handshake: Client selects h2, SCRED downgrades
3. Client sends: HTTP/1.1 GET /path?api_key=secret123
4. SCRED checks: Pattern matches, schedules redaction
5. SCRED → Upstream TLS: h2 ALPN
6. Upstream responds: h2 HEADERS + h2 DATA (binary)
7. H2Transcoder: Converts to HTTP/1.1 text
   - HEADERS → "HTTP/1.1 200 OK\r\nContent-Type: ...\r\n"
   - DATA → body bytes (unchanged)
8. Redaction: Applied to HTTP/1.1 text (existing logic)
   - API key in response redacted: "secret123" → "[REDACTED]"
9. Response to client: HTTP/1.1 (already downgraded)
10. Client: Sees redacted response
```

**Key insight**: Redaction happens at step 8, always on HTTP/1.1 text.

---

## Code Organization

All HTTP/2 code is in `scred-http` for maximum reuse:

```
crates/scred-http/src/h2/
├── alpn.rs              # Protocol detection (h2 vs http/1.1)
├── frame.rs             # Basic frame parsing (reference, http2 crate used in production)
├── h2_reader.rs         # H2ResponseReader, H2ResponseConverter
├── transcode.rs         # H2Transcoder (h2 → HTTP/1.1)
└── mod.rs               # Module exports

crates/scred-http/src/
├── upstream_h2_client.rs
│   ├── UpstreamConnectionInfo         # Protocol info struct
│   ├── extract_upstream_protocol()    # ALPN → HttpProtocol
│   ├── select_upstream_handler()      # Route based on protocol
│   ├── H2UpstreamReader               # Phase 2 placeholder
│   └── handle_upstream_protocol_selection()  # Helper
└── lib.rs               # Module exports
```

### MITM Integration

```
crates/scred-mitm/src/mitm/
├── tls_mitm.rs
│   ├── handle_tls_mitm()              # Main MITM function
│   ├── handle_upstream_protocol_selection()  # Helper (uses upstream_h2_client)
│   └── [Phase 2 TODO] Route h2 to H2UpstreamReader
└── tls_acceptor.rs      # Client TLS (already h2-aware)
```

### Redaction Integration

Redaction engine (`scred-redactor` crate) sees only HTTP/1.1 text:

```
Input:
  - Client h2 → HTTP/1.1 request (after downgrade)
  - Upstream h2 → HTTP/1.1 response (after transcode)

Redaction (unchanged):
  - Pattern matching on HTTP/1.1 text
  - Secret replacement
  - Headers + body redaction

Output:
  - Redacted HTTP/1.1 response
```

---

## Phase 2 Preview: Full Native HTTP/2

When Phase 2 is implemented, the architecture will be:

```
┌────────────────────────────┐
│ HTTP/2 Client              │
└────────────┬───────────────┘
             │ h2 multiplexed streams
             ↓
┌────────────────────────────────────────────┐
│ SCRED MITM (Phase 2)                       │
│ 1. Accept h2 connection (no downgrade)     │
│ 2. Demultiplex per-stream                  │
│ 3. Create redaction state per stream       │
│ 4. Send to upstream (h2 if available)      │
│ 5. Receive upstream response (h2 or h1)    │
│ 6. Transcode if needed                     │
│ 7. Apply redaction per stream              │
│ 8. Remultiplex h2 response to client       │
└────────────┬───────────────────────────────┘
             │
             ↓ (h2 multiplexed response)
┌────────────────────────────┐
│ HTTP/2 Client (happy!)     │
│ Multiple streams, all works│
└────────────────────────────┘
```

**Phase 2 Effort**: 30-50 hours (demultiplexing, per-stream state, remultiplexing)

---

## Testing Strategy

### Unit Tests (✅ Done)
- ALPN parsing
- Frame parsing (reference implementation)
- H2ResponseReader + H2ResponseConverter
- H2Transcoder state machine
- upstream_h2_client routing

### Integration Tests (⏳ Phase 1.7)
1. **Client Downgrade Test**
   - Client connects with h2 ALPN
   - SCRED responds with HTTP/1.1
   - Client auto-downgrades
   - Request processed normally

2. **Upstream H2 Test**
   - SCRED connects upstream with h2 ALPN
   - Upstream responds with h2 frames
   - H2Transcoder converts to HTTP/1.1
   - Redaction applied
   - Response to client is HTTP/1.1

3. **End-to-End Test**
   - Client h2 → SCRED → Upstream h2 → Redacted HTTP/1.1
   - Verify: secrets redacted in both request and response

4. **Error Cases**
   - Upstream h2 connection fails
   - Malformed h2 frame
   - HPACK decompression error

---

## Security Considerations

### Secret Redaction in HTTP/2

**Question**: Are secrets visible in HTTP/2 if redaction is on HTTP/1.1?

**Answer**: No, because:
1. TLS encrypts everything (no plaintext HTTP/2 on wire)
2. SCRED operates between TLS decryption and redaction
3. Redaction applied before re-encryption
4. Flow: TLS decrypt → HTTP/1.1 text → Redaction → TLS encrypt

**Timeline**:
```
Client → TLS encrypted h2
         ↓ (SCRED decrypts)
         HTTP/1.1 text (redaction-ready)
         ↓ (Redaction applied)
         HTTP/1.1 redacted text
         ↓ (SCRED encrypts)
         TLS encrypted response
         ↓
         Client (sees redacted response)
```

**Threat model**: Redaction is not about hiding from TLS (encryption does that), but about hiding from log files, caches, and other SCRED-internal systems.

---

## Performance Implications

### Phase 1 (Fallback)
- **Overhead**: Minimal (ALPN detection + transcode)
- **ALPN**: ~0.1ms (certificate + ALPN exchange)
- **Transcode**: ~1-2ms (h2 frame → HTTP/1.1 text)
- **Redaction**: Unchanged

### Phase 2 (Full Native)
- **Overhead**: Moderate (demux/remux + per-stream state)
- **Demultiplexing**: ~5-10ms per stream
- **Per-stream redaction state**: ~100 bytes per stream
- **Remultiplexing**: ~5-10ms per stream
- **Total**: +10-30ms vs HTTP/1.1 (acceptable for Phase 2 if users need multiplexing)

---

## Roadmap

| Phase | Goal | Complexity | Timeline | Status |
|-------|------|-----------|----------|--------|
| 1 | Transparent fallback (h2 → HTTP/1.1) | Low | 1 week | IN PROGRESS |
| 2 | Full native h2 multiplexing | High | 2-3 weeks | NOT STARTED |
| 3 | H2 proxy upstream adaptation | High | 2-3 weeks | NOT STARTED |
| 4 | H2 push promise support | Medium | 1 week | NOT STARTED |

---

## Files Modified in Phase 1

### New Files
- `crates/scred-http/src/upstream_h2_client.rs` (275 LOC)
- `crates/scred-http/tests/h2_integration.rs` (TBD, Subtask 1.7)

### Modified Files
- `crates/scred-http/src/lib.rs` (1 line: export module)
- `crates/scred-mitm/src/mitm/tls_mitm.rs` (50 LOC: protocol detection + logging)
- `crates/scred-mitm/src/mitm/tls_acceptor.rs` (no changes, already h2-ready)

### Unchanged (Already Done)
- `crates/scred-http/src/h2/alpn.rs` (110 LOC, 8 tests)
- `crates/scred-http/src/h2/frame.rs` (360 LOC, 9 tests)
- `crates/scred-http/src/h2/h2_reader.rs` (150 LOC, 8 tests)
- `crates/scred-http/src/h2/transcode.rs` (140 LOC, 5 tests)

---

## References

- **RFC 7540**: HTTP/2 Specification
- **RFC 7541**: HPACK Header Compression
- **RFC 7540 Section 3.4**: HTTP/2 Connection Preface (includes HTTP/1.1 fallback)
- **http2 crate**: Production-grade HTTP/2 library (used for HPACK decompression)
- **SCRED Architecture**: `README.md` (core redaction logic)

---

## Next Steps

1. **Subtask 1.7**: Integration tests
2. **Subtask 1.8**: Documentation + cleanup
3. **Phase 2 Planning**: Full native h2 design document
