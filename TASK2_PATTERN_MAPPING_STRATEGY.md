# TASK 2: Pattern Mapping & Validation - Comprehensive Test Strategy

## Executive Summary

**Objective**: Map and validate all 270 patterns using Task 1 decomposition findings

**Strategy**: Test patterns by category (A/B/C/D) with synthetic examples

**Duration**: 2 hours (15+30+45+20 min phases)

**Deliverable**: 270 test cases + validation report

---

## Phase 1: Pattern Classification Verification (15 minutes)

Verify Task 1's decomposition is correct by analyzing actual pattern definitions.

### Task 1 Decomposition Mapping

```
Category A: PREFIX + FIXED_LENGTH
├─ Pattern structure: PREFIX + exact_length chars
├─ FFI path: match_prefix() + validate_charset() + length_check
├─ Examples:
│  ├─ aio_[a-zA-Z0-9]{28}              → PREFIX="aio_" + 28 alphanumeric
│  ├─ CFPAT-[a-zA-Z0-9_-]{43}          → PREFIX="CFPAT-" + 43 alphanumeric_hyphen
│  └─ AIzaSy[A-Za-z0-9_-]{33}          → PREFIX="AIzaSy" + 33 alphanumeric_underscore
├─ Count: 80+ patterns
└─ Performance: 0.1 ms/MB (13x faster than regex)

Category B: PREFIX + MIN_LENGTH
├─ Pattern structure: PREFIX + variable chars (min_length specified)
├─ FFI path: match_prefix() + validate_charset() + length_check (min only)
├─ Examples:
│  ├─ ghp_[0-9a-zA-Z]{36,}             → PREFIX="ghp_" + min 36 alphanumeric
│  ├─ [rs]k_live_[a-zA-Z0-9]{20,247}   → PREFIX="[rs]k_live_" + 20-247 alphanumeric
│  └─ sk-(?:proj-)?[a-zA-Z0-9_-]{20,}  → PREFIX="sk-" + optional "proj-" + min 20
├─ Count: 20+ patterns
└─ Performance: 0.1 ms/MB (13x faster than regex)

Category C: MULTI-PREFIX (ALTERNATION)
├─ Pattern structure: (pre1|pre2|pre3)[charset]{length}
├─ FFI path: Split into N separate PREFIX patterns
├─ Examples:
│  ├─ (dop|doo|dor)_v1_[a-f0-9]{64}    → 3 patterns: dop_, doo_, dor_
│  ├─ (hf_|api_org_)[a-zA-Z0-9]{34}    → 2 patterns: hf_, api_org_
│  └─ dd[pw]_[a-zA-Z0-9]{36}            → 2 patterns: ddp_, ddw_
├─ Count: 15 patterns → 40+ when split
└─ Performance: 0.1 ms/MB each (13x faster than regex)

Category D: TRULY COMPLEX
├─ Pattern structure: Lookahead, captures, complex URLs
├─ FFI path: match_regex() only
├─ Examples:
│  ├─ (?i)Authorization:\s*(?:Bearer|...) → Lookahead required
│  ├─ (?P<user>...):(?P<pass>...)         → Named captures required
│  └─ mongodb+srv://(?P<user>...)@...     → Complex URL parsing
├─ Count: 25-40 patterns (keep as regex)
└─ Performance: 1.3 ms/MB (necessary complexity)
```

### Verification Checklist

- [ ] Extract all 270 patterns from patterns.zig
- [ ] Count patterns by structure type
- [ ] Verify Category A: 80+ patterns
- [ ] Verify Category B: 20+ patterns
- [ ] Verify Category C: 15 patterns (40+ when split)
- [ ] Verify Category D: 25-40 patterns
- [ ] Total: ~269-270 patterns accounted for
- [ ] Map each pattern to FFI function path

---

## Phase 2: Test Case Generation (30 minutes)

Create synthetic test examples for each pattern category.

### Template: Category A (PREFIX + FIXED_LENGTH)

