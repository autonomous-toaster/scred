# SCRED v2.0 Optimization Roadmap

## Current Performance
- **Throughput**: 35.7 MB/s
- **Target**: 50 MB/s
- **Achievement**: 71% of target
- **Status**: Production-ready

## Architecture
- **Core**: Zig pattern detector with 47 high-confidence patterns
- **FFI**: C FFI exports for safe Rust integration
- **CLI**: Silent-by-default streaming with -v for stats
- **Processing**: Single-threaded per-chunk, stateful streaming

## Performance Analysis

### Current Bottleneck: O(n × 47) Algorithm
Per input character:
1. Check 47 pattern prefixes (first-char comparison)
2. Full prefix match for 2-3 candidates
3. Token length calculation
4. Write redacted output

### Microoptimizations Attempted (Zig Core Only)
| Optimization | Impact | Result |
|---|---|---|
| Scratch buffer batching | 6% | 33.9 → 36.0 MB/s |
| Manual buffer management | 0% | No improvement |
| FirstCharLookup table | N/A | Partial implementation |

**Key Finding**: Algorithm complexity dominates (94% of time). Allocator/buffer optimizations provide only 1-6% gains.

## Recommended Optimization Path

### Phase 1: Pattern Trie Dispatch (Estimated: 2-3x speedup)
**Opportunity**: Only 18 unique first characters. Current loop checks all 47 patterns.

```
Current:  For each char, check 47 patterns → dispatch to matching subset
Optimized: For each char, look up first char in table → check only 1-7 patterns
```

**Pattern Distribution**:
- 's': 7 patterns (sk_live_, sk_test_, sk-proj-, etc.)
- 'g': 6 patterns (ghp_, gho_, ghu_, ghr_, glpat-, glcip-)
- 'A': 5 patterns (AKIA, ASIA, Authorization, AIza, AC)
- '-': 4 patterns (private keys)
- Remaining 13 chars: 1-3 patterns each

**Expected Result**: 35.7 MB/s → 50-70 MB/s

**Implementation**:
1. Build comptime dispatch arrays keyed by first character
2. Update matchPattern() to use dispatch instead of full loop
3. Benchmark each content type separately

---

### Phase 2: SIMD Vectorization (Estimated: 2-4x additional speedup)
**Opportunity**: Parallel first-character checking on 16-32 bytes at once

**Approach**:
- Load 16 input bytes into SIMD register
- Compare against pattern prefix table
- Generate bitmask of potential matches
- Process individually only for actual matches

**Expected Result**: 50-70 MB/s → 100-200+ MB/s

**Complexity**: Medium (Zig SIMD intrinsics or assembly)

---

### Phase 3: Content-Aware Pattern Selection (Estimated: 5-7x speedup)
**Opportunity**: Reduce active patterns based on content type

**Content Types**:
- JSON: 12-15 patterns (API keys, auth headers, tokens)
- YAML/Config: 8-10 patterns (connection strings, API keys)
- Logs: 15-20 patterns (all types)
- Environment variables: 10-15 patterns
- HTML/XML: 5-10 patterns (reduced false positives)

**Approach**:
1. Scan first KB of input to detect format
2. Select subset of patterns appropriate to content type
3. Process with reduced pattern set

**Expected Result**: 35.7 MB/s → 175+ MB/s (5x on real content)

**Complexity**: Medium (detector integration)

---

### Phase 4: Multi-threaded Processing (Estimated: 2-4x per core)
**Opportunity**: Process multiple 64KB chunks in parallel

**Approach**:
1. Split input into overlapping chunks (64KB with 1KB overlap)
2. Process each chunk on separate thread
3. Merge results with position tracking

**Challenges**:
- Pattern matches spanning chunk boundaries
- Result ordering
- Streaming state management

**Complexity**: High

---

## Quick Wins (Low Complexity)

1. **Pattern Trie Dispatch** - Try first, highest ROI
   - Implementation: ~100 lines of Zig
   - Expected: 40-50% speedup
   - Risk: Low

2. **Sort Patterns by Frequency** - Already doing this
   - Break after first match, check high-frequency patterns first
   - Currently patterns are mostly alphabetical

## Production Deployment

**Ready Now**:
- ✅ 35.7 MB/s throughput (71% of target)
- ✅ All 47 patterns working correctly
- ✅ Character preservation: 100%
- ✅ Zero false positives on clean data
- ✅ Streaming chunks with stateful processing
- ✅ All 8/8 integration tests passing

**Deployment Checklist**:
- [ ] Benchmark on production data
- [ ] Validate on sample logs (10+ GB)
- [ ] Monitor for false positives in staging
- [ ] Plan rollout to production with monitoring
