# TASK 2: Pattern Mapping & Validation - Execution Plan

## Overview

Map and validate all 270 patterns using the Task 1 decomposition strategy:
- **80+ patterns**: Category A (PREFIX + FIXED_LENGTH)
- **20+ patterns**: Category B (PREFIX + MIN_LENGTH)
- **40+ patterns**: Category C (MULTI-PREFIX alternation)
- **30+ patterns**: Category D (TRULY COMPLEX - keep regex)

**Duration**: 2 hours
**Deliverable**: 270 test cases with synthetic examples + validation results

---

## Strategy: Leverage Task 1 Decomposition

### Phase 1: Pattern Classification Validation (30 minutes)

Verify Task 1's decomposition inventory is accurate:

```python
# For each pattern in patterns.zig, extract and classify

# Category A: PREFIX + FIXED_LENGTH
verify_fixed_length_patterns = [
    ("adafruitio", "aio_[a-zA-Z0-9]{28}", prefix="aio_", fixed_len=31),
    ("age-secret-key", "AGE-SECRET-KEY-1[...]{58}", prefix="AGE-SECRET-KEY-1", fixed_len=59),
    ("apideck", "sk_live_[a-z0-9A-Z-]{93}", prefix="sk_live_", fixed_len=101),
    # ... 80 patterns
]

# Category B: PREFIX + MIN_LENGTH
verify_min_length_patterns = [
    ("github-pat", "ghp_[0-9a-zA-Z]{36,}", prefix="ghp_", min_len=36),
    ("stripe", "[rs]k_live_[...]{20,247}", prefix="[rs]k_live_", min_len=20),
    # ... 20 patterns
]

# Category C: MULTI-PREFIX (ALTERNATION)
verify_alternation_patterns = [
    ("digitalocean", "(dop|doo|dor)_v1_[a-f0-9]{64}", prefixes=["dop_v1_", "doo_v1_", "dor_v1_"]),
    # ... 15 patterns
]

# Category D: TRULY COMPLEX
verify_complex_patterns = [
    ("authorization_header", "(?i)Authorization:\\s*(?:Bearer|Basic|Token)\\s+..."),
    # ... 30 patterns
]
```

### Phase 2: Create Synthetic Test Cases (1 hour)

For each of 270 patterns, create:
1. **Synthetic secret** matching the pattern
2. **Context** (where secret appears)
3. **Expected match** (what should be detected)
4. **Tier assignment** (CRITICAL, API_KEYS, etc.)

Example structure:

```rust
#[test]
fn test_pattern_adafruitio_detection() {
    let secret = "aio_ABCDEFGHIJ0123456789";  // 28 chars alphanumeric
    let context = format!(
        "API_KEY={}",
        secret
    );
    
    let detector = create_detector();
    let matches = detector.detect(&context);
    
    assert!(matches.iter().any(|m| m.name == "adafruitio"));
    assert_eq!(matches[0].tier, PatternTier::API_KEYS);
}
```

### Phase 3: Pattern Tier Validation (20 minutes)

Verify tier assignment for each pattern:

```
TIER: CRITICAL (highest priority)
- AWS access keys, GitHub tokens, Private keys, JWT
- Count: ~50 patterns
- Validation: Check tier == CRITICAL in patterns.zig

TIER: API_KEYS (medium priority)
- Most service API keys
- Count: ~150 patterns
- Validation: Check tier == API_KEYS

TIER: INFRASTRUCTURE (platform secrets)
- Database passwords, connection strings
- Count: ~40 patterns
- Validation: Check tier == INFRASTRUCTURE

TIER: PATTERNS (generic/low-priority)
- Headers, generic patterns
- Count: ~30 patterns
- Validation: Check tier == PATTERNS
```

### Phase 4: FFI Function Mapping (10 minutes)

Map each pattern to its detector function:

