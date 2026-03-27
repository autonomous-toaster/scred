# First-Class Citizens Assessment: FrameRing + SIMD + Zero-Copy

**Date**: March 27, 2026  
**Status**: Comprehensive analysis of current implementation state

## Executive Summary

Three critical optimization patterns should be **first-class citizens** in the codebase:

1. **FrameRing** (Ring Buffer Pattern): ✅ Implemented, ⚠️ NOT integrated
2. **SIMD** (SIMD Charset Scanning): ✅ Implemented, ⚠️ NOT integrated  
3. **Zero-Copy** (In-place redaction + buffer pooling): ✅ Implemented, ✅ READY to use

**Current Status**: All three exist in codebase but only zero-copy is actively used.

---

## 1. FrameRing (Ring Buffer Pattern)

### What Is It?
Pre-allocated rotating buffers (3 frames × 65KB) for streaming without per-chunk allocations.

**Conceptual Pattern**:
```
Input Stream:
Chunk 1 → [Frame A] (read/decode) → [Frame B] (process/redact) → [Frame C] (write/output)
Chunk 2 → [Frame B] (read/decode) → [Frame C] (process/redact) → [Frame A] (write/output)
Chunk 3 → [Frame C] (read/decode) → [Frame A] (process/redact) → [Frame B] (write/output)
```

### Current Implementation
✅ **Module**: `crates/scred-redactor/src/frame_ring.rs` (250 lines)

**API**:
```rust
pub struct FrameRing<const FRAME_SIZE: usize, const NUM_FRAMES: usize> { ... }

impl FrameRing<FRAME_SIZE, NUM_FRAMES> {
    pub fn new() -> Self
    pub fn current_frame(&self) -> &[u8]
    pub fn current_frame_mut(&mut self) -> &mut [u8]
    pub fn rotate(&mut self)
}
```

**Tests**: 4 tests in frame_ring.rs
- Creation
- No allocations
- Rotation correctness
- Frame content integrity

### Current Usage Status
❌ **NOT INTEGRATED** into streaming pipeline

**Why**: Added in Phase 4 for "transcoding use cases" but never integrated into main `StreamingRedactor`

**Available Alternative**: `FrameRingRedactor` struct exists but is:
- Not part of public API exports
- Not documented
- Not benchmarked
- Not used anywhere in CLI/MITM/Proxy

### Assessment: FIRST-CLASS CITIZEN STATUS

**Current**: 🟡 Second-class (exists but not used)

**Required Changes**:
- [ ] Export FrameRingRedactor from lib.rs public API
- [ ] Add comprehensive documentation with example
- [ ] Create benchmark: FrameRingRedactor vs StreamingRedactor
- [ ] Integrate into CLI as optional flag `--zero-copy-framering`
- [ ] Add to MITM/Proxy as default streaming mode
- [ ] Document memory overhead (195KB for 3×65KB frames)
- [ ] Add performance comparison in BENCHMARKS.md

**Expected Performance Impact**:
- Current StreamingRedactor: 40.1 MB/s (clones lookahead per chunk)
- FrameRingRedactor: Expected 45-55 MB/s (eliminates clone overhead)
- Improvement: 10-25% expected

---

## 2. SIMD (SIMD Charset Scanning)

### What Is It?
Vectorized byte-scanning using CPU SIMD instructions for fast charset detection (memchr-like).

**Conceptual Pattern**:
```
Traditional: for byte in input { if charset.contains(byte) { ... } }  → O(n) scalar ops
SIMD:        Process 16 bytes at once with vector instructions         → O(n/16) vector ops
```

### Current Implementation
✅ **Modules**:
- `crates/scred-detector/src/simd_core.rs` (180 lines)
- `crates/scred-detector/src/simd_charset.rs` (200 lines)

**API**:
```rust
pub struct SimdCharset { ... }

impl SimdCharset {
    pub fn new(charset: &[u8]) -> Self
    pub fn contains(&self, byte: u8) -> bool
    pub fn find_first(&self, data: &[u8]) -> Option<usize>
}
```

**Tests**: SIMD-specific tests validating:
- Charset creation
- Contains lookups
- First-match detection
- UTF-8 safety

