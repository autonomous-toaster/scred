# Production Quality TODO - SCRED HTTP/2 MITM

**Status**: 435/435 tests passing. Code is functionally complete but has many "For now" comments that need production implementations.

## Critical Path (Blocking)

### 1. HTTP/2 Upstream → HTTP/1.1 Client Transcoding
**File**: crates/scred-mitm/src/mitm/tls_mitm.rs:342
**Issue**: When upstream is HTTP/2 but client is HTTP/1.1, we need to transcode
**Impact**: HIGH - curl test fails with "Empty reply from server"
**Solution**: Route to proper H2 frame reader and convert to HTTP/1.1 response lines

### 2. H2ProxyBridge Implementation
**File**: crates/scred-http/src/h2/h2_proxy_bridge.rs
**Issue**: Bridge created but event loop not implemented
**Impact**: MEDIUM - Blocks HTTP/2 through HTTP/1.1 proxy scenario
**Solution**: Complete the bridge event loop for multiplexing H2 → H1.1 proxy

## Important (Production-Grade Code Quality)

### 3. Header Redactor - Full SCRED Integration
**File**: crates/scred-http/src/h2/header_redactor.rs
**Current**: Uses placeholder redaction, not full SCRED pattern matching
**Solution**: Integrate full RedactionEngine for all 47 patterns

### 4. Per-Stream Redactor - Apply Redaction to Body
**File**: crates/scred-http/src/h2/per_stream_redactor.rs:175
**Current**: Returns unchanged value, TODO says "Apply redaction to headers"
**Solution**: Actually apply StreamingRedactor to header values

### 5. Host Identification - SNI Extraction
**File**: crates/scred-http/src/host_identification.rs
**Current**: Uses simplified TLS parsing, says "In production, use a TLS parsing library"
**Solution**: Either use proper TLS parser or mark as acceptable limitation

### 6. HPACK Huffman Decode
**File**: crates/scred-http/src/h2/hpack.rs
**Current**: Returns placeholder for Huffman decoding
**Solution**: Implement proper RFC 7541 Appendix B Huffman decoding

## Nice-to-Have (Code Cleanliness)

### 7. Stream Manager Redaction
**File**: crates/scred-http/src/h2/stream_manager.rs:130-131
**Issue**: Clone without redaction with TODO comments
**Solution**: Remove or implement actual redaction

### 8. Upstream H2 Client
**File**: crates/scred-http/src/upstream_h2_client.rs
**Issue**: "simplified version", not using http2 crate integration
**Solution**: Complete integration or mark as known limitation

### 9. PING Frame ACK
**File**: crates/scred-mitm/src/mitm/h2_mitm.rs
**Issue**: "not yet implemented"
**Solution**: Send PING ACK frames per RFC 7540

### 10. SETTINGS Frame Parsing
**File**: crates/scred-mitm/src/mitm/h2_mitm.rs
**Issue**: "settings not yet parsed"
**Solution**: Parse SETTINGS frame and update connection config

## Action Items

Priority 1 (Must implement):
- [ ] H2 upstream transcode to H1.1 response (fixes curl test)
- [ ] H2ProxyBridge event loop (enables proxy scenario)

Priority 2 (Should implement):
- [ ] Full SCRED header redaction integration
- [ ] Per-stream body redaction
- [ ] Proper SNI extraction (or document limitation)

Priority 3 (Code cleanliness):
- [ ] HPACK Huffman decoding
- [ ] PING/SETTINGS frame handling
- [ ] Remove all clone+TODO patterns

## Quality Gates

**Before marking "production-ready"**:
- ✅ 435/435 tests passing
- ✅ Zero unsafe blocks
- ✅ Zero production unwraps
- ✅ Zero compiler warnings (strict)
- ❌ Zero "For now" comments in production code
- ❌ Zero "In production" comments
- ❌ All curl tests passing (HTTP/2 upstream transcoding)

**Current Status**: 3/6 quality gates passed
