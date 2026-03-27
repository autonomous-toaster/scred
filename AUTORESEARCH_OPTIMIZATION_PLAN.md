# SCRED Speed & Throughput Optimization - Full Stack

**Date**: March 27, 2026  
**Baseline**: 48 MB/s streaming throughput (10MB, 64KB chunks, with secrets)  
**Target**: 200+ MB/s (4.2x improvement)  
**Previous Peak**: 154 MB/s (from earlier session notes - higher workload?)

---

## Baseline Analysis

### Current Performance
- **Streaming throughput**: 48.0 MB/s (run 1), 46.7 MB/s (run 2), 29.3 MB/s (run 3 - slower)
- **Test setup**: 10MB data, 64KB chunks, 10485760 patterns found
- **Platform**: macOS ARM64

### Known Bottlenecks (from memory)
1. **Detection (79% of time)** - "Other" detectors slow (52.5 MB/s)
   - SSH multiline patterns
   - URI patterns
   - Regex patterns
   - Multiple detection passes

2. **Validation (44.4% of detection time)**  
   - Could reach 600-800 MB/s with SIMD
   - Current: 478 MB/s

3. **String allocations**
   - UTF-8 validation repeated
   - String cloning for substrings
   - Intermediate allocations in hot path

---

## Optimization Strategy

### Phase 1: Profile & Identify Hot Paths (30 min)
- [ ] Build with debug symbols for profiling
- [ ] Run perf/flamegraph
- [ ] Identify exact bottleneck function
- [ ] Verify detection vs redaction split

### Phase 2: Quick Wins (1-2 hours)
1. **Buffer reuse** - Avoid allocations in hot path
2. **Lazy compilation** - Cache regex patterns
3. **SIMD validation** - If detection is bottleneck
4. **Chunk size optimization** - Find optimal size

### Phase 3: Deep Optimization (2-4 hours)  
1. **Pattern prioritization** - Check common patterns first
2. **Fast-path detection** - HTTP-specific patterns
3. **Streaming optimization** - Better lookahead management
4. **Parallel processing** - If not memory-bound

### Phase 4: Integration & Testing (1 hour)
1. Test all components (detector, redactor, CLI, MITM, proxy)
2. Benchmark with real workloads
3. Verify no functionality lost

---

## Optimization Ideas (Priority Order)

### 🔴 Critical (High Impact, Low Effort)
- [ ] **Buffer pooling** (if not done) - Reuse Vec allocations
- [ ] **Chunk size tuning** - Test 32KB, 128KB, 256KB
- [ ] **Early exit patterns** - Skip irrelevant detectors

### 🟡 High (Medium Impact, Medium Effort)
- [ ] **Lazy regex compilation** - Cache compiled patterns
- [ ] **SIMD for validation** - Parallel byte scanning
- [ ] **Detect-first optimization** - Only redact if detected

### 🟢 Medium (Lower Impact, Higher Effort)
- [ ] **HTTP fast-path** - Limit patterns for HTTP streams
- [ ] **Parallel chunks** - rayon for chunk processing
- [ ] **Memory pooling** - Arena allocator

---

## Measurement Strategy

Each optimization run should measure:
1. **Primary metric**: Throughput (MB/s)
2. **Secondary metrics**: 
   - Pattern detection count (verify correctness)
   - Memory usage
   - CPU usage
   - Variance (run 3x, report min/max/avg)

Discard if:
- Throughput decreases
- Patterns found changes
- High variance across runs

---

## Success Criteria

| Target | Current | Gap | Effort |
|--------|---------|-----|--------|
| 100 MB/s | 48 MB/s | 2.1x | 1-2h |
| 150 MB/s | 48 MB/s | 3.1x | 2-4h |
| 200 MB/s | 48 MB/s | 4.2x | 4-6h |

**Realistic**: 2-3x (96-144 MB/s) in 2-3 hours
**Optimistic**: 4x (192 MB/s) in 4-6 hours

---

## No Cheating Rules

✅ DO:
- Optimize actual algorithms
- Improve data structures
- Reduce allocations
- Profile first
- Measure accurately

❌ DON'T:
- Skip patterns (breaks security)
- Use smaller test data (unfair)
- Hard-code test results
- Reduce lookahead (breaks correctness)
- Disable checks

---

