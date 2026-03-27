# TODO Updates - Based on User Feedback

## P1: Redaction Duplication - FIXED ✅

**Status**: COMPLETED
- Extracted common `apply_redaction_rule()` helper function
- Both `redact_text()` and `redact_in_place()` now use shared logic
- ~60 lines of duplication removed
- All 368+ tests passing

**Key Insight**: Prefer `redact_in_place()` for production (zero-copy)
- MITM and proxy may only detect, not always redact
- redact_in_place() is more efficient for streaming

**Files Modified**:
- crates/scred-detector/src/detector.rs

---

## P2: HTTP/2 and Architecture Assessment - NEEDS REASSESSMENT

**Status**: UNCLEAR - TODOs need clarification

**Context**: User states "http2 must be fully implemented"
This suggests HTTP/2 is either:
1. Already fully implemented (TODOs are outdated)
2. Partially implemented (TODOs are accurate)
3. Not implemented (TODOs are aspirational/future work)

### Current TODOs Found

**File**: crates/scred-mitm/src/lib.rs
```rust
// TODO: Replace with h2_mitm_handler (new h2 crate integration)
// TODO: Export new h2_mitm_handler instead
```

**File**: crates/scred-mitm/src/mitm/tls_mitm.rs
```rust
// TODO: Phase 1.2+ - Implement H2 upstream support via h2 crate
// TODO: Phase 1.2 - Replace with h2_mitm_handler (new h2 crate integration)
```

**File**: crates/scred-mitm/src/mitm/upstream_connector.rs
```rust
// Future work: TODO - Implement true HTTP/2 multiplexing
```

**File**: crates/scred-proxy/src/main.rs
```rust
// TODO: Full h2c upstream proxy (phase 1.3 extension)
```

### Questions for Reassessment

1. **Is HTTP/2 actually implemented?**
   - Check if h2 crate is used anywhere
   - Check if HTTP/2 frames are being parsed/handled
   - Check actual protocol support in TLS MITM

2. **Are these TODOs outdated comments?**
   - If HTTP/2 is implemented, remove TODOs
   - Document what actually works
   - Update comments to reflect reality

3. **What's actually needed for HTTP/2 support?**
   - ALPN negotiation (likely done in TLS layer)
   - HTTP/2 frame parsing (h2 crate?)
   - Multiplexing support (per TODO)
   - Redaction in HTTP/2 streams

### MITM Architecture Question

**Important**: User states "mitm and proxy may actually only detect and not redact"

This is a critical architectural insight:
- Detection: Always happens
- Redaction: May not happen in mitm/proxy (detected only)
- Redaction: Happens later in pipeline?

This affects:
- How we handle HTTP/2 streams
- How we handle chunked encoding
- What the TODO comments mean

### Recommendation

Before P2 can be fixed, we need:
1. Code inspection: Is HTTP/2 actually implemented?
2. Architecture review: Where does redaction actually happen?
3. TODO audit: Which comments reflect reality vs aspirations?

---

## P3: Code Cleanup - PROCEED ✅

**Status**: APPROVED

Recommended cleanup tasks:
1. ✅ Remove/consolidate 9 benchmark files (1 hour)
2. ✅ Move tests from source files to tests/ (2-3 hours)
3. ✅ Move bin/validate_debug.rs to examples/ (15 min)
4. ✅ Improve test error handling (30 min)
5. ✅ Update async/sync patterns (1 hour)

**Priority**: After P1 and P2 fixes

---

## Summary

| Phase | Status | Action |
|-------|--------|--------|
| P1 | ✅ COMPLETE | Duplication fixed, tests passing |
| P2 | ⏳ NEEDS REVIEW | Assess HTTP/2 + architecture |
| P3 | ✅ APPROVED | Schedule cleanup work |