```rust
#[test]
fn test_pattern_{name}() {
    // Pattern metadata
    let pattern = PatternDefinition {
        name: "{name}",
        prefix: "{prefix}",
        charset: CharsetType::{charset},
        min_len: {exact_len},
        max_len: {exact_len},  // Same as min for fixed-length
        tier: PatternTier::{tier},
    };
    
    // Generate synthetic secret
    let secret = generate_secret_fixed_length(
        pattern.prefix,
        pattern.min_len,
        pattern.charset
    );
    
    // Create realistic context
    let contexts = vec![
        format!("API_KEY={}", secret),
        format!("export AUTH_TOKEN={}", secret),
        format!("\"key\": \"{}\"", secret),
        format!("key: {}", secret),
    ];
    
    // Test detection
    let detector = FFIDetector::new();
    for context in contexts {
        let matches = detector.detect(&context);
        
        assert!(
            matches.iter().any(|m| m.name == pattern.name),
            "Failed to detect {} in context: {}",
            pattern.name,
            context
        );
        
        let m = matches.iter().find(|m| m.name == pattern.name).unwrap();
        assert_eq!(m.tier, pattern.tier);
        assert_eq!(m.matched_text, secret);
    }
}
```

### Template: Category B (PREFIX + MIN_LENGTH)

```rust
#[test]
fn test_pattern_{name}_min_length() {
    let pattern = PatternDefinition {
        name: "{name}",
        prefix: "{prefix}",
        charset: CharsetType::{charset},
        min_len: {min_len},
        max_len: {max_len},  // 0 = unbounded
        tier: PatternTier::{tier},
    };
    
    // Test minimum length (should detect)
    let secret_min = generate_secret_min_length(pattern.prefix, pattern.min_len);
    let detector = FFIDetector::new();
    let matches_min = detector.detect(format!("key={}", secret_min));
    assert!(matches_min.iter().any(|m| m.name == pattern.name),
            "Failed to detect {} at minimum length", pattern.name);
    
    // Test above minimum (should detect)
    let secret_max = generate_secret_max_length(pattern.prefix, pattern.max_len);
    let matches_max = detector.detect(format!("key={}", secret_max));
    assert!(matches_max.iter().any(|m| m.name == pattern.name),
            "Failed to detect {} at maximum length", pattern.name);
    
    // Test below minimum (should NOT detect)
    let secret_short = generate_secret_below_min(pattern.prefix, pattern.min_len);
    let matches_short = detector.detect(format!("key={}", secret_short));
    assert!(!matches_short.iter().any(|m| m.name == pattern.name),
            "Incorrectly detected {} below minimum length", pattern.name);
}
```

### Template: Category C (MULTI-PREFIX - split patterns)

```rust
#[test]
fn test_pattern_{name}_split_prefix() {
    let prefixes = vec!["{prefix1}", "{prefix2}", "{prefix3}"];  // Split from alternation
    
    for (idx, prefix) in prefixes.iter().enumerate() {
        let pattern_name = format!("{}-{}", "{name}", idx);
        let secret = generate_secret_fixed_length(
            prefix,
            {expected_length},
            CharsetType::{charset}
        );
        
        let detector = FFIDetector::new();
        let matches = detector.detect(format!("token={}", secret));
        
        assert!(
            matches.iter().any(|m| m.name.contains(&pattern_name)),
            "Failed to detect {} with prefix '{}'",
            pattern_name,
            prefix
        );
    }
}
```

### Template: Category D (TRULY COMPLEX - regex only)

```rust
#[test]
fn test_pattern_{name}_regex_complex() {
    let pattern = PatternDefinition {
        name: "{name}",
        requires_regex: true,
        tier: PatternTier::{tier},
    };
    
    // Realistic example context (must match regex)
    let context = r#"{realistic_example}"#;
    
    let detector = FFIDetector::new();
    let matches = detector.detect(context);
    
    assert!(
        matches.iter().any(|m| m.name == pattern.name),
        "Failed to detect {} in context",
        pattern.name
    );
}
```

### Specific Pattern Examples

```rust
// Category A Examples
test_pattern_adafruitio() {
    secret: "aio_ABCDEFGHIJKLMNOPQRSTUVWXYZ01"  // 31 chars
    context: "aio_key=aio_ABCDEFGHIJKLMNOPQRSTUVWXYZ01"
}

test_pattern_anthropic_admin01() {
    secret: "sk-ant-admin01-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789AA"
    context: "anthropic_api_key=sk-ant-admin01-..."
}

// Category B Examples
test_pattern_github_pat_min_length() {
    secret_min: "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456"  // 36 chars min
    secret_max: "ghp_" + [a-zA-Z0-9 × 100]  // Unbounded
    secret_short: "ghp_ABCDEF"  // Below 36 (should NOT detect)
}

test_pattern_stripe_min_max() {
    secret_min: "sk_live_" + "a" × 20  // 28 chars min
    secret_max: "sk_live_" + "a" × 247  // 255 chars max
    secret_short: "sk_live_abc"  // Below 20 (should NOT detect)
}

// Category C Examples
test_pattern_digitalocean_split() {
    test dop_v1_[hex]{64}  ✓
    test doo_v1_[hex]{64}  ✓
    test dor_v1_[hex]{64}  ✓
}

// Category D Examples
test_pattern_authorization_header() {
    context: "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9..."
    must match: Bearer + JWT token
}

test_pattern_mongodb() {
    context: "MONGO_URL=mongodb+srv://user:pass@cluster0.mongodb.net/db"
    must match: Full connection string with credentials
}
```

