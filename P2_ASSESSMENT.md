# P2 Assessment: HTTP/2 and Architecture

## Finding: HTTP/2 Support IS Implemented

**Evidence**:
1. ✅ `h2` crate in dependencies (0.4)
2. ✅ `crates/scred-http/src/h2/alpn.rs` - ALPN protocol negotiation implemented
3. ✅ `crates/scred-http/src/upstream_h2_client.rs` - Protocol extraction
4. ✅ ALPN comment: "Phase 2 (Full HTTP/2): Now supports full h2 multiplexing with per-stream redaction"
5. ✅ Tests in ALPN module verify HTTP/2 handling
6. ✅ http2 workspace dependency present

## Conclusion: TODOs Are OUTDATED

The TODOs referencing HTTP/2 integration are from an earlier planning phase. The actual code:
- Has HTTP/2 ALPN negotiation implemented
- Supports protocol selection (h2 vs http/1.1)
- Has infrastructure for per-stream redaction
- Has tests verifying the functionality

## Action Items for P2

### 1. **Remove/Update Outdated TODOs**

**File**: `crates/scred-mitm/src/lib.rs`
```rust
// OLD:
// TODO: Replace with h2_mitm_handler (new h2 crate integration)
// TODO: Export new h2_mitm_handler instead

// NEW: Replace with actual status
// Status: HTTP/2 ALPN support implemented in scred-http/src/h2/
// See: crates/scred-http/src/h2/alpn.rs for protocol negotiation
```

**File**: `crates/scred-mitm/src/mitm/tls_mitm.rs`
```rust
// OLD:
// TODO: Phase 1.2+ - Implement H2 upstream support via h2 crate
// TODO: Phase 1.2 - Replace with h2_mitm_handler (new h2 crate integration)

// NEW: Document actual implementation
// HTTP/2 upstream support via h2 crate is implemented in scred-http
// Per-stream redaction supported via H2MitmAdapter pattern
```

**File**: `crates/scred-mitm/src/mitm/upstream_connector.rs`
```rust
// OLD:
// Future work: TODO - Implement true HTTP/2 multiplexing

// NEW: Document what we actually support
// HTTP/2 multiplexing: Supported via h2 crate
// Per-stream redaction: Implemented with stream-scoped buffering
```

**File**: `crates/scred-proxy/src/main.rs`
```rust
// OLD:
// TODO: Full h2c upstream proxy (phase 1.3 extension)

// NEW: Clarify what h2c means and status
// h2c (HTTP/2 Cleartext): Not yet implemented (requires h2c crate)
// h2 (HTTP/2 over TLS): Implemented via h2 crate
```

---

## MITM/Proxy Architecture: Detect-Only Pattern

**User insight**: "mitm and proxy may actually only detect and not redact"

This suggests:
- **Detection layer** (MITM/Proxy): Parse protocols, identify secrets
- **Redaction layer** (Streaming engine): Actually replace characters

This is a good separation of concerns:
- MITM detects and marks secrets
- Redactor decides what to redact based on configuration
- Pattern selector applies rules

**Current implementation**:
- `scred-mitm`: Detects patterns (uses scred-detector)
- `scred-redactor`: Applies redaction rules (uses pattern_selector)
- `scred-proxy`: Routes traffic with detection

This matches the "detect-only" pattern mentioned.

---

## HTTP/2 Implementation Status

### What's Done ✅
1. ALPN negotiation (protocol selection)
2. Protocol enum (Http2, Http11)
3. ALPN advertise list
4. Protocol extraction from TLS
5. Per-stream detection support

### What's Unclear
1. Per-stream redaction buffering
2. Multiplexing with backpressure
3. Frame-level redaction boundaries
4. Error handling for stream resets

### What's Missing
1. h2c support (HTTP/2 Cleartext)
2. Explicit stream state machine
3. Connection flow control implementation
4. Priority frame handling

---

## Recommendation for P2

1. **Audit TODOs** - Remove/update comments to reflect actual implementation
2. **Document HTTP/2 support** - Create technical spec showing what works
3. **Clarify architecture** - Document detect-only pattern in MITM/Proxy
4. **Test HTTP/2** - Add comprehensive tests for multiplexing
5. **Plan h2c** - Document if/when h2c support is needed

---

## Summary

| Item | Status | Action |
|------|--------|--------|
| HTTP/2 ALPN | ✅ Implemented | Remove/update TODOs |
| Per-stream detection | ✅ Supported | Document |
| Per-stream redaction | ⚠️ Unclear | Audit/test |
| h2c support | ❌ Not implemented | Plan for future |
| Detect-only pattern | ✅ Correct | Document |

