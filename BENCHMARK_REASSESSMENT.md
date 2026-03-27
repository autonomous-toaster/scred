# Benchmark Reassessment Report

**Date**: March 27, 2026  
**Branch**: main (Post Phase 1 Merge)  
**Status**: Ready for Next Investigation

## Current Performance Summary

### Measured Baseline
```
Detection throughput:        35.02 MB/s
Streaming throughput:        40.1 MB/s
Target requirement:          125 MB/s
Gap to target:               3.1x improvement needed (312%)
```

## What Has Been Implemented

### Phase 3: Aho-Corasick Integration ✅
- SIMPLE_PREFIX_AUTOMATON: OnceLock-cached multi-pattern matcher
- VALIDATION_AUTOMATON: OnceLock-cached validation patterns
- URI patterns: DATABASE_AC and WEBHOOK_AC for URI detection
- Status: Integrated into detect_all() function

**Expected impact**: Reduce pattern matching from O(k*n) to O(n+m)  
**Actual impact measured**: 35-40 MB/s (limited improvement)

### Phase 1: Code Quality & Infrastructure ✅
- CLI streaming consolidation (59% code reduction)
- BufferPool module (3 × 65KB pre-allocated)
- In-place redaction API (3600+ MB/s - extremely fast)
- Status: Merged into main

**Expected impact**: +15% throughput  
**Actual impact measured**: 0-5% (optimization targets non-bottleneck)

## Critical Finding

**Detection is the bottleneck (83.7% of execution time), not redaction**

Component breakdown:
- Detection: 38 MB/s (83.7% of combined time)
- Redaction: 3600+ MB/s (0.9% of combined time)
- Other: 15.4% of combined time

Implication: Even with Aho-Corasick integrated, we're only achieving 35-40 MB/s.
This suggests other bottlenecks exist WITHIN the detection path.

## Potential Remaining Bottlenecks

1. **Lookahead Buffer Management** (5-10% impact)
   - `let mut combined = lookahead.clone();` per chunk
   - Can be optimized with ring buffer

2. **String Allocations** (10-20% impact)
   - Match struct contains owned String fields
   - Pattern names and redacted text cause allocations

3. **Multiple Detection Passes** (5-15% impact)
   - detect_all() calls multiple sub-detectors
   - Results combined together

4. **UTF-8 Validation** (5-10% impact)
   - `from_utf8_lossy()` validates entire input
   - Can be optimized or cached

5. **Regex Pattern Fallback** (3-5% impact)
   - 18 REGEX_PATTERNS still in hot path
   - Could be replaced with Aho-Corasick

## Investigation Plan

To identify exact bottleneck:

1. **Verify Aho-Corasick Usage**
   - Are ALL patterns using Aho-Corasick?
   - Or are some still using regex?

2. **Profile with Flamegraph**
   - Run: `perf record -F 99 ./target/release/profile_phase1`
   - Generate: `flamegraph.pl > flame.svg`
   - Identify hot function line-by-line

3. **Measure Individual Detectors**
   - Time simple_prefix detection alone
   - Time validation detection alone
   - Time JWT detection alone
   - Time SSH key detection alone
   - Time URI detection alone

4. **Optimize Bottleneck**
   - Once identified, optimize the specific function
   - Expected improvement: 10-30%

5. **Re-measure**
   - Verify improvement
   - Assess gap to 125 MB/s

## Expected Outcomes

### Conservative Estimate
- Improvement: 5-10%
- Final: 38-44 MB/s
- Gap: 2.8-3.3x to target

### Realistic Estimate
- Improvement: 20-50%
- Final: 42-60 MB/s
- Gap: 2.1-3.0x to target

### Optimistic Estimate
- Improvement: 50-100%
- Final: 70-80 MB/s
- Gap: 1.5-1.8x to target

## Conclusion

Phase 1 is merged and complete with excellent code quality. However, real throughput
improvement requires investigating and optimizing the detection path beyond just
Aho-Corasick. The next phase should focus on identifying and optimizing the specific
bottleneck within detection (lookahead buffer, string allocations, UTF-8 validation, etc.).

Created TODO-a69cd1d8 with detailed investigation and optimization plan.