---

## Phase 3: Test Execution & Results (45 minutes)

Run test suite and collect results.

### Execution Plan

```bash
# Step 1: Compile test suite
cargo test --test task2_pattern_mapping --no-run

# Step 2: Run tests with detailed output
cargo test --test task2_pattern_mapping -- --nocapture --test-threads=1

# Step 3: Collect results
cargo test --test task2_pattern_mapping 2>&1 | tee /tmp/task2_results.log

# Step 4: Parse results
grep "test result:" /tmp/task2_results.log
grep "^test " /tmp/task2_results.log | grep "ok\|FAILED"
```

### Expected Results

```
Category A (PREFIX + FIXED_LENGTH):
  Total: 80 patterns
  Expected: 80 pass, 0 fail
  Reason: Simple structure, SIMD-optimizable

Category B (PREFIX + MIN_LENGTH):
  Total: 20 patterns
  Expected: 20 pass, 0 fail
  Reason: Simple structure, SIMD-optimizable

Category C (MULTI-PREFIX):
  Total: 40 patterns (15 → 40 after split)
  Expected: 40 pass, 0 fail
  Reason: Each split variant is simple

Category D (TRULY COMPLEX):
  Total: 30 patterns
  Expected: 25-30 pass, 0-5 fail
  Reason: Regex complexity, some edge cases

Overall:
  Total: 270 patterns
  Expected: 260-270 pass (96-100%)
  Failures: 0-10 (edge cases only)
```

---

## Phase 4: Validation Report (20 minutes)

Document findings and create comprehensive report.

### Report Format

```markdown
# TASK 2: Pattern Mapping & Validation Report

## Executive Summary
- Total patterns: 270
- Successfully mapped: 270
- Successfully tested: 260-270
- Blockers: 0 (or documented)
- Performance: On track for 45-50 MB/s

## Category Breakdown

### Category A: PREFIX + FIXED_LENGTH (80 patterns) ✅
Status: ALL PASS
- Patterns tested: 80
- Passed: 80
- Failed: 0
- Performance: 0.1 ms/MB (as expected)

Examples:
- ✅ adafruitio: aio_[a-zA-Z0-9]{28}
- ✅ clojars: CLOJARS_[a-z0-9]{60}
- ✅ groq: gsk_[a-zA-Z0-9]{52}

### Category B: PREFIX + MIN_LENGTH (20 patterns) ✅
Status: ALL PASS
- Patterns tested: 20
- Passed: 20
- Failed: 0
- Performance: 0.1 ms/MB (as expected)

Examples:
- ✅ github-pat: ghp_[0-9a-zA-Z]{36,}
- ✅ stripe: [rs]k_live_[a-zA-Z0-9]{20,247}
- ✅ razorpay: rzp_live_[A-Za-z0-9]{14,}

### Category C: MULTI-PREFIX (40 patterns from 15) ✅
Status: ALL PASS
- Original patterns: 15
- Split patterns: 40
- Passed: 40
- Failed: 0
- Performance: 0.1 ms/MB per prefix (as expected)

Examples:
- ✅ digitalocean: dop_, doo_, dor_ (3 patterns)
- ✅ huggingface: hf_, api_org_ (2 patterns)

### Category D: TRULY COMPLEX (30 patterns) ⚠️
Status: 25/30 PASS (83%)
- Patterns tested: 30
- Passed: 25
- Failed: 5
- Issues documented below

Examples passing:
- ✅ authorization_header
- ✅ jwt
- ✅ mongodb

Examples with issues:
- ⚠️ private-key: Requires multi-line handling
- ⚠️ uri: Complex URL parsing edge cases

## Tier Assignment Verification

CRITICAL tier (high priority):
- Patterns: 50
- All verified ✅

API_KEYS tier (medium priority):
- Patterns: 150
- All verified ✅

INFRASTRUCTURE tier:
- Patterns: 40
- All verified ✅

PATTERNS tier (low priority):
- Patterns: 30
- 25/30 verified ✅

## FFI Function Mapping

All 10 FFI functions used:
1. detect_content_type() ✅
2. get_candidate_patterns() ✅
3. match_patterns() ✅
4. get_pattern_info() ✅
5. validate_charset() ✅ (Categories A, B, C)
6. match_prefix() ✅ (Categories A, B, C)
7. match_regex() ✅ (Category D)
8. get_pattern_tier() ✅
9. allocate_match_result() ✅
10. free_match_result() ✅

## Performance Validation

Category A performance:
- Expected: 0.1 ms/MB
- Measured: 0.09-0.11 ms/MB ✅

Category B performance:
- Expected: 0.1 ms/MB
- Measured: 0.09-0.12 ms/MB ✅

Category C performance:
- Expected: 0.1 ms/MB per pattern
- Measured: 0.09-0.11 ms/MB ✅

Category D performance:
- Expected: 1.3 ms/MB
- Measured: 1.2-1.5 ms/MB ✅

Overall throughput impact:
- Baseline (10 patterns): 43 MB/s
- With 270 decomposed: 45-50 MB/s (estimated)
- Improvement: 15-25% ✅

## Blockers & Issues

### No critical blockers identified ✅

Minor issues (Category D - fixable):
1. Private-key detection needs line-break handling
   - Fix: Adjust regex for multiline matching
   - Impact: Low (not commonly used in single line)
   
2. URI parsing edge cases
   - Fix: Improve URL validation logic
   - Impact: Low (covered by specific patterns)

## Recommendations

1. Proceed with implementation using decomposed patterns ✅
2. Use fast-path (Categories A, B, C) for 210+ patterns
3. Use regex path (Category D) for 30 complex patterns
4. Expect 45-50 MB/s throughput on full 270-pattern set
5. Address Category D edge cases in next iteration (optional)

## Validation Complete ✅

All 270 patterns mapped to FFI functions
All 270 patterns validated with synthetic test cases
All 270 patterns verified for correct tier assignment
Performance targets confirmed achievable
Ready for Task 5 (comprehensive test suite)
```

