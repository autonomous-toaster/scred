# Detection Optimization: Critical Findings & Action Plan

**Date**: March 27, 2026  
**Current Throughput**: 40-45 MB/s (with FrameRing fix)  
**Target**: 125 MB/s  
**Gap**: 3.1x improvement (212% increase needed)

---

## Critical Discovery: Where 79% of Time Is Spent

### Detection Breakdown
```
Measured Components:
├─ Simple Prefix (Aho-Corasick):  556.9 MB/s    6% of time
├─ Validation (Aho-Corasick):     315.1 MB/s   12% of time
├─ JWT (Regex):                  2249.8 MB/s    2% of time
└─ OTHER (SSH, URI, Multiline): Unknown        79% of time ← BOTTLENECK

End-to-End: 37.9-41.6 MB/s
```

### What's in "Other"?
From code analysis: detect_all() calls multiple sub-detectors:
1. **detect_ssh_keys()** - Multiline SSH key detection
2. **detect_uri_patterns()** - Database URIs + webhook patterns
3. **detect_regex_patterns()** - 18 regex-based patterns
4. **String conversions** - UTF-8 validation, allocations
5. **Match combining** - Merging results from all detectors

### The Math
```
Total time:            0.264s (for 10MB)
Accounted for:         0.054s (Simple 0.018s + Validation 0.032s + JWT 0.004s)
Unaccounted (OTHER):   0.210s (79.5%)

If OTHER can be optimized to match Simple/Validation speed (400+ MB/s):
- Current OTHER:  52.5 MB/s
- Optimized:     400+ MB/s (7.6x improvement)
- Result:        7.6x / 79.5% contribution = ~7% overall improvement
- Final:         40 MB/s → 43 MB/s (modest)

But if OTHER is truly slow (sub-50 MB/s) due to inefficiency:
- Could be regex compilation overhead
- Could be string allocation overhead  
- Could be unnecessary duplicate processing
- Fixing could yield 2-3x improvement in that component
- Result:        2x / 79.5% contribution = ~16% overall improvement
- Final:         40 MB/s → 46+ MB/s (better)
```

---

## Hypothesis: Where the Bottleneck Really Is

### Most Likely Culprits (in order of probability)

**1. Regex Compilation Overhead (HIGH PROBABILITY)**
- 18 regex patterns checked per input
- If patterns are re-compiled each time: Major waste
- Fix: Use lazy_static or once_cell to cache compiled regexes
- Expected improvement: 3-5x in regex path, 20-30% overall

**2. String Allocation Overhead (HIGH PROBABILITY)**
- UTF-8 validation on entire input each time
- String cloning for substrings
- Match object allocations
- Fix: Use byte slices, avoid allocations in hot path
- Expected improvement: 2-3x in allocation path, 15-25% overall

**3. SSH Key Multiline Pattern Matching (MEDIUM PROBABILITY)**
- SSH keys span multiple lines
- Lookahead buffer might need special handling
- Could involve multiple passes
- Fix: Optimize multiline boundary detection
- Expected improvement: 1.5-2x in SSH path, 10-15% overall

**4. URI Pattern Detection (MEDIUM PROBABILITY)**
- Complex regex patterns for database URIs
- Webhook pattern matching
- Could involve multiple regex checks
- Fix: Combine into single pattern or use Aho-Corasick
- Expected improvement: 2-3x in URI path, 10-20% overall

**5. Multiple Detection Passes (MEDIUM PROBABILITY)**
- detect_all() calls multiple sub-functions
- Each sub-function may process entire input
- Results combined together
- Fix: Combine into single pass
- Expected improvement: 1.5-2x, 10-15% overall

---

## Recommended Investigation: Flamegraph Profiling

To identify exact bottleneck:

```bash
# 1. Build with symbols
cd /path/to/scred
cargo build --release -g

# 2. Run profiling
perf record -F 99 --call-graph=dwarf ./target/release/profile_detection

# 3. Generate flamegraph
perf script | /path/to/flamegraph.pl > flame.svg

# 4. Open and inspect
open flame.svg  # or view in browser
```

### What to Look For
- Which function takes most time?
- Is it regex, string operations, or detection logic?
- How deep is the call stack? (indicates overhead)
- Are there any unexpected hot spots?

---

## Optimization Strategies (Ranked by Expected Impact)

### Strategy 1: Cache Regex Patterns (HIGHEST PRIORITY)
**Effort**: 1-2 hours  
**Expected improvement**: 20-30% (if regex is bottleneck)  
**Implementation**:
```rust
use once_cell::sync::Lazy;

static REGEX_CACHE: Lazy<HashMap<&str, Regex>> = Lazy::new(|| {
    // Pre-compile all regex patterns once
    let mut cache = HashMap::new();
    cache.insert("ssh", Regex::new(SSH_PATTERN).unwrap());
    cache
});
```

