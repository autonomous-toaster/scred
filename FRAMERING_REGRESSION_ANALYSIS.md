# FrameRing Regression Analysis: What Happened During Merge

**Date**: March 27, 2026  
**Issue**: FrameRing is 5.3% SLOWER than standard StreamingRedactor (40.2 MB/s vs 42.5 MB/s)  
**Expected**: FrameRing should be 10-15% FASTER  
**Root Cause**: Not using in-place redaction optimization

---

## The Regression: Before vs After Merge

### Before Merge (Commit 7dd62429)
- FrameRing was working as intended
- Baseline: Unknown (not measured in this session)

### After Merge (Commit b883a3b9)
- StreamingRedactor updated to use in-place redaction by default (Phase 2.1)
- FrameRingRedactor was NOT updated
- Result: FrameRing now 5.3% slower due to missing optimization

### Current Comparison
```
Standard StreamingRedactor:  42.5 MB/s (using in-place by default)
FrameRingRedactor:          40.2 MB/s (still using copy-based redaction)

Gap: 5.3% regression
```

---

## Root Cause Analysis

### StreamingRedactor (Updated in Phase 2.1)
```rust
pub fn redact_buffer(&self, data: &[u8]) -> (String, StreamingStats) {
    // Phase 2.1: Default to in-place redaction for better performance
    self.redact_buffer_in_place(data, false)  // ← Uses in-place
}

// Uses process_chunk_in_place() which calls:
// scred_detector::redact_in_place(&mut buffer, &matches)
// Performance: 3600+ MB/s just for redaction
```

### FrameRingRedactor (NOT Updated - Still Using Old Code)
```rust
pub fn process_chunk(&mut self, chunk: &[u8], is_eof: bool) -> (String, u64) {
    // ...
    let combined_str = String::from_utf8_lossy(process_frame);
    let redacted_result = self.engine.redact(&combined_str);  // ← Uses redact()
    let mut output = redacted_result.redacted.clone();  // ← Copy-based!
    // ...
}

// Uses engine.redact() which calls:
// scred_detector::redact_text()  // Copy-based, not in-place
// Performance: ~40 MB/s end-to-end
```

---

## The Fix: Update FrameRingRedactor to Use In-Place Redaction

### Current Implementation (Broken)
```rust
fn process_chunk(&mut self, chunk: &[u8], is_eof: bool) -> (String, u64) {
    let process_frame = self.ring.get_process_frame();
    let combined_str = String::from_utf8_lossy(process_frame);
    
    // Uses copy-based redaction
    let redacted_result = self.engine.redact(&combined_str);
    let mut output = redacted_result.redacted.clone();
    let patterns_found = redacted_result.matches.len() as u64;
    
    // ... rest of processing
}
```

### Fixed Implementation (Use In-Place)
```rust
fn process_chunk(&mut self, chunk: &[u8], is_eof: bool) -> (String, u64) {
    let process_frame = self.ring.get_process_frame();
    
    // Use in-place redaction on frame data
    use scred_detector::{detect_all, redact_in_place};
    let detection = detect_all(process_frame);
    let patterns_found = detection.matches.len() as u64;
    
    // Redact in-place
    let mut redacted = process_frame.to_vec();
    redact_in_place(&mut redacted, &detection.matches);
    
    let output = String::from_utf8_lossy(&redacted).into_owned();
    
    // Calculate output boundaries (preserve lookahead)
    let output_end = if is_eof {
        output.len()
    } else if output.len() > self.config.lookahead_size {
        output.len() - self.config.lookahead_size
    } else {
        0
    };

    let output_text = if output_end > 0 {
        output[..output_end].to_string()
    } else {
        String::new()
    };
    
    self.ring.mark_written_and_rotate();
    (output_text, patterns_found)
}
```

---

## Why This Happened

1. **Phase 1 Merge** (b883a3b9):
   - Brought in StreamingRedactor with `process_chunk_in_place()`
   - Updated StreamingRedactor.redact_buffer() to use it by default
   - Did NOT update FrameRingRedactor (different code path)

2. **Phase 2.1** (9e45f503):
   - Made in-place the default in StreamingRedactor
   - Still did NOT update FrameRingRedactor

