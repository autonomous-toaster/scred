# 🔴 Strategic Reassessment: Custom HTTP/2 vs h2 Crate

**Status**: DECISION REQUIRED | **Priority**: HIGH | **Impact**: Blocks HTTP/2 rollout

## Executive Summary

SCRED has invested 200+ hours over 9+ months building a custom HTTP/2 implementation from scratch (3,900+ LOC). **This implementation is currently broken** (curl reports "Error in the HTTP2 framing layer").

**Critical Question**: Is the custom implementation justified, or should we migrate to the mature h2 crate?

### Quick Comparison

| Metric | Custom | h2 Crate |
|--------|--------|----------|
| **LOC** | 3,900+ | 50,000+ |
| **Status** | ❌ Broken | ✅ Production-ready |
| **RFC 7540** | 85% | 99%+ |
| **RFC 7541** | 90% | 99%+ |
| **Maintenance** | High debt | Battle-tested |
| **Time to fix** | 20-40h more | ~10-15h to integrate |
| **Time invested** | 200-300h | N/A |

---

## Problem Statement

### Current Situation
- **9+ months of development** with custom HTTP/2 implementation
- **14 modules** in `crates/scred-http/src/h2/`
- **80+ unit tests passing**, but **real-world curl fails**
- **HPACK encoder works**, but frame transmission broken
- **Huffman decoder** is custom implementation with incomplete table

### Why This Matters
The custom HTTP/2 is a **critical blocker** for:
- MITM proxy HTTP/2 support
- Transparent header interception
- Real-time header redaction
- Production deployment

But if it's not working, we need to question the approach.

---

## Why Custom HTTP/2? (Unvalidated Assumptions)

### Hypothesis 1: Header Redaction Needs Frame-Level Control
- SCRED intercepts, redacts, modifies headers
- Custom HTTP/2 gives per-frame visibility
- **Could h2 work?** Maybe with adapter layer
- **Status**: Not documented, not validated

### Hypothesis 2: Per-Stream Redaction Engine
- Each stream has its own redaction state
- Custom implementation allows tapping into stream lifecycle
- **Could h2 work?** h2 has streams, just need hooks
- **Status**: Not documented, not validated

### Hypothesis 3: MITM Bidirectional Role
- Must act as both HTTP/2 client AND server simultaneously
- Custom implementation supports arbitrary forwarding
- **Could h2 work?** Unknown, needs investigation
- **Status**: Not validated

### Hypothesis 4: Custom Backpressure
- Intelligent buffering for redaction processing
- Custom flow control for throughput tuning
- **Could h2 work?** h2 has standard RFC 7540 flow control
- **Status**: Not documented if standard is insufficient

### Reality Check
⚠️ **NONE of these reasons are documented in code comments or code architecture**

---

## Cost-Benefit Analysis

### Path A: Continue with Custom Implementation ❌
**Option A-1: Debug Current Issues**
- Estimate: 20-40 more hours to fix framing error
- Maintenance: Ongoing burden of 3,900 LOC HTTP/2 code
- Risk: May still not work after 20-40 hours
- Benefit: Full protocol control, custom optimizations
- Verdict: **High risk, uncertain payoff**

**Option A-2: Complete & Stabilize**
- Estimate: 50-100 hours total remaining
- Maintenance: Maintain in perpetuity
- Risk: Security edge cases, RFC compliance gaps
- Benefit: Full control
- Verdict: **Massive sunk cost, unclear necessity**

### Path B: Migrate to h2 Crate ✅
**Option B-1: Full Migration**
- Estimate: 10-15 hours to integrate h2
- Maintenance: Use battle-tested, actively maintained code
- Risk: May need architecture refactoring
- Benefit: RFC 7540/7541 compliance, stability
- Verdict: **Fast, lower risk, proven alternative**

**Option B-2: Hybrid (h2 + Custom Redaction Layer)**
- Estimate: 15-20 hours (h2 integration + thin adapter)
- h2 handles: Frame parsing, HPACK compression, flow control
- Custom handles: Header redaction, stream interception
- Benefit: Stable protocol + custom redaction where needed
- Verdict: **Best balance, fast ROI**

### Sunk Cost Analysis

```
Current state:
- Invested: 200-300 hours
- Result: Broken implementation
- More hours: 20-40 to "fix"
- Success rate: Unknown

Alternative:
- Investment: 10-15 hours
- Result: Production-ready HTTP/2
- Success rate: ~99% (h2 is proven)

Decision:
- Keep investing in broken approach? 🤔
- Or switch to proven approach? ✅
```

