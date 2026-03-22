# Autoresearch: SCRED Phase 2 - HTTP/2 Upstream with Frame Forwarding

## Objective

Fix Phase 2 implementation: When HTTP/2 upstream is detected, handle properly based on **client protocol**:

**Current behavior (INCOMPLETE)**:
- Upstream HTTP/2 → Transcode to HTTP/1.1 → Always send HTTP/1.1 to client

**Target behavior (CORRECT - Smart Routing)**:
- **Client HTTP/2 + Upstream HTTP/2**: Use `frame_forwarder` for H2↔H2 forwarding with redaction (Scenario 4)
- **Client HTTP/1.1 + Upstream HTTP/2**: Use `H2UpstreamClient` for H1.1→H2↔H2→H1.1 transcoding (Scenarios 1-3)
- **Client HTTP/1.1 + Upstream HTTP/1.1**: Forward normally (existing behavior)

### Why This Matters

| Scenario | Client | Upstream | Current | Correct |
|----------|--------|----------|---------|---------|
| 1 | H1.1 direct | H2 | ✅ Transcode | ✅ Transcode |
| 2 | H1.1 via proxy | H2 | ✅ Transcode | ✅ Transcode |
| 3 | H2 via proxy | H2 | ❌ Transcode (wastes H2) | ✅ Transcode (proxy only speaks H1.1) |
| 4 | H2 direct | H2 | ❌ Transcode (wastes H2) | ✅ Frame forward (proper H2 support) |

**Key**: SCRED can only forward H2↔H2 directly when **both client and upstream are H2 with no proxy in between** (Scenario 4).
All proxy scenarios require transcoding because HTTP/1.1 proxies downgrade the connection.

## Metrics

- **Primary**: `tests_passing` (higher is better)
- **Secondary**: None (keep tests passing, zero regressions)

## How to Run

```bash
cd /Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred
cargo test --lib --all 2>&1 | grep "test result:" | head -1 | sed 's/.* \([0-9]*\) passed.*/\1/'
```

## Files in Scope

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `crates/scred-mitm/src/mitm/tls_mitm.rs` | MITM handler | Replace H2 downgrade logic with frame_forwarder call |
| `crates/scred-http/src/h2/frame_forwarder.rs` | Frame forwarding (existing) | None - already complete |
| `crates/scred-http/src/h2/mod.rs` | H2 module exports | Ensure frame_forwarder is exported |
| Tests | Verify HTTP/2 handling | May need to update or add tests |

## Off Limits

- Do NOT modify: H2UpstreamClient, HpackEncoder (existing implementations)
- Do NOT downgrade H2 to H1.1
- Do NOT remove functionality for HTTP/1.1 upstream

## Constraints

1. Must maintain 458 tests passing (zero regressions)
2. All tests must pass before committing
3. HTTP/1.1 upstream behavior unchanged
4. No new dependencies
5. Production code quality

## What's Been Tried

Nothing yet - this is the first attempt to fix the implementation.

## Implementation Plan

### Step 1: Understand Current State ✅
- Review tls_mitm.rs lines 345-450 (HTTP/2 detection + downgrade logic)
- Verify frame_forwarder module is working (already has tests)
- Check module exports in h2/mod.rs
- **DONE**: See analysis above

### Step 2: Add Client Protocol Detection
- In `handle_tls_mitm()`, after TLS negotiation, detect client ALPN
- Store client protocol (H2 or H1.1)
- Use this to decide routing strategy downstream

### Step 3: Smart Routing Based on Client Protocol
When HTTP/2 upstream is detected:

**If client is HTTP/2 (ALPN "h2")**:
- ✅ Use `frame_forwarder::forward_h2_frames()` (Scenario 4)
- Bidirectional H2↔H2 frame forwarding
- Per-stream header redaction via HeaderRedactor

**If client is HTTP/1.1 (no ALPN or "http/1.1")**:
- ✅ Keep `H2UpstreamClient` transcoding (Scenarios 1-3)
- Transcode H1.1→H2 upstream
- Transcode H2→H1.1 downstream

### Step 4: Update Client Detection
- Get client ALPN from TLS negotiation
- Pass through to protocol handling logic
- Already have: client_alpn info from tls_acceptor

### Step 5: Test & Verify
- Run full test suite (458+ tests)
- Verify H2↔H2 forwarding works (Scenario 4)
- Verify H1.1→H2 transcoding still works (Scenarios 1-3)
- Check header redaction is applied
- Verify no regressions

## Key Components

**frame_forwarder** (already implemented):
```rust
pub async fn forward_h2_frames<C, U>(
    mut client_conn: C,
    mut upstream_conn: U,
    _host: &str,
    config: FrameForwarderConfig,
) -> Result<ForwardingStats>
```

Features:
- Bidirectional frame forwarding
- Stream ID mapping
- Per-stream header redaction
- SETTINGS frame handling
- Connection preface exchange

**Integration point** (tls_mitm.rs):
```rust
// Current: Downgrade logic (lines 349-430)
// Replace with: forward_h2_frames() call
```

## Decision Tree

**When HTTP/2 upstream is detected:**

1. **Did client negotiate H2 (ALPN "h2")?**
   - YES → Use `frame_forwarder::forward_h2_frames()` (Scenario 4 - H2↔H2)
   - NO → Go to step 2

2. **Client is HTTP/1.1**
   - Use `H2UpstreamClient` for transcoding (Scenarios 1-3 - H1.1→H2↔H2→H1.1)
   - Send HTTP/1.1 response back to client

**Result:**
- ✅ Scenario 1 (H1.1 direct → H2): Transcode via H2UpstreamClient
- ✅ Scenario 2 (H1.1 proxy → H2): Transcode via H2UpstreamClient  
- ✅ Scenario 3 (H2 proxy → H2): Transcode via H2UpstreamClient (proxy only speaks H1.1)
- ✅ Scenario 4 (H2 direct → H2): Forward via frame_forwarder

## Success Criteria

✅ All 458 tests pass
✅ HTTP/2 upstream frames are forwarded properly
✅ Header redaction is applied to H2 frames
✅ No regressions in HTTP/1.1 handling
✅ Code compiles without warnings (Phase 2 related)