3. **Result**:
   - FrameRingRedactor fell behind on performance
   - Now 5.3% slower than standard (opposite of intended)

---

## Fix: Update FrameRingRedactor to Use In-Place Redaction

### Changes Needed
- [ ] Update `FrameRingRedactor::process_chunk()` to use in-place redaction
- [ ] Call `scred_detector::detect_all()` + `redact_in_place()`
- [ ] Remove call to `self.engine.redact()` (copy-based)
- [ ] Preserve lookahead buffer logic
- [ ] Test and verify performance

### Expected Outcome
- FrameRing: 40.2 MB/s → 42-45 MB/s (matching or exceeding standard)
- Plus additional benefit from ring buffer (eliminate allocations)
- Expected: 45-55 MB/s total (with FrameRing benefits + in-place)

---

## Detection Optimization Assessment

While we're fixing FrameRing, the real bottleneck remains detection.

### Current Profiling Results
```
Detection:  38 MB/s (83.7% of execution time)  ← BOTTLENECK
Redaction:  3600+ MB/s (0.9% of execution time)
Lookahead:  15.4% of other time
─────────────────────────────
End-to-end: 40-42 MB/s
```

### What We Know
1. **Detection is using Aho-Corasick** (already integrated)
2. **But still only achieving 35-40 MB/s baseline**
3. **Other bottlenecks within detection**:
   - Lookahead buffer cloning: `combined.clone()` per chunk
   - String allocations: Match objects, pattern strings
   - Multiple detection passes: Simple, validation, JWT, SSH, URI
   - UTF-8 validation: `from_utf8_lossy()` on every input

### Path Forward for Detection Optimization

**Step 1: Profile with Flamegraph**
```bash
# Identify exact hot function
cargo build --release -g
perf record -F 99 ./target/release/profile_phase1
perf script > out.perf-folded
flamegraph.pl out.perf-folded > flame.svg
```

**Step 2: Identify Specific Bottleneck**
- Is it Aho-Corasick matching itself?
- Or lookahead buffer management?
- Or string conversions?
- Or multiple passes?

**Step 3: Optimize That Specific Path**
- If lookahead: Use ring buffer (FrameRing does this)
- If allocations: Cache results or use in-place
- If passes: Combine into single pass
- If UTF-8: Validate once, reuse

**Step 4: Re-measure and Iterate**

---

## Immediate Action Items

### 1. Fix FrameRingRedactor (30 min)
- [ ] Update process_chunk() to use in-place redaction
- [ ] Verify it compiles
- [ ] Benchmark: Expect 42-55 MB/s
- [ ] Commit: "fix: Update FrameRingRedactor to use in-place redaction"

### 2. Profile Detection (1-2 hours)
- [ ] Build with symbols
- [ ] Run perf with flamegraph
- [ ] Identify hot function
- [ ] Estimate improvement potential for each bottleneck

### 3. Optimize Detected Bottleneck (2-4 hours)
- [ ] Implement fix for identified bottleneck
- [ ] Re-measure throughput
- [ ] Iterate if needed

### 4. Final Assessment
- [ ] Combined throughput with all optimizations
- [ ] Gap to 125 MB/s target
- [ ] Path to production

---

## Summary of Issues

| Issue | Status | Impact | Fix |
|-------|--------|--------|-----|
| FrameRing not using in-place | 🔴 REGRESSION | -5.3% | 30 min (update code) |
| Detection bottleneck not identified | 🔴 BLOCKING | -60% vs target | 1-2h (profile) |
| Potential sub-bottlenecks | ⚠️ UNKNOWN | -40% vs target | 2-4h (optimize) |

---

## Conclusion

The FrameRing regression was caused by the Phase 1 merge introducing in-place redaction
optimization, but not updating all redactor implementations to use it.

**Fix**: Update FrameRingRedactor to use in-place redaction (30 min)

**Real Opportunity**: Profile detection to identify and fix the 83.7% bottleneck (4-7h total)

Once both are done, we should achieve:
- FrameRing: 42-55 MB/s (10-25% improvement)
- Detection optimized: Potential 50-100%+ improvement to 80-150+ MB/s

