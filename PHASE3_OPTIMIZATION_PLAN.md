# Phase 3: Optimization & Scaling Plan

**Goal**: Achieve 65-75 MB/s throughput with optimized pattern matching  
**Current Baseline**: ~35-40 MB/s  
**Target Improvement**: 2-4x faster  
**Constraint**: AGENT.md Rule 5 - SIMD as first-class citizen  

---

## Phase 3 Breakdown

### Phase 3a: Benchmarking & Profiling (1 hour)
**Goal**: Establish baseline and identify bottlenecks

**Tasks**:
1. Create throughput benchmark (50 MB test file)
2. Measure pattern detection time
3. Measure redaction time
4. Profile with `perf` or `flamegraph`
5. Identify hottest functions
6. Document baseline metrics

**Deliverable**: `PHASE3_BENCHMARK_BASELINE.md` + benchmark binary

**Success Criteria**:
- Baseline: 35-40 MB/s confirmed
- Hottest function identified
- Overhead quantified

---

### Phase 3b: SIMD Aggressive Integration (2 hours)
**Goal**: Optimize pattern matching with SIMD

**Current State**:
- simd_wrapper.zig exists but conservative
- Uses std.mem.indexOf() fallback
- Not true SIMD parallel matching

**Tasks**:
1. Review simd_match.zig (`findFirstCharMatches`, `scanForTokenEnd`)
2. Create aggressive wrapper that uses SIMD
3. Batch character comparisons (process 16 bytes parallel)
4. Add benchmarks: scalar vs SIMD
5. Measure throughput improvement

**New Functions Needed**:
```zig
// Batch scan for prefix first characters
pub fn batchFindFirstChar(text: []const u8, first_chars: []const u8) []usize
// SIMD-optimized token length calculation
pub fn simdScanTokenLength(text: []const u8, start: usize, charset: Charset) usize
```

**Expected Impact**: 2-4x faster prefix matching

**Deliverable**: Updated simd_wrapper.zig + benchmarks

**Success Criteria**:
- Throughput: 60-80 MB/s
- SIMD reduces pattern matching by 50%+
- No regressions in test suite

---

### Phase 3c: Pattern Trie Implementation (2-3 hours)
**Goal**: Reduce O(n*p) to O(n + matches) complexity

**Current State**:
- pattern_trie.zig exists but unused
- Sequential prefix search for 96 patterns
- Bottleneck: Check every pattern on every text chunk

**Tasks**:
1. Review/fix pattern_trie.zig implementation
2. Build trie from pattern prefixes
3. Integrate into find_all_matches()
4. Replace sequential search with trie lookup
5. Benchmark: linear search vs trie
6. Measure throughput improvement

**Expected Impact**: 
- Pattern detection: O(n*p) → O(n + matches)
- Matches typically << patterns
- ~50% reduction in pattern checks

**Deliverable**: Integrated pattern_trie + tests

**Success Criteria**:
- Throughput: 70-90 MB/s
- Pattern checks reduced to only relevant patterns
- All tests passing

---

### Phase 3d: REGEX Pattern Decomposition (2-3 hours)
**Goal**: Enable 220 REGEX patterns by decomposing to PREFIX_VALIDATION

**Current State**:
- 220 REGEX patterns defined but unused
- PREFIX_VALIDATION infrastructure complete
- Need to identify which patterns can decompose

**Tasks**:
1. Analyze REGEX_PATTERNS array
2. Identify patterns with simple structure:
   - Prefix-based (starts with specific string)
   - Fixed format (consistent character set)
   - Length constraints possible
3. Decompose to PREFIX_VALIDATION format:
   - Extract prefix
   - Define charset (hex, base64, alphanumeric, etc.)
   - Set min/max length
4. Import into redaction_impl
5. Test decomposed patterns
6. Measure coverage increase

**Expected Patterns to Decompose**: ~60-100 patterns (25-45% of REGEX)

**Deliverable**: Decomposed patterns + updated patterns.zig

**Success Criteria**:
- 150+ patterns total active (96 + decomposed)
- Coverage increase visible in test results
- No regressions

---

### Phase 3e: Advanced SIMD - Batch Redaction (1-2 hours)
**Goal**: Batch redaction operations using SIMD

**Tasks**:
1. Create batch redaction function
2. Process multiple matches in parallel
3. Use SIMD for character replacement (x → x repeated)
4. Handle overlapping matches
5. Benchmark: single vs batch

**Expected Impact**: 10-20% faster redaction

**Deliverable**: Batch redaction function + benchmarks

**Success Criteria**:
- Throughput: 75-100 MB/s
- Redaction 15%+ faster

---

### Phase 3f: Full Pattern Integration (1-2 hours)
**Goal**: Enable all decomposable patterns

**Tasks**:
1. Extend patterns.zig with decomposed patterns
2. Update PREFIX_VALIDATION_PATTERNS array
3. Test all new patterns
4. Document pattern coverage
5. Create pattern reference guide

