# Phase 4: Performance Optimization (6-8 hours remaining)

**Goal**: Improve from 31.4 MB/s to 50+ MB/s for streaming redaction
**Focus**: Regex compilation overhead reduction
**Approach**: Lazy pattern compilation + aggressive caching

## Current Bottleneck Analysis

### Where Time is Spent
- Regex matching: ~80% of time
- Pattern recompilation: Happens per chunk (inefficient)
- String operations: ~15% of time
- Overhead: ~5% of time

### Optimization Opportunities

1. **Lazy Compilation** (High Impact)
   - Currently: All 188 patterns compiled upfront
   - Proposal: Only compile patterns that match input prefix
   - Estimated gain: 2-3x speedup

2. **Pattern Filtering** (High Impact)
   - Use quick prefix checks before regex matching
   - Skip expensive regex if prefix not present
   - Estimated gain: 1.5-2x speedup

3. **Caching** (Medium Impact)
   - Cache compiled regexes in static
   - Already doing lazy_static - verify it's working
   - Estimated gain: 1-1.5x speedup

4. **Zig Integration** (Low Priority for Phase 4)
   - Implement Tier 1 in native Zig
   - Only pursue if Tier 1 patterns are hot path
   - Estimated gain: Small (Tier 1 is 10/198 patterns)

## Implementation Plan

### Step 1: Profile Current Implementation
- [ ] Time each pattern type
- [ ] Identify hot patterns
- [ ] Measure regex compilation time
- [ ] Baseline: 31.4 MB/s

### Step 2: Implement Pattern Filtering
- [ ] Add prefix extraction for all patterns
- [ ] Early rejection on prefix mismatch
- [ ] Measure impact

### Step 3: Lazy Regex Compilation
- [ ] Only compile patterns that have matching prefixes
- [ ] Cache compiled regexes
- [ ] Measure impact

### Step 4: Optimize Hot Patterns
- [ ] Profile which patterns match most
- [ ] Consider simplifying their regex
- [ ] Consider moving common patterns to Tier 1
- [ ] Measure impact

### Step 5: Benchmark with Real Data
- [ ] Test with 100MB+ log file
- [ ] Test with GB-scale data
- [ ] Measure steady-state performance
- [ ] Compare with target (50 MB/s)

## Success Criteria

- [ ] 50+ MB/s sustained throughput
- [ ] All integration tests still passing
- [ ] Character preservation maintained
- [ ] False positive rate unchanged

## File Changes Expected

```
crates/scred-redactor/src/
├── redactor.rs (add pattern filtering)
├── streaming.rs (optimize chunk processing)
└── hybrid_detector.rs (lazy compilation)
```

## Timeline

- Profile & measure: 1 hour
- Pattern filtering: 1 hour
- Lazy compilation: 2 hours
- Testing & validation: 1.5 hours
- Real-world benchmarking: 1-2 hours
- **Total: 6.5-8 hours**

## Acceptance Criteria

✅ 50+ MB/s measured throughput
✅ 8/8 integration tests passing
✅ 37/37 unit tests passing
✅ No regressions in false positive rate
✅ Character preservation maintained

---

**Next Action**: Start profiling current implementation
**Target Completion**: Within 2-3 hours
