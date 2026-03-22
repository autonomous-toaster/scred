# Phase 4: Performance Analysis & Optimization Strategy

**Baseline Measurements** (10MB test files):
- **No secrets (pass-through)**: 27.9 MB/s ✅ Good
- **All secrets (matching)**: 0.9 MB/s ⚠️ Needs 10-50x improvement
- **Mixed (10% secrets)**: ~5-10 MB/s (varies with density)

## Root Cause Analysis

The current system has a critical performance bottleneck:

```
For each chunk:
  ├─ Tier 1 prefix matching: ~160+ MB/s (fast)
  └─ Tier 2/3 regex matching: ~0.9 MB/s (slow)
      ├─ Compile regex for each pattern: HIGH OVERHEAD
      ├─ Run regex.is_match(): Medium overhead
      ├─ Iterate captures_iter(): Medium overhead
      └─ Overlap detection: Low overhead
```

**Problem**: We compile ALL 198 regex patterns per chunk, even if only 5-10 could possibly match

## Optimization Strategy

### Phase 4.1: Implement Lazy Pattern Selection (CURRENT)
```
For each chunk:
  1. Analyze content (colons, slashes, hyphens, etc)
  2. Select only 10-20 candidate patterns (instead of 198)
  3. Only compile those patterns
  4. Estimated improvement: 10-15x reduction in compile time
  5. Expected throughput: 9-14 MB/s for secret-heavy workloads
```

**Status**: ✅ Architecture ready
- pattern_index.rs: Analyze content → applicable patterns
- redactor_optimized.rs: Lazy compilation engine
- Foundation complete, integration pending

### Phase 4.2: Aggressive Caching
```
Cache strategy:
  1. Pattern regex cache (already using lazy_static)
  2. Per-chunk pattern selection cache
  3. Pre-compile common patterns on startup
  
Expected improvement: 1-1.5x (caching already in place)
```

### Phase 4.3: Regex Optimization
```
For hot patterns (those matching most):
  1. Measure which patterns match most frequently
  2. Optimize their regex (simplify, remove backtracking)
  3. Move high-confidence patterns to Tier 1
  
Example: If 60% of matches are AWS keys:
  - Fast path already handles AWS AKIA-, ASIA-, etc
  - Other tokens in same pattern need regex optimization
```

## Implementation Roadmap

### Immediate (1-2 hours)
- [x] Create OptimizedRedactionEngine with lazy selection
- [x] Create pattern_index for content analysis
- [ ] Integrate into CLI (--optimized flag)
- [ ] Benchmark improvement

### Short-term (2-3 hours)
- [ ] Profile actual pattern match distribution
- [ ] Optimize top 10 patterns
- [ ] Measure cumulative improvement
- [ ] Target: 10-15 MB/s for secret-heavy workloads

### Medium-term (3-4 hours)
- [ ] Implement pattern pre-compilation
- [ ] Cache pattern selections per content type
- [ ] Real-world benchmarking (GB-scale logs)
- [ ] Final tuning

## Success Criteria

For 50% secret-density workload (realistic log scenario):
- Original: ~5 MB/s
- Phase 4.1 (lazy selection): 7-10 MB/s ✓ (achievable)
- Phase 4.2 (caching): 8-12 MB/s ✓ (achievable)
- Phase 4.3 (regex optimization): 12-20 MB/s ✓ (achievable)

**Final Target**: 15+ MB/s for workloads with 50% secret density
(Realistic pass-through + secret handling average)

## Trade-offs

### Current System
- ✅ Comprehensive (tests all 198 patterns)
- ❌ Slow (1 MB/s on secrets)
- ❌ Simple but inefficient

### Optimized System
- ✅ Fast (15+ MB/s target)
- ✅ Smart pattern selection
- ⚠️ Slightly more complex
- ⚠️ Requires content analysis

## Architecture Decision

We chose **content-aware lazy selection** over:
1. ❌ Pre-compile all patterns: Still slow (compile overhead)
2. ❌ Use single pattern: Miss 90% of secrets
3. ❌ Use Zig FFI: Too risky, may not help

**Why content-aware?**
- postgres://... → Only test connection string patterns
- AKIAIOSFODNN7EXAMPLE → Only test AWS + general token patterns
- Normal logs → Only test generic patterns
- Typically reduces pattern count to test by 80-90%

## Next Steps

1. **Integrate OptimizedRedactionEngine into CLI**
   - Create --optimized flag
   - Support both original and optimized modes
   - Benchmark side-by-side

2. **Profile Pattern Matching**
   - Which patterns match most frequently?
   - Which patterns take longest to compile?
   - Build optimization priority list

3. **Incremental Optimization**
   - Start with lazy selection (10x compile reduction)
   - Measure improvement
   - Add more optimizations if needed

---

**Key Insight**: The bottleneck isn't pattern matching itself, it's compiling 188 regexes per chunk when only 10-20 could match. Lazy selection fixes this.
