# SCRED Autoresearch Rules & Framework

## Optimization Goal

**Primary Metric**: `detect_ms` (detection latency in milliseconds)
**Direction**: Lower is better
**Target**: Maximize throughput and speed of pattern detection

## Current Status

**Sessions Completed**: 12
**Current Performance**: 2.31-2.39ms (97% improvement from ~60ms baseline)
**Status**: Optimization ceiling reached, production-ready
**Tests**: 26/26 passing (100%)

## Baseline Performance

- **Original (Session 0)**: ~60ms per 1MB detection
- **After Session 1** (SIMD + parallel): 9.8ms (95% improvement)
- **After Session 2** (micro-opts): 2.43ms (96% total)
- **After Session 8** (threshold tuning): 2.77ms (97% total, Session 8 +31% gain)
- **Current** (Sessions 11-12 validation): 2.31-2.39ms (stable, within noise)

## Optimization Constraints

### Must Maintain
1. **100% correctness**: All secrets must be detected
2. **0% false positives**: No incorrect matches
3. **Character preservation**: Exact byte positions
4. **No regex**: Production requirement
5. **Backward compatibility**: No API changes
6. **Safe Rust**: No unsafe code

### Avoid
1. **Overfitting to benchmarks**: Validate on multiple workloads
2. **Cheating**: Don't sacrifice correctness for speed
3. **Premature saturation claims**: Systematic exploration before declaring limits
4. **Complexity without benefit**: KISS principle

## Performance Breakdown (Current 2.31ms)

```
memchr searching:       1.4ms (60%)  ← System SIMD (glibc), cannot be beaten
charset validation:    280µs (12%)  ← 8× unrolled, early-exit required
rayon overhead:        150µs (6%)   ← 6.5× speedup on 8 cores
JWT + simple_prefix:   400µs (17%)  ← Parallelized/sequential hybrid
merge/sync:             91µs (4%)   ← rayon reduce optimized
────────────────────────────────────
Total:                2.31ms
```

## What Has Been Explored

### ✅ Implemented & Productive
| Session | Technique | Gain | Status |
|---------|-----------|------|--------|
| 1 | SIMD charset (8× unroll) | 46% | Production |
| 1 | Rayon parallelization | 71% | Production |
| 2 | First-byte filtering | 6% | Production |
| 2 | rayon reduce merge | 5% | Production |
| 2 | OnceLock caching | 3% | Production |
| 3 | Parallel first-byte filter | 9% | Production |
| **8** | **Validation threshold 4096** | **31%** | **Production** |

### ✅ Infrastructure (Non-Integrated, Reference)
| Module | LOC | Reason |
|--------|-----|--------|
| SIMD pattern matching | 350+ | Sequential 17% slower than rayon |
| Pattern Trie | 140 | O(n×m) slower than O(n) memchr |
| Std::simd memchr | 100 | Nightly required, not faster |
| SIMD multi-pattern | 200 | Sequential beats parallelization |

### ❌ Tested & Rejected
- Pattern Trie (O(n×m) overhead)
- Std::simd memchr (nightly required, not faster)
- SIMD detection (sequential < parallel rayon)
- Adaptive thresholds (no improvement over hardcoded)
- Higher thresholds (8192: regression)
- Lower thresholds (<3072: regression)

## Why Further Optimization is Hard

### 1. memchr Bottleneck (1.4ms = 60%)
- System-level SIMD applied (glibc AVX2/SSE)
- Decades of compiler & library tuning
- **Conclusion**: Cannot be beaten with portable code
- Would require: Different algorithm (GPU, FSM, etc.)

### 2. Charset Validation (280µs = 12%)
- Already 8× loop unrolled
- Cannot parallelize (early-exit on first invalid byte)
- **Conclusion**: Near-theoretical limit for sequential validation

### 3. Parallelization (150µs = 6%)
- Already achieving 6.5× speedup on 8 cores (near-linear)
- rayon reduce already optimal for merging
- **Conclusion**: Further gains require more cores

### 4. First-Byte Filtering (400µs = 17%)
- Already filters 220 → ~50 relevant patterns (77% reduction)
- Cannot improve further without losing correctness
- **Conclusion**: Maximal for this pattern set

## Validated Not Helpful

| Approach | Session | Result |
|----------|---------|--------|
| CPU-adaptive thresholds | 12 | No benefit over hardcoded 4096 |
| Higher thresholds (8192) | 8-9 | Regression (3.06ms) |
| Lower thresholds | 11 | Regression |
| Sequential memchr | 9 | Regression (3.00ms) |
| Bitset charset scanning | — | Slower than bool[256] |

## When To Continue Optimization

Optimization should continue if:
1. **New architectural idea** not yet explored
2. **Different workload profile** suggests new bottleneck
3. **Requirements change** (different pattern set, stricter latency SLA)
4. **Hardware improves** (more cores, different CPU family)

Current system is optimal for:
- Standard AWS/GCP/Azure pattern detection
- 8-core production machines
- 97% improvement vs baseline
- Production SLAs under 5ms

## Autoresearch Session Framework

### Before Each Session
1. Read autoresearch.md (this file)
2. Check autoresearch.ideas.md for unexplored paths
3. Review autoresearch.jsonl for historical performance
4. Verify all tests pass: `cargo test --lib`

### Running Experiments
1. Make code changes
2. Run: `cargo bench --bench realistic` (main benchmark)
3. Use `run_experiment` + `log_experiment` for timing
4. Record ASI (Actionable Side Information) with results

### After Session Completes
1. Document findings (SESSION*.md)
2. Update autoresearch.ideas.md (keep/prune ideas)
3. Update autoresearch.jsonl (automatically via log_experiment)
4. Commit with meaningful message

### Quality Checks
- ✅ All 26 tests pass
- ✅ No regressions on secondary benchmarks
- ✅ No overfitting (test on no_secrets, many_matches, mixed_realistic)
- ✅ Correctness verified (100% secrets detected, 0% false positives)

## Recommendation

**✅ DEPLOY AT 2.31ms**

- Exceeds all reasonable SLAs
- 97% improvement from baseline
- 100% test pass rate
- Production-quality code
- Further optimization has **negative ROI**

Further work only justified if:
- Requirement changes to <2.0ms (unlikely)
- New algorithmic insight discovered
- Hardware changes significantly

---

**Last Updated**: Session 12 (2026-03-27)
**Status**: OPTIMIZATION COMPLETE
**Next Action**: Consider for production deployment
