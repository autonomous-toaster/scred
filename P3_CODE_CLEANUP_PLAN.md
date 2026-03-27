# P3: Code Cleanup Plan - Detailed Implementation

**Date**: March 27, 2026  
**Status**: Ready to Implement  
**Estimated Effort**: 4-5 hours  
**Priority**: High (after P1 + P2 complete)

---

## Overview

P3 focuses on organizing code structure without changing functionality:
1. Consolidate benchmark files
2. Move tests out of source files
3. Move bin/ executables to examples/
4. Improve test error handling

**Zero functional changes** - pure organization and error handling improvements.

---

## Task 1: Consolidate Benchmark Files (1 hour)

### Current State
Multiple benchmark files scattered across the workspace:

```
crates/scred-redactor/benches/
  ├── phase1_benchmark.rs (legacy, might be duplicate)
  ├── streaming_benchmark.rs (main benchmark)
  └── ... other files

crates/scred-detector/benches/
  ├── various benchmark files

crates/scred-video-optimization/benches/
  ├── frame_ring_comparison.rs (might be redundant)
```

### Consolidation Target

**Goal**: Reduce from 9+ files to 2-3 focused benchmark suites:

1. **benches/throughput.rs** - Main streaming performance
   - Combines: streaming_benchmark.rs + phase1_benchmark.rs
   - Tests: 149-154 MB/s baseline
   - Patterns: Detection + redaction

2. **benches/components.rs** - Component-level profiling
   - Combines: detection, redaction, specific pattern types
   - Tests: Detection speed, redaction speed, validation

3. **benches/optional_video.rs** - Video/FrameRing (if separate)
   - Combines: frame_ring_comparison.rs
   - Tests: Character preservation, FrameRing efficiency

### Steps

1. **Identify benchmark sources** (10 min)
   ```bash
   find crates -name "*.rs" -path "*/benches/*" | xargs wc -l
   find crates -name "*.rs" -path "*/src/bin/*" | xargs wc -l
   ```

2. **Merge similar benchmarks** (30 min)
   - Extract common patterns from each
   - Consolidate into focused suites
   - Remove redundancy

3. **Update Cargo.toml** (15 min)
   - Remove old bench entries
   - Add new consolidated ones
   - Verify `cargo bench` works

4. **Test execution** (5 min)
   ```bash
   cargo bench --release
   ```

### Success Criteria
- ✅ 9+ files → 2-3 files
- ✅ No performance regression
- ✅ All benchmarks runnable
- ✅ Coverage of key metrics maintained

---

## Task 2: Move Tests from Source to tests/ (2-3 hours)

### Current State

Tests are embedded in source files:

```
crates/scred-detector/src/
  ├── detector.rs           (contains ~40 tests)
  ├── patterns.rs           (contains ~20 tests)
  ├── simd_charset.rs       (contains ~10 tests)
  └── simd_core.rs          (contains ~15 tests)

crates/scred-redactor/src/
  ├── streaming.rs          (contains ~20 tests)
  └── ... other files with tests
```

### Target Structure

```
crates/scred-detector/tests/
  ├── detector_test.rs      (extracted from detector.rs)
  ├── patterns_test.rs      (extracted from patterns.rs)
  ├── simd_test.rs          (extracted from simd_charset/core)
  └── integration_test.rs   (cross-module tests)

crates/scred-redactor/tests/
  ├── streaming_test.rs     (extracted from streaming.rs)
  ├── redaction_test.rs     (extracted from redactor.rs)
  └── integration_test.rs
```

### Steps

1. **Identify all in-source tests** (20 min)
   ```bash
   grep -r "#\[test\]" crates/*/src --include="*.rs" | wc -l
   grep -r "#\[tokio::test\]" crates/*/src --include="*.rs" | wc -l
   ```

2. **For each test-containing file** (2+ hours):
   a. Extract test module to separate file
   b. Update imports in test file
   c. Remove test module from source
   d. Verify `cargo test --lib` still passes
   e. Commit incrementally

