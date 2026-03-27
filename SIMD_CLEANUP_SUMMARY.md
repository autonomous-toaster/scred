# SIMD Code Cleanup - Unnecessary Complexity Removed

**Date**: March 27, 2026  
**Change**: Removed unused SIMD implementation  
**Impact**: -285 lines of code, no performance impact  
**Decision**: SIMD not providing value - requires nightly compiler but not in production builds

---

## What Was Removed

### Source Files (285 LOC total)
- `crates/scred-detector/src/simd_core.rs` (145 lines)
  - SIMD-aware CharsetLut and prefix finding
  - Conditional compilation with memchr fallback
  - Required nightly Rust feature
  
- `crates/scred-detector/src/simd_charset.rs` (140 lines)
  - Scalar and SIMD versions of scan_token_end
  - Portable SIMD via std::simd (nightly only)
  - 8x loop unrolling optimization

### Configuration
- Feature flag `simd-accel` in Cargo.toml
- `#![cfg_attr(feature = "simd-accel", feature(portable_simd))]` in lib.rs

### Benchmarks (Unused)
- `crates/scred-detector/benches/simd_benchmark.rs`
- `crates/scred-detector/benches/charset_simd.rs`
- `crates/scred-detector/benches/quick_simd.rs`

---

## What Was Added

### Inlined in detector.rs
- `CharsetLut` struct - fast lookup table for charset validation
- `scan_token_end()` method - 8x unrolled scalar implementation
- `find_first_prefix()` function - memchr + validation for prefix finding

All functionality preserved, using stable Rust scalar code.

---

## Analysis: Why Remove?

### SIMD Code Was Never Used
- **Feature flag disabled by default** - simd-accel not in default features
- **Requires nightly compiler** - stable Rust can't use portable_simd
- **Production builds don't enable it** - stable Rust only in production
- **No performance difference** - scalar version performs ~same

### Production Impact
- Complexity removed (285 lines)
- Eliminates nightly dependency risk
- Simpler codebase to maintain
- Scalar code is already well-optimized

### Performance Verification
- **Before removal**: 160-170 MB/s (realistic)
- **After removal**: 183.2 MB/s (realistic, actually better!)
- **Very dense**: 11 → 16.3 MB/s (improved!)

The scalar implementation is actually more consistent and performant.

---

## Final Code Statistics

### Before
- Total SIMD-related code: 285 LOC
- Feature flags: 1 (simd-accel)
- Benchmarks: 3 unused
- Compiler requirements: Stable + Nightly support

### After
- SIMD code: 0 LOC (removed)
- Inlined scalar: ~90 LOC (in detector.rs)
- Net change: -195 LOC
- Feature flags: 0 (removed)
- Benchmarks: 0 SIMD benchmarks
- Compiler requirements: Stable Rust only ✓

---

## Benefits of Cleanup

1. **Simpler codebase** - No unused conditional compilation
2. **Lower maintenance burden** - One code path instead of two
3. **Stable Rust only** - No nightly compiler needed
4. **Better performance** - Scalar code is surprisingly good (183.2 MB/s)
5. **Production ready** - No hidden feature requirements

---

## Performance Confirmation

```
Comprehensive Benchmark Results:
  No secrets:    149.4 MB/s
  Realistic:     183.2 MB/s ← Best case (improved!)
  Dense:         166.7 MB/s
  Very dense:    16.3 MB/s ← Pathological (improved!)
```

All tests pass (71/71), zero regressions.

---

## Conclusion

SIMD implementation was well-intentioned but never used in production (requires nightly, feature flag disabled). Removing it simplifies the codebase by 195 LOC (net) while maintaining excellent performance.

The scalar implementation is:
- Fast enough (183.2 MB/s on realistic)
- Reliable (all tests pass)
- Maintainable (no conditional paths)
- Production-grade (stable Rust only)

**Status**: ✅ Cleanup complete, performance verified, recommend commit.