```rust
// All patterns use these 10 FFI functions from detector_ffi.zig

Functions:
1. detect_content_type() → Identify content (HTTP, JSON, ENV)
2. get_candidate_patterns() → Get relevant patterns for content
3. match_patterns() → Main detection function
4. get_pattern_info() → Get pattern metadata
5. validate_charset() → Character class validation (for Tier 1)
6. match_prefix() → Prefix matching (for Tier 1)
7. match_regex() → Regex matching (for Tier 2)
8. get_pattern_tier() → Get pattern criticality
9. allocate_match_result() → Memory management
10. free_match_result() → Cleanup

Mapping:
- All 71 original simple patterns → Via match_prefix() + validate_charset()
- All 80+ decomposed patterns (Cat A) → Via match_prefix() + validate_charset()
- All 20+ decomposed patterns (Cat B) → Via match_prefix() + validate_charset()
- All 40+ split patterns (Cat C) → Via match_prefix() + validate_charset()
- All 30+ complex patterns (Cat D) → Via match_regex()
```

---

## Detailed Test Case Creation

### Category A: PREFIX + FIXED_LENGTH (80+ patterns)

Template:

```rust
#[test]
fn test_pattern_{name}() {
    // Pattern definition
    let pattern_name = "{name}";
    let prefix = "{prefix}";
    let fixed_len = {fixed_len};
    let charset = CharsetType::{charset};
    
    // Synthetic secret: prefix + random chars matching charset
    let secret = generate_secret(prefix, fixed_len - prefix.len(), charset);
    
    // Context: secret in realistic location
    let context = generate_context(pattern_name, &secret);
    
    // Test
    let detector = FFIDetector::new();
    let matches = detector.detect(&context);
    
    // Verify
    assert!(matches.iter().any(|m| m.name == pattern_name));
    assert_eq!(matches[0].tier, PatternTier::{expected_tier});
    assert_eq!(matches[0].matched_text, secret);
}
```

**Examples**:

```rust
// Pattern: adafruitio
// Regex: aio_[a-zA-Z0-9]{28}
test_pattern_adafruitio {
    secret: "aio_ABCDEFGHIJKLMNOPQRSTUVWXYZ01"  // 31 chars total
    context: "adafruitio_key = aio_ABCDEFGHIJKLMNOPQRSTUVWXYZ01"
    expected_tier: API_KEYS
}

// Pattern: age-secret-key
// Regex: AGE-SECRET-KEY-1[QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L]{58}
test_pattern_age_secret_key {
    secret: "AGE-SECRET-KEY-1QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7LQPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L"  // 59 chars
    context: "export AGE_SECRET_KEY=AGE-SECRET-KEY-1QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L..."
    expected_tier: CRITICAL
}

// Pattern: stripe
// Regex: [rs]k_live_[a-zA-Z0-9]{20,247}
test_pattern_stripe_live {
    secret: "sk_live_ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJK"  // 47 chars min
    context: "stripe_key = sk_live_ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJK"
    expected_tier: CRITICAL
}
```

### Category B: PREFIX + MIN_LENGTH (20+ patterns)

Similar template, but verify minimum length constraint:

```rust
#[test]
fn test_pattern_{name}_min_length() {
    let secret_min = generate_secret_at_min_length(prefix, min_len);
    let secret_over = generate_secret_at_min_length(prefix, min_len + 10);
    
    let detector = FFIDetector::new();
    
    // Should detect both
    let matches_min = detector.detect(format!("key={}", secret_min));
    assert!(matches_min.iter().any(|m| m.name == pattern_name));
    
    let matches_over = detector.detect(format!("key={}", secret_over));
    assert!(matches_over.iter().any(|m| m.name == pattern_name));
    
    // Should NOT detect below minimum
    let secret_short = generate_secret_below_min(prefix, min_len);
    let matches_short = detector.detect(format!("key={}", secret_short));
    assert!(!matches_short.iter().any(|m| m.name == pattern_name));
}
```

### Category C: MULTI-PREFIX (ALTERNATION) (40+ split patterns)

Test each prefix variant separately:

```rust
#[test]
fn test_pattern_digitalocean_dop() {
    let secret = "dop_v1_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567";  // 64 hex chars
    let detector = FFIDetector::new();
    let matches = detector.detect(format!("token={}", secret));
    
    assert!(matches.iter().any(|m| m.name == "digitalocean-dop"));
}

#[test]
fn test_pattern_digitalocean_doo() {
    let secret = "doo_v1_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567";
    let detector = FFIDetector::new();
    let matches = detector.detect(format!("token={}", secret));
    
    assert!(matches.iter().any(|m| m.name == "digitalocean-doo"));
}

#[test]
fn test_pattern_digitalocean_dor() {
    let secret = "dor_v1_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567";
    let detector = FFIDetector::new();
    let matches = detector.detect(format!("token={}", secret));
    
    assert!(matches.iter().any(|m| m.name == "digitalocean-dor"));
}
```