---

## Test Case File Structure

Create: `crates/scred-pattern-detector/tests/task2_pattern_mapping.rs`

```rust
//! Task 2: Pattern Mapping & Validation
//! 
//! Tests all 270 patterns using Task 1 decomposition strategy:
//! - Category A (80): PREFIX + FIXED_LENGTH
//! - Category B (20): PREFIX + MIN_LENGTH
//! - Category C (40): MULTI-PREFIX (split from 15)
//! - Category D (30): TRULY COMPLEX (regex)

#[cfg(test)]
mod task2_patterns {
    use scred_pattern_detector::*;
    
    // Category A: PREFIX + FIXED_LENGTH
    #[test]
    fn test_category_a_fixed_length_patterns() { ... }
    
    // Category B: PREFIX + MIN_LENGTH
    #[test]
    fn test_category_b_min_length_patterns() { ... }
    
    // Category C: MULTI-PREFIX
    #[test]
    fn test_category_c_multi_prefix_patterns() { ... }
    
    // Category D: TRULY COMPLEX
    #[test]
    fn test_category_d_complex_patterns() { ... }
    
    // Helper functions
    fn generate_secret_fixed_length(prefix: &str, len: usize, charset: CharsetType) -> String { ... }
    fn generate_secret_min_length(prefix: &str, min: usize) -> String { ... }
    fn generate_secret_below_min(prefix: &str, min: usize) -> String { ... }
    fn create_realistic_context(pattern: &str, secret: &str) -> String { ... }
}
```

---

## Success Criteria

✅ All 270 patterns classified correctly
✅ 80+ Category A patterns validated
✅ 20+ Category B patterns validated
✅ 40+ Category C patterns validated (split)
✅ 30 Category D patterns validated
✅ All tier assignments verified
✅ All FFI functions used correctly
✅ Performance projections confirmed
✅ Test case file ready for Task 5
✅ Comprehensive report generated

---

## Next Steps

After Task 2 completion:

1. Proceed to Task 3: Streaming Metadata Design
   - Design metadata struct with tier info
   - Plan cross-component validation

2. Then Task 5: Comprehensive Test Suite
   - Use 270 test cases from Task 2
   - Create full test suite
   - Implement performance benchmarks

---

## Timeline Summary

- **Phase 1** (15 min): Verify decomposition
- **Phase 2** (30 min): Generate test cases
- **Phase 3** (45 min): Execute tests
- **Phase 4** (20 min): Generate report
- **Total**: 110 minutes (under 2 hours)
- **Remaining**: 10 minutes buffer

Ready to execute!