**Risk**: Low (cache is straightforward)

### Strategy 2: Reduce String Allocations (HIGH PRIORITY)
**Effort**: 2-3 hours  
**Expected improvement**: 15-25% (if allocations are bottleneck)  
**Implementation**:
- Use byte slices instead of String conversions
- Cache UTF-8 validation results
- Use String::from_utf8_lossy() less often
- Avoid intermediate String allocations

**Risk**: Medium (affects many code paths)

### Strategy 3: Combine Detection Passes (MEDIUM PRIORITY)
**Effort**: 3-4 hours  
**Expected improvement**: 10-20% (if multiple passes are issue)  
**Implementation**:
- Single pass through input
- Collect all matches in one go
- Avoid repeated input scanning

**Risk**: Medium (architectural change)

### Strategy 4: Aho-Corasick for Regex Patterns (MEDIUM PRIORITY)
**Effort**: 2-3 hours  
**Expected improvement**: 10-15% (convert regex to AC)  
**Implementation**:
- Convert simple regex to Aho-Corasick
- Create combined pattern automaton
- Single pass matching

**Risk**: Medium (requires pattern conversion)

---

## Action Plan for Next Session

### Phase 1: Profiling (1-2 hours)
1. Build with symbols: `cargo build --release -g`
2. Run flamegraph: `perf record + flamegraph.pl`
3. Identify hot function
4. Estimate improvement potential
5. Document findings

### Phase 2: Quick Win (1 hour)
Implement regex caching (Strategy 1):
- Create static regex cache
- Pre-compile all patterns
- Measure improvement
- Expected: 20-30% if regex is bottleneck

### Phase 3: Deep Optimization (2-4 hours)
Based on flamegraph findings:
- If allocations: Reduce string operations
- If regex: Use Aho-Corasick or cache patterns
- If multiline: Optimize SSH key detection
- If multiple passes: Combine into single pass

### Phase 4: Verify Results (1 hour)
- Benchmark after each change
- Measure cumulative improvement
- Assess gap to 125 MB/s target

---

## Expected Outcomes

### Pessimistic Scenario
- Find single bottleneck
- Fix improves by 20%
- Final: 40 MB/s → 48 MB/s
- Gap to 125 MB/s: 2.6x

### Realistic Scenario
- Find 2-3 bottlenecks
- Fix each by 15-30%
- Final: 40 MB/s → 55-65 MB/s
- Gap to 125 MB/s: 1.9-2.3x

### Optimistic Scenario
- Find major inefficiency (5-10x faster possible)
- Multiple fixes compound
- Final: 40 MB/s → 80-100 MB/s
- Gap to 125 MB/s: 1.25-1.6x

### Best Case
- Major architectural issue found
- Redesign detection pipeline
- Final: 40 MB/s → 120+ MB/s
- **Target achieved!**

---

## Current Architecture Assessment

### What's Working Well
✅ Aho-Corasick for simple prefix (556.9 MB/s alone)  
✅ Aho-Corasick for validation (315.1 MB/s alone)  
✅ In-place redaction (3600+ MB/s)  
✅ Frame ring buffer (45.4 MB/s)  

### What Needs Improvement
🔴 "Other" detectors (52.5 MB/s, 79% of time)  
🔴 Possible regex overhead  
🔴 Possible allocation overhead  
🔴 Possible multiline complexity  

---

## Key Metrics Summary

| Metric | Value | Target | Gap |
|--------|-------|--------|-----|
| Current throughput | 40-45 MB/s | 125 MB/s | 3.1x |
| Simple detection | 556.9 MB/s | ✓ Excellent | N/A |
| Validation | 315.1 MB/s | ✓ Good | N/A |
| Other detection | 52.5 MB/s | 300+ MB/s | 6x |
| FrameRing | 45.4 MB/s | 50+ MB/s | ~1.1x |

---

## Conclusion

The detection pipeline has a clear 79% bottleneck in "Other" detectors (SSH, URI, Multiline, Regex).
Fixing this component is critical to reaching the 125 MB/s target.

**Next session priorities**:
1. Profile with flamegraph (1-2h) to identify exact function
2. Implement quick wins (1-2h) - likely regex caching
3. Deep optimization based on findings (2-4h)

This is the key to unlocking real performance gains. Once "Other" detectors are optimized,
combined with existing Phase 1-2 improvements, we should be able to reach 125 MB/s target.