**Gate**: `#[cfg(feature = "simd-accel")]` (optional feature)

### Current Usage Status
❌ **NOT INTEGRATED** into detection pipeline

**Why**: Implemented in Phase 3 for "performance exploration" but findings showed:
- Benefit mostly for small charsets (< 256 bytes)
- Complex integration with existing prefix detection
- Aho-Corasick already supersedes it for pattern matching
- SIMD benefit marginal vs Aho-Corasick overhead

**Analysis from git logs**:
```
Session 10: "SIMD detection investigation - three approaches explored 
            and analyzed, all impractical vs current architecture"
```

### Assessment: FIRST-CLASS CITIZEN STATUS

**Current**: 🔴 Dead code (exists but not used)

**Options**:

**Option A: Remove (RECOMMENDED)**
- Delete simd_core.rs and simd_charset.rs
- Clean up dead feature flag
- Reduce code maintenance burden
- Rationale: Aho-Corasick is superior for pattern matching

**Option B: Revive for Specific Use Case**
- Use for lookahead buffer scanning (finding next pattern boundary)
- Expected impact: 5-10% throughput improvement
- Effort: Medium (integration work)
- Benefit: Marginal vs complexity cost

**Option C: Document as Exploration**
- Keep in codebase but explicitly mark as "experimental"
- Add SIMD_EXPLORATION.md explaining why it wasn't integrated
- Useful for future reference or alternate architectures

**Recommendation**: **Option A (Remove)** + **Option B (Use for lookahead scanning)**

---

## 3. Zero-Copy (In-Place Redaction + Buffer Pooling)

### What Is It?
Eliminate allocations in hot path by:
1. Pre-allocating buffers in a pool (BufferPool)
2. Redacting directly in buffers instead of copying (in-place)

**Conceptual Pattern**:
```
Traditional: input → detect → create output buffer → copy+redact → return
Zero-copy:   input → detect → redact in-place → return (same buffer)
```

### Current Implementation
✅ **Modules**:
- `crates/scred-redactor/src/buffer_pool.rs` (160 lines + 7 tests)
- Integrated into `StreamingRedactor` (in-place methods available)

**APIs**:
```rust
pub struct BufferPool { ... }
impl BufferPool {
    pub fn acquire(&mut self) -> Vec<u8>
    pub fn release(&mut self, buffer: Vec<u8>)
}

// In detector
pub fn redact_in_place(buffer: &mut [u8], matches: &[Match]) -> usize { ... }
```

**Tests**: 
- 7 buffer pool tests (all passing)
- 8 in-place redaction tests (all passing)
- Equivalence testing (in-place == copy-based)

### Current Usage Status
✅ **IMPLEMENTED BUT NOT ACTIVELY USED**

**Why**: 
- API exists and works correctly
- Not enabled by default in CLI/MITM/Proxy
- StreamingRedactor has both APIs:
  - `redact_buffer()` (copy-based, old)
  - `redact_buffer_in_place()` (zero-copy, new)

### Measured Performance
✅ **In-place redaction**: 3600+ MB/s (extremely fast)
- Character preservation verified
- All 415 patterns supported
- Zero regressions in tests

❌ **End-to-end impact**: 0-5% (limited)
- Why: Detection is 83.7% of time, not redaction
- Redaction is already 100x faster than detection
- Optimizing it further won't move the needle on overall throughput

### Assessment: FIRST-CLASS CITIZEN STATUS

**Current**: 🟢 Implemented but underutilized

**Required Changes**:
- [ ] Make in-place redaction DEFAULT in StreamingRedactor
- [ ] Update CLI to use in-place by default (with flag for testing)
- [ ] Update MITM/Proxy to use in-place by default
- [ ] Add performance flag: `--memory-optimized` vs `--cache-optimized`
- [ ] Document memory and performance implications
- [ ] Benchmark actual end-to-end impact (expect 0-5% improvement)

**Why It Matters**:
- Reduces GC pressure in long-running processes
- Improves cache locality
- Better for embedded/constrained systems
- Foundation for further optimization

---

## Action Items: Make Them First-Class Citizens