3. **Verify test organization** (15 min)
   ```bash
   cargo test --release
   # Should show all 368+ tests
   ```

### Example Migration

**Before** (detector.rs has 500+ lines with tests at bottom):
```rust
pub struct Detector { ... }

impl Detector {
    pub fn detect(&self, text: &str) -> Vec<Match> { ... }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_aws_key_detection() { ... }
    
    #[test]
    fn test_github_pat() { ... }
    
    // ... 40+ tests
}
```

**After** (detector.rs contains only code):
```rust
pub struct Detector { ... }

impl Detector {
    pub fn detect(&self, text: &str) -> Vec<Match> { ... }
}
```

**New file** (tests/detector_test.rs):
```rust
use scred_detector::*;

#[test]
fn test_aws_key_detection() { ... }

#[test]
fn test_github_pat() { ... }

// ... 40+ tests
```

### Benefits
- ✅ Source files more readable (~30% size reduction)
- ✅ Tests organized by module
- ✅ Better IDE navigation
- ✅ Cleaner compilation during development

### Success Criteria
- ✅ All tests moved to tests/ directory
- ✅ All 368+ tests still passing
- ✅ Zero regressions
- ✅ Source files cleaner

---

## Task 3: Move Bin Executables to Examples (15 minutes)

### Current State
```
crates/scred-detector/src/bin/
  ├── validate_debug.rs (diagnostic tool)

crates/scred-redactor/src/bin/
  ├── profile_components.rs (profiling)
  ├── ... other diagnostic tools
```

### Target Structure
```
crates/scred-detector/examples/
  ├── validate_debug.rs

crates/scred-redactor/examples/
  ├── profile_components.rs
  ├── ... other tools
```

### Steps

1. **Move files** (5 min)
   ```bash
   mkdir -p crates/scred-detector/examples
   mv crates/scred-detector/src/bin/validate_debug.rs crates/scred-detector/examples/
   # Repeat for each crate
   ```

2. **Update Cargo.toml** (5 min)
   ```toml
   # Before
   [[bin]]
   name = "validate_debug"
   path = "src/bin/validate_debug.rs"
   
   # After (if needed - may be automatic)
   # Usually examples/ are discovered automatically
   ```

3. **Test** (5 min)
   ```bash
   cargo run --example validate_debug --release
   ```

### Benefits
- ✅ Clear separation: examples vs internal tools
- ✅ Cleaner src/ structure
- ✅ Better discoverability for users

### Success Criteria
- ✅ All bins moved to examples/
- ✅ All examples still runnable
- ✅ Cargo.toml updated

---

## Task 4: Improve Test Error Handling (30 minutes)

### Current Issues

Tests use `.unwrap()` liberally:

```rust
#[test]
fn test_config_loading() {
    let temp_dir = TempDir::new().unwrap();  // ❌ Panics on error
    let config_path = temp_dir.path().join("config.yaml");
    fs::write(&config_path, yaml_content).unwrap();  // ❌ Panics
    let config = Config::load(&config_path).unwrap();  // ❌ Panics
    assert_eq!(config.enabled, true);
}
```

**Problem**: If any `.unwrap()` fails, test panics with minimal error info.

### Target

Use proper error handling with helpful messages:

```rust
#[test]
fn test_config_loading() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;  // ✅ Propagates error with context
    let config_path = temp_dir.path().join("config.yaml");
    fs::write(&config_path, yaml_content)?;  // ✅ Same
    let config = Config::load(&config_path)?;  // ✅ Same
    assert_eq!(config.enabled, true);
    Ok(())
}
```

### Steps

1. **Find all test `.unwrap()` calls** (10 min)
   ```bash
   grep -r "\.unwrap()" crates/*/tests --include="*.rs" | wc -l
   ```

2. **Convert test signatures** (10 min)
   - Change: `fn test_name()` → `fn test_name() -> Result<(), Box<dyn std::error::Error>>`
   - Change: `unwrap()` → `?`
   - Add: `Ok(())` at end

3. **Test thoroughly** (10 min)
   ```bash
   cargo test --release
   ```