### Category D: TRULY COMPLEX (30+ regex patterns)

Test regex-only patterns:

```rust
#[test]
fn test_pattern_authorization_header() {
    let secret = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let context = format!("Authorization: {}", secret);
    
    let detector = FFIDetector::new();
    let matches = detector.detect(&context);
    
    assert!(matches.iter().any(|m| m.name == "authorization_header"));
    assert_eq!(matches[0].tier, PatternTier::PATTERNS);
}

#[test]
fn test_pattern_jwt() {
    let secret = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let context = format!("jwt_token = {}", secret);
    
    let detector = FFIDetector::new();
    let matches = detector.detect(&context);
    
    assert!(matches.iter().any(|m| m.name == "jwt"));
    assert_eq!(matches[0].tier, PatternTier::PATTERNS);
}

#[test]
fn test_pattern_mongodb_connection_string() {
    let secret = "mongodb+srv://user:password@cluster0.mongodb.net/database?retryWrites=true";
    let context = format!("db_url = {}", secret);
    
    let detector = FFIDetector::new();
    let matches = detector.detect(&context);
    
    assert!(matches.iter().any(|m| m.name == "mongodb"));
    assert_eq!(matches[0].tier, PatternTier::INFRASTRUCTURE);
}
```

---

## Implementation Steps

### Step 1: Extract Patterns from patterns.zig (15 min)

```bash
# Extract all patterns
grep -E '\.name = "|\.pattern = "' \
    crates/scred-pattern-detector/src/patterns.zig | \
    sed 's/^.*\.name = "//' | \
    sed 's/", \.pattern.*//' > /tmp/pattern_names.txt

# Count by category
wc -l /tmp/pattern_names.txt  # Should be ~270
```

### Step 2: Generate Test Case Template (30 min)

```rust
// In crates/scred-pattern-detector/tests/task2_pattern_mapping.rs

#[cfg(test)]
mod task2_pattern_mapping {
    use scred_pattern_detector::FFIDetector;
    
    // Category A: PREFIX + FIXED_LENGTH (80+ patterns)
    #[test]
    fn test_category_a_patterns() {
        let patterns_a = vec![
            // (name, prefix, fixed_len, example_secret, expected_tier)
            ("adafruitio", "aio_", 31, "aio_ABCDEFGHIJKLMNOPQRSTUVWXYZ01", "API_KEYS"),
            // ... 80 patterns
        ];
        
        let detector = FFIDetector::new();
        for (name, _, _, secret, tier) in patterns_a {
            let context = format!("key = {}", secret);
            let matches = detector.detect(&context);
            
            assert!(matches.iter().any(|m| m.name == name),
                    "Pattern {} not detected in context: {}", name, context);
            assert_eq!(matches[0].tier, tier);
        }
    }
    
    // Category B: PREFIX + MIN_LENGTH (20+ patterns)
    #[test]
    fn test_category_b_patterns() {
        let patterns_b = vec![
            // (name, prefix, min_len, example_secret, expected_tier)
            ("github-pat", "ghp_", 36, "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123", "CRITICAL"),
            // ... 20 patterns
        ];
        
        let detector = FFIDetector::new();
        for (name, _, _, secret, tier) in patterns_b {
            let context = format!("key = {}", secret);
            let matches = detector.detect(&context);
            
            assert!(matches.iter().any(|m| m.name == name),
                    "Pattern {} not detected", name);
            assert_eq!(matches[0].tier, tier);
        }
    }
    
    // Category C: MULTI-PREFIX (40+ patterns)
    #[test]
    fn test_category_c_patterns() {
        let patterns_c = vec![
            // (name, prefix, fixed_len, example_secret, expected_tier)
            ("digitalocean-dop", "dop_v1_", 71, "dop_v1_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567", "API_KEYS"),
            ("digitalocean-doo", "doo_v1_", 71, "doo_v1_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567", "API_KEYS"),
            // ... 40 patterns
        ];
        
        let detector = FFIDetector::new();
        for (name, _, _, secret, tier) in patterns_c {
            let context = format!("key = {}", secret);
            let matches = detector.detect(&context);
            
            assert!(matches.iter().any(|m| m.name == name),
                    "Pattern {} not detected", name);
        }
    }
    
    // Category D: TRULY COMPLEX (30+ patterns)
    #[test]
    fn test_category_d_patterns() {
        let patterns_d = vec![
            // (name, example_secret, expected_tier)
            ("authorization_header", "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...", "PATTERNS"),
            ("jwt", "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c", "PATTERNS"),
            // ... 30 patterns
        ];
        
        let detector = FFIDetector::new();
        for (name, secret, tier) in patterns_d {
            let matches = detector.detect(secret);
            
            assert!(matches.iter().any(|m| m.name == name),
                    "Pattern {} not detected in: {}", name, secret);
        }
    }
}
```