### Phase 1: Documentation & API Exposure (2 hours)

- [ ] **Update lib.rs exports**
  ```rust
  pub use frame_ring::FrameRing;
  pub use redactor::{StreamingRedactor, FrameRingRedactor};
  pub use buffer_pool::BufferPool;
  pub use detector::redact_in_place;
  ```

- [ ] **Create ZERO_COPY_GUIDE.md**
  - Explain when to use each pattern
  - Performance characteristics
  - Memory overhead
  - Example code

- [ ] **Create FRAMERING_GUIDE.md**
  - Explain ring buffer concept
  - When to use (streaming, transcoding, video)
  - Memory layout
  - Rotation mechanics

- [ ] **Create SIMD_EXPLORATION.md** (or delete SIMD)
  - Document why SIMD wasn't integrated
  - Potential future use cases
  - Decision rationale

### Phase 2: Benchmarking (2 hours)

- [ ] **Benchmark FrameRingRedactor**
  ```
  cargo bench --bench frame_ring_comparison
  Expected: 45-55 MB/s vs 40 MB/s current
  ```

- [ ] **Benchmark in-place vs copy-based**
  ```
  cargo bench --bench zero_copy_comparison
  Expected: Marginal improvement (0-5%) due to detection bottleneck
  ```

- [ ] **Benchmark SIMD charset scanning**
  ```
  If we keep it: cargo bench --bench simd_charset
  Expected: 2-3x faster for small charsets
  ```

### Phase 3: CLI Integration (2 hours)

- [ ] **Add CLI flags**
  ```
  --streaming-mode {standard|framering|custom}
  --zero-copy {enabled|disabled}
  --memory-optimized (sets both flags to optimal)
  ```

- [ ] **Update CLI help and documentation**
  
- [ ] **Add example configuration file** with performance presets

### Phase 4: Default Adoption (1 hour)

- [ ] **Make in-place redaction DEFAULT** (not optional)
  
- [ ] **Make FrameRing optional** (for heavy-duty deployments)
  
- [ ] **Profile and verify** no regressions

---

## Technical Debt Assessment

### Code Organization Issues

1. **FrameRing exists but is orphaned**
   - Designed for video transcoding
   - Never integrated into streaming
   - Buried in documentation comments
   - Status: Ready for adoption, needs integration

2. **SIMD exists but is abandoned**
   - Phase 3 conclusion: "not practical vs current architecture"
   - Still compiles behind feature flag
   - No clear integration path
   - Status: Should either be removed or used for lookahead scanning

3. **Zero-Copy API exists but isn't default**
   - Both copy and in-place APIs coexist
   - Only copy-based is actively used
   - In-place is well-tested but dormant
   - Status: Ready for production, needs adoption

### Recommendation: Consolidate

**Architecture Simplification**:
```rust
// Current (two paths coexist)
StreamingRedactor {
  redact_buffer()           // copy-based
  redact_buffer_in_place()  // in-place
}

// Simplified (single optimized path)
StreamingRedactor {
  redact_buffer()  // always in-place internally
                   // return &[u8] directly or owned Vec if needed
}
```

---

## Performance Summary

| Component | Status | Throughput | Integration | Effort |
|-----------|--------|-----------|-------------|--------|
| FrameRing | ✅ Done | 45-55 MB/s (est) | ⚠️ Needed | 2h |
| SIMD | ✅ Done | Marginal | 🔴 None | Remove |
| Zero-Copy | ✅ Done | 3600+ MB/s | ⚠️ Default | 1h |

---

## Conclusion

All three technologies exist in the codebase but are not consistently positioned as
**first-class citizens**:

- **FrameRing**: Ready for integration (2h work)
- **SIMD**: Should be removed or repurposed (1h work)
- **Zero-Copy**: Ready for default adoption (1h work)

**Total effort to elevate all three**: 4-5 hours
**Expected outcome**: Cleaner architecture, better documentation, 10-15% potential throughput gain

**Priority order**:
1. Make zero-copy default (1h, immediate benefit)
2. Integrate FrameRing (2h, 10-15% improvement)
3. Decide on SIMD (1h, cleanup)