### Success Criteria
- ✅ All `.unwrap()` replaced with `?` in tests
- ✅ All tests still passing
- ✅ Better error messages on failure

---

## Task 5: Optional - Clarify Async/Sync Patterns (1 hour, Optional)

### Current Issues

Some tests mix sync and async code:

```rust
#[tokio::test]
async fn test_async_redaction() {
    // This mixes async/sync - clarity could be better
    let detector = Detector::new();  // Sync
    let redactor = redactor.async_redact(text).await;  // Async
}
```

### Target

Clear comments explaining when/why mixing happens:

```rust
#[tokio::test]
async fn test_async_redaction() {
    // Detector is sync - no async needed for pattern matching
    let detector = Detector::new();
    
    // Redactor may be async for I/O operations
    let redactor = redactor.async_redact(text).await;
}
```

### Impact
- ✅ Reduces confusion when reading tests
- ✅ Documents architectural decisions
- ⚠️ Optional - only if time permits

---

## Implementation Order (Recommended)

1. **Task 2: Move Tests** (2-3 hours) - First, biggest impact
   - Largest refactoring
   - Most beneficial for code organization
   - Lower risk (tests in isolation still work)

2. **Task 1: Consolidate Benchmarks** (1 hour) - Then, cleaner repo
   - Medium effort
   - Nice-to-have cleanup
   - Reduces benchmark maintenance

3. **Task 3: Move Bins to Examples** (15 min) - Quick win
   - Very fast
   - Low risk
   - Improves clarity

4. **Task 4: Test Error Handling** (30 min) - Final polish
   - Quick refactoring
   - Immediate benefit
   - All tests still pass

5. **Task 5: Async/Sync Clarity** (1 hour, optional) - Only if time remains

**Total Conservative Estimate**: 4 hours (no Task 5)  
**Total With Task 5**: 5 hours

---

## Testing Strategy

After each major task:

```bash
# Run all tests
cargo test --release

# Check test count hasn't decreased
cargo test --release 2>&1 | grep "test result"

# Benchmark to verify no regression
cargo bench --release
```

---

## Commit Strategy

**Granular commits** (easier to review, easier to revert if needed):

```
commit: "refactor(tests): Move detector tests to tests/ directory"
commit: "refactor(tests): Move redactor tests to tests/ directory"
commit: "refactor(tests): Move streaming tests to tests/ directory"
commit: "refactor(benches): Consolidate benchmark files"
commit: "refactor(examples): Move bin/ executables to examples/"
commit: "refactor(tests): Improve test error handling with Result"
```

---

## Risk Assessment

| Task | Risk | Mitigation |
|------|------|-----------|
| Move tests | Low | Test before commit, incremental moves |
| Consolidate benches | Low | Run benchmarks after merge |
| Move bins | Very Low | Verify examples/ directory works |
| Error handling | Very Low | Just syntax changes |
| Async clarity | None | Documentation only |

**Overall Risk**: Very Low
**Rollback Plan**: Revert commits if issues found

---

## Success Metrics

After P3 completion:

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Source file avg size | 400+ lines | 250 lines | < 300 |
| Test organization | Scattered | Centralized | Organized |
| Benchmark files | 9+ | 2-3 | Consolidated |
| Test readability | `.unwrap()` | `Result` | Clear errors |
| Overall LOC in src/ | Reduced | - | Cleaner |

---

## Timeline Estimate

| Task | Est. Time | Actual | Status |
|------|-----------|--------|--------|
| Task 2: Move tests | 2-3 hrs | TBD | Not started |
| Task 1: Benches | 1 hr | TBD | Not started |
| Task 3: Bins | 15 min | TBD | Not started |
| Task 4: Error handling | 30 min | TBD | Not started |
| Task 5: Async (opt) | 1 hr | TBD | Not started |
| **TOTAL** | **4-5 hrs** | TBD | Planning |

---

## Next Steps

1. Review this plan
2. Start with Task 2 (Move tests)
3. Commit after each major task
4. Verify all tests passing
5. Create final P3 summary