---

## Critical Blockers to Investigate

Before continuing with custom implementation, need answers:

### 1. MITM Bidirectional Role
- **Question**: Can h2 act as both client and server simultaneously?
- **Why**: MITM proxy needs to forward frames bidirectionally
- **Investigation**: Review h2 documentation, test proof-of-concept

### 2. Header Redaction Hooks
- **Question**: Does h2 expose hooks for intercepting headers?
- **Why**: Need to inspect/modify every header frame
- **Investigation**: Review h2 API for stream callbacks, frame events

### 3. Adapter Layer Feasibility
- **Question**: Can we wrap h2 with a thin redaction layer?
- **Why**: Avoid reimplementing entire HTTP/2 stack
- **Investigation**: Design adapter architecture, estimate LOC

### 4. Redaction Requirements
- **Question**: What features truly need frame-level protocol control?
- **Why**: Determine if h2's abstractions sufficient
- **Investigation**: Review redaction engine specs, identify blockers

---

## Decision Framework

### Choose **Path B (Migrate to h2)** If:
- ✅ h2 can handle MITM bidirectional role
- ✅ Redaction doesn't require frame-level control
- ✅ Adapter layer is simpler than custom implementation
- ✅ Team agrees on faster integration cost/benefit

### Choose **Path A (Continue Custom)** If:
- ✅ Redaction REQUIRES frame-level protocol control that h2 can't provide
- ✅ MITM bidirectional role impossible with h2
- ✅ Custom implementation can be fixed and stabilized in reasonable time
- ✅ Team willing to maintain 3,900+ LOC indefinitely
- ✅ All blockers above answered with "Cannot use h2"

---

## Immediate Next Steps

### 1. Investigation (1-2 hours)
- [ ] Document specific HTTP/2 features actually needed
- [ ] Review h2 crate documentation
- [ ] Check h2 for MITM bidirectional support
- [ ] Identify actual blocking requirements
- [ ] Quick proof-of-concept with h2 + basic redaction

### 2. Team Decision (30 minutes)
- [ ] Present findings to team
- [ ] Decide: Path A (continue custom) vs Path B (migrate to h2) vs Path C (hybrid)
- [ ] Document rationale

### 3. Execute Decision
**If Path B chosen**:
- [ ] Create migration plan with timeline
- [ ] Start h2 integration
- [ ] Build redaction adapter
- [ ] Port tests to h2-based architecture
- [ ] Deprecate custom HTTP/2 modules

**If Path A chosen**:
- [ ] Document protocol requirements
- [ ] Create architectural spec for stabilization
- [ ] Set "HTTP/2 working with curl" milestone with acceptance criteria

---

## Key Files

**Current Custom Implementation**:
- `crates/scred-http/src/h2/` (14 modules, 3,900 LOC)
  - h2_connection.rs, h2_frame_handler.rs, h2_hpack_rfc7541.rs, h2_huffman.rs, etc.

**MITM Usage**:
- `crates/scred-mitm/src/mitm/h2_mitm.rs`
- `crates/scred-mitm/src/mitm/h2_handler.rs`

**Redaction Integration**:
- `crates/scred-redactor/src/lib.rs`

**Cargo.toml**:
- Already has `http2 = "0.5"` but not used
- Would add `h2 = "0.4"` for migration

---

## References

- **h2 crate**: https://crates.io/crates/h2
- **RFC 7540 (HTTP/2)**: https://tools.ietf.org/html/rfc7540
- **RFC 7541 (HPACK)**: https://tools.ietf.org/html/rfc7541
- **SCRED Redaction**: `crates/scred-redactor/src/lib.rs`

---

## Recommendation

🚨 **DO NOT CONTINUE debugging custom HTTP/2 without first answering these questions:**

1. Can h2 do what we need? (Investigate: 1-2 hours)
2. Is custom implementation actually necessary? (Validate assumptions)
3. What's the team's risk tolerance for custom vs proven? (Decide together)

**If h2 is viable** → **Migrate immediately** (saves 200+ hours of maintenance)

**If custom is necessary** → **Document why** and set realistic stabilization goals

**Current state** → **Unsustainable** (broken, expensive to maintain, uncertain necessity)

---

## TODO

See: `TODO-7d4d5202` - Full detailed investigation checklist

Created: 2026-03-22