**Expected Coverage**: 150-200 patterns active

**Deliverable**: Complete pattern library + documentation

**Success Criteria**:
- 150+ patterns active
- All tests passing
- Clear pattern reference

---

## Performance Roadmap

| Phase | Task | Current | Target | Improvement |
|-------|------|---------|--------|-------------|
| Baseline | Benchmark | - | 35-40 MB/s | - |
| 3a | Profiling | - | - | Identify hotspot |
| 3b | SIMD Aggressive | 35-40 | 60-80 | 2-4x |
| 3c | Pattern Trie | 60-80 | 70-90 | 1.2-1.5x |
| 3d | REGEX Decompose | 70-90 | 75-100 | 1.1-1.4x |
| 3e | Batch Redaction | 75-100 | 80-110 | 1.1x |
| 3f | Full Integration | 80-110 | 85-115 | 1.1x |

**Total Improvement**: 2.1-3.3x (35-40 → 85-115 MB/s)

---

## Implementation Order

### Week 1 (This week)
1. ✅ Phase 2: Validation + SIMD integration (DONE)
2. Phase 3a: Benchmarking (1 hour)
3. Phase 3b: SIMD aggressive (2 hours)
4. Phase 3c: Pattern trie (2-3 hours)

### Week 2
5. Phase 3d: REGEX decomposition (2-3 hours)
6. Phase 3e: Batch redaction (1-2 hours)
7. Phase 3f: Full integration (1-2 hours)

### Week 3+
- Stress testing (1000 MB files)
- Production deployment
- Monitoring and metrics

---

## Critical Path (Minimum for 65-75 MB/s)

**Must Do**:
1. Phase 3a: Benchmarking (need baseline)
2. Phase 3b: SIMD aggressive (2-4x improvement needed)
3. Phase 3c: Pattern trie (reduces pattern checks)

**Should Do**:
4. Phase 3d: REGEX decomposition (more patterns, 150+)

**Nice to Do**:
5. Phase 3e: Batch redaction (polish)
6. Phase 3f: Full integration (complete picture)

---

## Success Criteria

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Throughput | 35-40 MB/s | 65-75 MB/s | ⏳ |
| Patterns | 96 active | 150+ active | ⏳ |
| Tests | 29/29 passing | 100% passing | ✅ |
| SIMD | Integrated | First-class | ⏳ |
| Architecture | Solid | Production-ready | ⏳ |
| Regressions | 0 | 0 | ✅ |

---

## Tools & Techniques

### Benchmarking
- `cargo bench` - Built-in Rust benchmarking
- `flamegraph` - CPU profiling visualization
- `perf` - Linux performance analysis
- Custom benchmark binary - Realistic workloads

### SIMD
- `@Vector` - Zig SIMD vectors
- `simd_match.zig` - Existing implementations
- 16-byte batches - Process 16 bytes parallel

### Pattern Trie
- `pattern_trie.zig` - Existing implementation
- Prefix tree structure
- O(n) matching instead of O(n*p)

### Decomposition
- Manual analysis of REGEX_PATTERNS
- Identify prefix + charset + length
- Convert to PREFIX_VALIDATION structure

---

## Files to Create/Modify

### New Files
- `benchmarks/throughput.rs` - Benchmark binary
- `PHASE3_BENCHMARK_BASELINE.md` - Results
- `PHASE3_SIMD_OPTIMIZATION.md` - SIMD improvements
- `PHASE3_PATTERN_TRIE.md` - Trie analysis

### Modified Files
- `simd_wrapper.zig` - Aggressive SIMD
- `redaction_impl.zig` - Trie integration
- `patterns.zig` - Decomposed REGEX patterns
- `lib.zig` - Import updates

### Documentation
- `PHASE3_COMPLETION_REPORT.md` - Final results
- Pattern reference guide
- Performance tuning guide

---

## Estimated Timeline

- Phase 3a: 1 hour
- Phase 3b: 2 hours
- Phase 3c: 2-3 hours
- Phase 3d: 2-3 hours
- Phase 3e: 1-2 hours
- Phase 3f: 1-2 hours

**Total**: 10-14 hours of focused work

**Realistic**: 2-3 days (with breaks and testing)

---

## Key Dependencies

| Task | Depends On | Status |
|------|-----------|--------|
| 3a | Foundation | ✅ |
| 3b | 3a + SIMD | ✅ |
| 3c | 3b + Trie | ✅ |
| 3d | 3c + Analysis | ✅ |
| 3e | 3d | ✅ |
| 3f | 3e | ✅ |

All tasks parallelizable after 3a!

---

## Success Definition

**Phase 3 is COMPLETE when**:
- ✅ Throughput: 65-75 MB/s (verified with benchmark)
- ✅ Patterns: 150+ active
- ✅ Tests: 100% passing
- ✅ SIMD: First-class citizen (integrated + optimized)
- ✅ Architecture: Clean + maintainable
- ✅ Documentation: Clear and complete

