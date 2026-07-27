## Why

The codebase is in a **half-migrated state** from a custom HTTP/2 implementation to the external `h2` crate. The `scred-http::h2` module was cleaned up (86% reduction: 4,400 LOC → 650 LOC), removing custom frame parsing, HPACK, and stream handling. However, `scred-mitm` and `scred-proxy` still reference the deleted modules, causing **66 compilation errors** that block CI.

**Root Cause:**
- Custom HTTP/2 implementation was removed from `scred-http`
- Migration to `h2` crate was started but never completed
- `tls_mitm.rs`, `h2_upstream_forwarder/`, and `h2_mitm_handler.rs` still use old APIs

**Impact:**
- `cargo check --workspace` fails with 66 errors
- `just ci` cannot complete
- CRAP scores cannot be addressed (can't test code that doesn't compile)
- Development blocked on MITM and proxy features

## What Changes

### Phase 1: Remove Old Import Statements
- Remove imports for non-existent modules:
  - `scred_http::h2::h2_upstream_client`
  - `scred_http::h2::frame`
  - `scred_http::h2::frame_forwarder`
  - `scred_http::h2::hpack`
  - `crate::mitm::h2_mitm`

### Phase 2: Rewrite HTTP/2 Handling with `h2` Crate

#### In `scred-mitm`:
**Replace custom frame handling with `h2::server` and `h2::client`:**

```rust
// OLD (doesn't compile):
use scred_http::h2::frame::{Frame, FrameType};
let frame = Frame::parse(&header)?;
match frame.frame_type {
    FrameType::Headers => { ... }
    FrameType::Data => { ... }
}

// NEW (using h2 crate):
use h2::{server, client};
let mut h2_conn = server::handshake(io).await?;
while let Some(result) = h2_conn.accept().await {
    let (request, send_response) = result?;
    // Handle request
}
```

**Key Migrations:**
1. `handle_h2_client_transcoding()` - Rewrite using `h2::server` for client side, `h2::client` for upstream
2. `H2MitmHandler` - Replace with `h2::server` connection handler
3. HPACK decoding - Use `h2` crate's built-in HPACK (no manual decoding needed)
4. Frame forwarding - Let `h2` crate manage frames, intercept at request/response level

#### In `scred-proxy`:
**Replace custom HTTP/2 client with `h2::client`:**

```rust
// OLD (doesn't compile):
use scred_http::h2::h2_upstream_client::H2UpstreamClient;
let client = H2UpstreamClient::new();

// NEW:
use h2::client;
let (mut client, h2) = client::handshake(io).await?;
let response = client.send_request(request)?;
```

### Phase 3: Preserve Redaction Logic

**The original intent was correct:**
- Use `h2` crate for protocol handling (frames, streams, HPACK)
- Keep only redaction/policy logic in our code
- Intercept at request/response boundary, not frame level

**Redaction Points:**
```
┌─────────────────────────────────────────┐
│  h2::server (MITM)                      │
│  ↓ receives request                     │
│  [REDACTION HOOK] ← Inject here         │
│  ↓ forward to upstream                  │
│  h2::client                             │
│  ↓ receives response                    │
│  [REDACTION HOOK] ← Inject here         │
│  ↓ send to client                       │
└─────────────────────────────────────────┘
```

### Phase 4: Fix Compilation Errors in Other Crates
- `scred-proxy/src/handler.rs` - Fix H2 client usage
- `scred-cli/src/streaming.rs` - Fix trait bounds
- Fix generic argument mismatches (`Result<T>` → `Result<T, E>`)
- Add missing imports (`BufReader`, `Arc`, etc.)

## Capabilities

### New Capabilities
- `h2-crate-integration`: Use `h2::server` and `h2::client` for HTTP/2 protocol handling
- `request-level-redaction`: Intercept and redact at HTTP request/response level instead of frame level
- `simplified-architecture`: Delegate HTTP/2 complexity to `h2` crate, focus on redaction logic

### Modified Capabilities
- `h2-transcoding`: Now uses `h2` crate instead of custom frame handling
- `mitm-handler`: Simplified to request/response interception
- `upstream-forwarding`: Uses `h2::client` for upstream connections

## Impact

**Files Modified:**
- `crates/scred-mitm/src/mitm/tls_mitm.rs` - Rewrite H2 handling (~662 lines → ~300 lines)
- `crates/scred-mitm/src/mitm/h2_mitm_handler.rs` - Replace with `h2::server` wrapper
- `crates/scred-mitm/src/mitm/h2_upstream_forwarder/` - Replace with `h2::client` wrapper
- `crates/scred-proxy/src/handler.rs` - Fix H2 client usage
- `crates/scred-proxy/src/main.rs` - Fix imports
- `crates/scred-cli/src/streaming.rs` - Fix trait bounds and generics

**Files Deleted:**
- Backup files: `tls_mitm.rs.backup`, `tls_mitm.rs.bak`, etc.
- Unused: `h2_complete_handler.rs.bak`

**Dependencies:**
- `h2` crate already in `Cargo.toml` (no new dependencies)
- May need to enable features: `h2/stream`, `h2/unstable`

**Code Reduction:**
- `tls_mitm.rs`: 1,500 LOC → ~800 LOC (47% reduction)
- Overall: Less code to maintain, test, and debug

## Success Criteria

1. ✅ `cargo check --workspace --all-targets` passes with zero errors
2. ✅ `cargo build --workspace` succeeds
3. ✅ `h2` crate handles all HTTP/2 protocol details
4. ✅ Redaction logic preserved and functional
5. ✅ No regression in MITM or proxy functionality
6. ✅ Foundation laid for CRAP score improvements (smaller, testable functions)

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking changes in `h2` crate API | Pin `h2` version, read changelog |
| Losing custom behavior from old implementation | Document required behaviors, test thoroughly |
| Performance regression | Benchmark before/after, `h2` crate is well-optimized |
| Redaction logic breaks | Preserve redaction at request/response level, not frame level |
| Scope creep (full rewrite) | Focus on compilation fixes first, enhancements later |

## Technical Notes

**Why `h2` crate?**
- Maintained by tokio-rs team (same team as tokio)
- Used by hyper, tonic, and other major projects
- Implements RFC 7540 (HTTP/2) and RFC 7541 (HPACK)
- Async/await native, tokio-integrated
- Battle-tested in production

**Migration Pattern:**
```
Custom Implementation          →  h2 Crate
─────────────────────────────────────────────────
Frame::parse()                 →  h2::server::accept()
HPACK decoder                  →  Built into h2::server
Stream state machine           →  Managed by h2 crate
Manual frame writing           →  h2::client::send_request()
Connection preface handling    →  Automatic in h2::handshake()
```

**What We Keep:**
- Redaction engine integration
- Policy application
- Host extraction
- Upstream selection
- TLS handling
- Connection pooling

**What We Delegate to `h2`:**
- Frame parsing and writing
- HPACK encoding/decoding
- Stream multiplexing
- Flow control
- Connection management
- Error handling (protocol-level)