### Step 3: Run Tests and Document Results (45 min)

```bash
# Run tests
cargo test --test task2_pattern_mapping -- --nocapture

# Capture output
cargo test --test task2_pattern_mapping 2>&1 | tee /tmp/task2_results.txt

# Analyze failures
grep "test result:" /tmp/task2_results.txt
grep "assertion failed" /tmp/task2_results.txt
```

### Step 4: Create Validation Report (20 min)

Document:
- ✅ Patterns that work
- ⚠️ Patterns with issues
- ❌ Patterns that don't work
- Blockers identified

Format:

```
# TASK 2: Pattern Mapping & Validation Results

## Summary
- Total patterns tested: 270
- Passed: 260
- Failed: 10
- Blocked: 0

## Category A: PREFIX + FIXED_LENGTH (80+ patterns)
Status: ✅ ALL PASS (80/80)
Examples:
  ✅ adafruitio: aio_[...]{28}
  ✅ apideck: sk_live_[...]{93}
  ✅ clojars: CLOJARS_[...]{60}

## Category B: PREFIX + MIN_LENGTH (20+ patterns)
Status: ✅ ALL PASS (20/20)
Examples:
  ✅ github-pat: ghp_[...]{36,}
  ✅ stripe: [rs]k_live_[...]{20,247}

## Category C: MULTI-PREFIX (40+ patterns)
Status: ✅ ALL PASS (40/40)
Examples:
  ✅ digitalocean-dop, doo, dor
  ✅ huggingface: hf_, api_org_

## Category D: TRULY COMPLEX (30+ patterns)
Status: ⚠️ 10 ISSUES (20/30 pass)
Issues:
  ❌ private-key: Requires proper PEM format
  ❌ mongodb: Connection string parsing needs refinement
  ⚠️ jwt: Works but needs validation of payload

## Blockers
- None for Tier 1 (fast-path patterns)
- Minor issues in Tier 2 (regex patterns) - all fixable

## Recommendations
1. Proceed with implementation using decomposed patterns
2. Address Tier 2 regex issues in subsequent refinement
3. Performance target remains: 45-50 MB/s
```

---

## Success Criteria

✅ All 270 patterns mapped to FFI functions
✅ Synthetic test examples created for each pattern
✅ Pattern tier assignments verified
✅ Category A (80+ patterns) validated with Tier 1 FFI
✅ Category B (20+ patterns) validated with Tier 1 FFI
✅ Category C (40+ patterns) validated with Tier 1 FFI (split)
✅ Category D (30+ patterns) validated with Tier 2 FFI
✅ Blockers documented (if any)
✅ Test case file ready for Task 5

---

## Deliverable

**File**: `TASK2_PATTERN_MAPPING_VALIDATION.md` (500+ lines)

Contents:
- Pattern mapping inventory (270 patterns → functions)
- Test case templates (all 4 categories)
- Validation results (pass/fail status)
- Tier assignments verified
- Blockers documented
- Recommendations for Task 5

---

## Timeline

- Step 1: Pattern extraction - 15 min
- Step 2: Test template generation - 30 min
- Step 3: Test execution - 45 min
- Step 4: Results documentation - 20 min
- Total: ~110 minutes (just under 2 hours)

Remaining 10 minutes: Buffer for troubleshooting/unexpected issues
