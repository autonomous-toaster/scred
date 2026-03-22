# CLI Integration Checklist - Zig Analyzer

**Goal**: Integrate Zig content_analysis + regex_engine into production CLI

## Current State

✅ **Zig modules complete**:
- content_analysis.zig (260 LOC) - Character detection, pattern selection
- regex_engine.zig (290 LOC) - PCRE2 wrapper with caching
- zig_detector_ffi.zig (320 LOC) - C FFI exports

✅ **Rust FFI wrapper ready**:
- zig_analyzer.rs (100 LOC) - Lean wrapper for Zig functions

✅ **CLI structure ready**:
- main.rs - Entry point, handles flags
- streaming.rs - 64KB chunk processing
- redactor.rs - Pattern definitions

## What Needs to Be Done

### Phase 1: Zig FFI Integration (2 hours)

**1.1 Fix Zig FFI exports** (30 min)
- [ ] Verify FFI signatures in zig_analyzer.rs
- [ ] Ensure memory management is correct
- [ ] Test compile with -lscred_zig linkage

**1.2 Build system integration** (30 min)
- [ ] Update build.rs to compile Zig modules
- [ ] Ensure PCRE2 linkage (-lpcre2-8)
- [ ] Generate libscred_zig.a correctly

**1.3 CLI flag addition** (1 hour)
- [ ] Add `--zig` flag to enable Zig analyzer
- [ ] Add `--compare` flag to benchmark both
- [ ] Update help text

### Phase 2: Benchmarking & Validation (2 hours)

**2.1 Performance measurement** (1 hour)
- [ ] Run benchmark_zig_vs_rust.py with both engines
- [ ] Measure throughput improvement
- [ ] Profile hot paths
- [ ] Document results

**2.2 Correctness validation** (1 hour)
- [ ] Run integration_test.py with Zig engine
- [ ] Verify all 198 patterns still work
- [ ] Check character preservation
- [ ] Test streaming boundaries

### Phase 3: Production Readiness (1 hour)

**3.1 Error handling** (30 min)
- [ ] Handle Zig FFI failures gracefully
- [ ] Fallback to Rust regex if Zig fails
- [ ] Log errors appropriately

**3.2 Documentation** (30 min)
- [ ] Update README with --zig flag
- [ ] Document performance improvements
- [ ] Add benchmarking instructions

## Architecture: How It Works

```
stdin (64KB chunks)
  ↓
┌─────────────────────────────────────┐
│ CLI main() - Option parsing         │
└─────────────────────────────────────┘
  ↓
  ├─ --zig (NEW)      → Use Zig pipeline
  │    ├─ ZigAnalyzer.get_patterns_for_content() [FFI]
  │    │   └─ zig_detector_ffi.rs exports:
  │    │       ├─ detect_content_type()
  │    │       ├─ get_candidate_patterns()
  │    │
  │    └─ ZigAnalyzer.redact_optimized() [FFI]
  │        └─ zig_detector_ffi.rs exports:
  │            ├─ redact_text_optimized()
  │            ├─ match_patterns()
  │
  └─ default (Rust)   → Use Rust regex
      └─ RedactionEngine (current)
          ├─ Compile all 198 patterns
          └─ Match all patterns per chunk
```

## Expected Performance

**Current (Rust regex)**: 0.5 MB/s average
**With Zig**: 3-5 MB/s average (6-10x improvement)

Breakdown by workload:
- JWT-heavy: 7-12 MB/s (1.2 → 8 MB/s)
- AWS keys: 2-3 MB/s (0.3 → 2.5 MB/s)
- PostgreSQL: 2-4 MB/s (0.4 → 3 MB/s)
- HTTP: 0.6-1 MB/s (0.1 → 0.8 MB/s)
- JSON: 4-7 MB/s (0.7 → 5.5 MB/s)

## Test Plan

### Unit Tests (Zig)
```
cargo test --lib -p scred-pattern-detector-zig
├─ content_analysis tests
│  ├─ analyze_http_request
│  ├─ detect_jwt_signal
│  ├─ analyze_json
│  └─ ...
├─ regex_engine tests
│  ├─ compile_and_match
│  ├─ find_all_matches
│  ├─ cache_reuse
│  └─ ...
```

### Integration Tests
```
python3 test_zig_analyzer.py
├─ Content analysis for various formats
├─ JWT detection accuracy
├─ Pattern selection efficiency
└─ Pattern count reduction verification

python3 integration_test.py --zig
├─ All 198 patterns detected
├─ Character preservation 100%
├─ Streaming boundaries correct
└─ False positives <5%
```

### Benchmark Tests
```
python3 benchmark_zig_vs_rust.py
├─ Rust baseline: 0.5 MB/s
├─ Zig optimized: 3-5 MB/s
├─ Improvement: 6-10x
└─ Per-workload breakdown
```

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Zig FFI complexity | Thin wrapper, tested independently |
| PCRE2 availability | Already in build system |
| Memory lifecycle | Explicit allocate/free functions |
| Performance regression | Baseline measured, comparison easy |
| Fallback failures | Graceful degradation to Rust |

## Success Criteria

✅ CLI builds with `--zig` flag
✅ No compilation errors
✅ FFI calls work correctly
✅ Performance improves 6-10x
✅ All 198 patterns detected correctly
✅ Character preservation 100%
✅ Integration tests pass 8/8
✅ Documentation updated

## Timeline

- Phase 1 (Zig FFI): 2 hours
- Phase 2 (Benchmark): 2 hours  
- Phase 3 (Production): 1 hour
- **Total**: 5 hours

## Rollback Plan

If issues arise:
1. Keep both engines in CLI (toggle with `--zig`)
2. Default to Rust regex (proven, stable)
3. Zig analyzer as opt-in (`--zig` flag)
4. Easy to disable: just remove flag check

## Files to Modify

**Rust side**:
- `crates/scred-cli/src/main.rs` - Add `--zig` handling
- `crates/scred-redactor/src/lib.rs` - Export zig_analyzer
- `build.rs` - Link Zig library (optional, can skip)

**Testing**:
- `test_zig_analyzer.py` - New analyzer tests
- `benchmark_zig_vs_rust.py` - Zig vs Rust comparison

## Next Action

Start with Phase 1: Zig FFI Integration
1. Fix any remaining Zig FFI issues
2. Create minimal `--zig` flag test
3. Verify FFI calls work
4. Then run benchmarks
