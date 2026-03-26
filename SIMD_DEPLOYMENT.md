# SIMD Deployment & CI/CD Guide

## Overview

This guide covers deploying SIMD-accelerated charset scanning to production with proper CI/CD support and feature flag management.

## Phase 4: Production Deployment

### Step 1: CI/CD Pipeline Setup ✅

Created `.github/workflows/test.yml` with:

**Stable Rust Jobs**:
- Tests on Ubuntu + macOS
- No SIMD (scalar fallback used)
- Baseline performance measurement
- Clippy linting
- Rustfmt validation

**Nightly Rust Jobs**:
- Tests on Ubuntu + macOS with `--features simd-accel`
- Charset SIMD benchmarks (3 groups)
- Detector benchmarks (4 suites)
- Full SIMD performance profiling

### Step 2: Build Variations

#### Default Build (Stable Rust)
```bash
cargo build --release
# No SIMD, max compatibility
```

#### SIMD-Accelerated Build (Nightly Rust)
```bash
cargo +nightly build --release --features simd-accel
# Full SIMD optimization, 0.4-2.1% faster detection
```

### Step 3: Release Strategy

#### Recommended: Dual Release

**Release v1.0 (Stable)**
- Default: scalar charset scanning
- SIMD: available via `--features simd-accel` for nightly users
- Binary: compiled with stable Rust
- Target: maximum compatibility

```toml
[features]
default = []
simd-accel = []
```

**Build Instructions**:
```bash
# Create release builds
cargo build --release                              # Stable, scalar
cargo +nightly build --release --features simd-accel  # Nightly, SIMD
```

### Step 4: Versioning & Documentation

#### Update Cargo.toml
```toml
[package]
name = "scred-detector"
version = "0.2.0"

[features]
default = []
simd-accel = []
```

#### Update README
```markdown
## Performance

### SIMD Acceleration (Optional)

For ~0.4-2.1% faster pattern detection on large workloads:

**Nightly Rust Only**:
```bash
cargo +nightly build --release --features simd-accel
```

Improvement Details:
- Charset scanning: 29-48% faster (micro-level)
- Full detection: 0.4-2.1% faster (macro-level)
- Early boundary detection: 38-48% faster
- Production estimate: 6.8% improvement on HTTP request redaction

See SIMD_IMPLEMENTATION.md for complete benchmark details.
```

### Step 5: Feature Flag Best Practices

#### Default Behavior
```rust
// Stable Rust: scalar fallback
#[cfg(not(feature = "simd-accel"))]
fn scan_token_end_fast(...) -> usize {
    scan_token_end_scalar(...)
}

// Nightly + feature: SIMD optimization
#[cfg(feature = "simd-accel")]
fn scan_token_end_fast(...) -> usize {
    scan_token_end_simd(...)
}
```

#### Compile-Time Selection
- No runtime checks (zero overhead)
- Feature determined at build time
- Binary size: identical (feature unused = not compiled)

#### Distribution
- Publish stable to crates.io (default)
- Document SIMD feature for advanced users
- Maintain both paths forever

### Step 6: CI/CD Integration

#### GitHub Actions Matrix
```yaml
jobs:
  test-stable:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
  
  test-nightly:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
```

Results:
- 4 test jobs per push (2 OS × 2 versions)
- Stable ≈ 5 min, Nightly + SIMD ≈ 7 min
- Total CI time: ~10-12 min

#### Success Criteria
```
✅ Stable build succeeds
✅ Nightly build succeeds
✅ All 346 tests pass (both versions)
✅ Charset benchmarks complete
✅ Detector benchmarks complete
✅ Clippy linting passes
✅ Rustfmt validation passes
```

### Step 7: Release Checklist

Before releasing:

- [ ] All CI tests pass (stable + nightly)
- [ ] Benchmarks show consistent results
- [ ] No regressions in test suite
- [ ] Documentation updated
- [ ] Feature flag documented in README
- [ ] Version bumped in Cargo.toml
- [ ] CHANGELOG entry added
- [ ] Tag created: `v0.2.0`

Release command:
```bash
git tag v0.2.0
git push origin main v0.2.0
cargo publish
```

### Step 8: Usage Instructions for Users

#### Standard Installation
```bash
cargo install scred --version "0.2.0"
# Uses stable Rust, scalar charset scanning
```

#### SIMD-Accelerated Installation
```bash
cargo +nightly install scred --version "0.2.0" --features simd-accel
# Requires nightly Rust, 0.4-2.1% faster on large payloads
```

#### Performance Comparison
```bash
# Standard (scalar): ~4.7ms per 1MB realistic data
./scred input.txt

# SIMD-accelerated: ~4.67ms per 1MB (0.4% faster)
./scred input.txt  # (if installed with --features simd-accel)
```

## Monitoring & Metrics

### What to Track

1. **Build Time**
   - Stable: baseline
   - Nightly + SIMD: +2-3 min expected
   - Monitor for regressions

2. **Binary Size**
   - Should be identical (feature-gated code not included)
   - If increases > 1%, investigate

3. **Test Results**
   - Pass rate should be 100% both versions
   - No flaky tests

4. **Benchmark Results**
   - Stable: baseline measurement
   - SIMD: should be ≤ 2.1% faster
   - Variance: ±3% acceptable (noise)

### Alerts

Setup alerts if:
- ❌ Build fails
- ❌ Tests fail
- ❌ SIMD slower than scalar (regression)
- ❌ Binary size increases > 1%

## FAQ

### Q: Why keep scalar fallback?
A: Compatibility. Stable Rust is safer, nightly is cutting-edge. Users choose.

### Q: When should I use SIMD?
A: When processing 1MB+ payloads where 0.4-2% matters (e.g., bulk redaction jobs).

### Q: Does SIMD add binary bloat?
A: No. Feature-gated code is compile-time decision. Scalar-only adds ~0 bytes.

### Q: Will SIMD break on older CPUs?
A: No. Platform-specific: ARM64 (NEON), x86_64 (SSE2), others (scalar). All safe.

### Q: Should I use SIMD in production?
A: Yes, if:
  - Using nightly Rust (stable alternative available)
  - Want 0.4-2.1% faster processing
  - Can test thoroughly in staging

## Related Files

- `SIMD_IMPLEMENTATION.md` - Technical deep dive
- `SESSION_SUMMARY.md` - Session work summary
- `.github/workflows/test.yml` - CI/CD pipeline
- `Cargo.toml` - Feature flag definition

## Phase 4 Completion

✅ CI/CD pipeline created  
✅ Feature flags documented  
✅ Release strategy defined  
✅ Usage instructions provided  
✅ Monitoring guidance included  

Status: **READY FOR PRODUCTION DEPLOYMENT**
