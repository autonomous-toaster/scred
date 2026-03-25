# Pattern Decomposition Analysis: REGEX → PREFIX_VALIDATION

**Goal**: Convert easy-to-decompose REGEX patterns to PREFIX_VALIDATION for 2-3x speedup

**Current State**:
- SIMPLE_PREFIX: 26 patterns (pure prefix, no validation)
- PREFIX_VALIDATION: 45 patterns (prefix + charset/length)
- JWT: 1 pattern
- REGEX: 203 patterns (many decomposable)
- **Total: 275 patterns**

**Target**: Decompose 50-75 REGEX patterns to PREFIX_VALIDATION
- Goal: 95-120 PREFIX_VALIDATION patterns (from current 45)
- Gain: 50-75 additional patterns checked via SIMD
- Estimated speedup: 2-3x on matching phase

---

## Easy Decomposition Candidates (Score 9-10)

### 1. **adafruitio** (TRIVIAL)
```rust
// Current REGEX:
\b(aio\_[a-zA-Z0-9]{28})\b

// Decompose to:
PREFIX_VALIDATION {
    name: "adafruitio",
    prefix: "aio_",
    charset: alphanumeric,
    min_len: 28,
    max_len: 28,
}

// Why: Fixed-length token after clear prefix
// Difficulty: ⭐ TRIVIAL
```

### 2. **age-secret-key** (TRIVIAL)
```rust
// Current REGEX:
AGE-SECRET-KEY-1[QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L]{58}

// Decompose to:
PREFIX_VALIDATION {
    name: "age-secret-key",
    prefix: "AGE-SECRET-KEY-1",
    charset: base64url,  // Uses Bech32 charset
    min_len: 58,
    max_len: 58,
}

// Why: Fixed format, clear terminator
// Difficulty: ⭐ TRIVIAL
// Note: May need custom charset for Bech32
```

### 3. **slack-bot-token** (TRIVIAL)
```rust
// Current REGEX (example):
xoxb-[0-9]{10,13}-[0-9]{10,13}-[a-zA-Z0-9]{24}

// Decompose to:
PREFIX_VALIDATION {
    name: "slack-bot-token",
    prefix: "xoxb-",
    charset: alphanumeric_dash,  // [0-9a-zA-Z-]
    min_len: 38,  // 10+1+10+1+24 minimum
    max_len: 52,  // 13+1+13+1+24 maximum
}

// Why: Tokenized format with known structure
// Difficulty: ⭐ TRIVIAL
```

### 4. **slack-app-token** (TRIVIAL)
```rust
// Similar to bot token
PREFIX_VALIDATION {
    name: "slack-app-token",
    prefix: "xoxp-",
    charset: alphanumeric_dash,
    min_len: 38,
    max_len: 52,
}
```

### 5. **github-oauth-token** (EASY)
```rust
// Current REGEX:
gho_[0-9a-zA-Z]{36}

// Decompose to:
PREFIX_VALIDATION {
    name: "github-oauth-token",
    prefix: "gho_",
    charset: alphanumeric,
    min_len: 36,
    max_len: 36,
}
```

### 6. **github-fine-grained-token** (EASY)
```rust
// Current REGEX:
github_pat_[0-9a-zA-Z_]{82}

// Decompose to:
PREFIX_VALIDATION {
    name: "github-fine-grained-token",
    prefix: "github_pat_",
    charset: alphanumeric_underscore,
    min_len: 82,
    max_len: 82,
}
```

### 7. **npm-token** (EASY)
```rust
// Current REGEX (example):
npm_[A-Za-z0-9_-]{36}

// Decompose to:
PREFIX_VALIDATION {
    name: "npm-token",
    prefix: "npm_",
    charset: base64url,  // [a-zA-Z0-9_-]
    min_len: 36,
    max_len: 36,
}
```

### 8. **stripe-api-key** (EASY)
```rust
// Current REGEX:
sk_(?:live|test)_[0-9a-zA-Z]{32,100}

// Decompose to TWO patterns:
PREFIX_VALIDATION {
    name: "stripe-api-key-live",
    prefix: "sk_live_",
    charset: alphanumeric,
    min_len: 32,
    max_len: 100,
}

PREFIX_VALIDATION {
    name: "stripe-api-key-test",
    prefix: "sk_test_",
    charset: alphanumeric,
    min_len: 32,
    max_len: 100,
}
```

### 9. **openai-api-key** (EASY)
```rust
// Current REGEX:
sk-proj-[a-zA-Z0-9_-]{20,}

// Already decomposed!
// See patterns.zig - sk-proj- is PREFIX_VALIDATION
```

### 10. **sendgrid-token** (EASY)
```rust
// Current REGEX (example):
SG\.[0-9a-zA-Z_-]{60,}

// Decompose to:
PREFIX_VALIDATION {
    name: "sendgrid-token",
    prefix: "SG.",
    charset: base64url,
    min_len: 60,
    max_len: 100,  // Reasonable upper bound
}
```

---

## Medium Decomposition Candidates (Score 7-8)

### 11. **AWS-Managed-Temporary-Credential** (MEDIUM)
```rust
// Current REGEX:
ASIA[0-9A-Z]{16}

// Decompose to:
PREFIX_VALIDATION {
    name: "aws-temporary-credential",
    prefix: "ASIA",
    charset: alphanumeric,
    min_len: 16,
    max_len: 16,
}

// Difficulty: ⭐⭐ EASY (clear prefix, fixed length)
```

### 12. **databricks-token** (MEDIUM)
```rust
// Current REGEX:
dapi[a-h0-9]{32}

// Decompose to:
PREFIX_VALIDATION {
    name: "databricks-token",
    prefix: "dapi",
    charset: base64url,  // [a-h0-9] is subset
    min_len: 32,
    max_len: 32,
}

// Difficulty: ⭐⭐ EASY
```

### 13. **mailchimp-api-key** (MEDIUM)
```rust
// Current REGEX:
[0-9a-f]{32}-us[0-9]{1,2}

// Decompose to:
PREFIX_VALIDATION {
    name: "mailchimp-api-key",
    prefix: "-us",  // Actually, this is suffix-based
    charset: hex_and_dash_and_letters,
    // This pattern is SUFFIX-based, not PREFIX-based
    // Requires special handling
    // Difficulty: ⭐⭐⭐ HARD (suffix pattern, not prefix)
}

// Better approach: Decompose as TWO checks
// 1. Find "-us" using standard search
// 2. Validate prefix is 32 hex chars
// For now: SKIP (not worth the effort)
```

---

## Hard Decomposition (Score 4-6)

### 14. **Private Key Patterns** (HARD)
```rust
// Current REGEX:
-----BEGIN (?:RSA |DSA |EC )?PRIVATE KEY-----

// This is a multi-line pattern
// Decompose to:
PREFIX_VALIDATION {
    name: "private-key-header",
    prefix: "-----BEGIN",
    charset: alphanumeric_space_dash,
    // Need to validate "PRIVATE KEY-----" ending
    // Not ideal for simple PREFIX_VALIDATION
}

// Difficulty: ⭐⭐⭐ HARD (requires multi-line handling)
```

### 15. **Certificate Patterns** (HARD)
```rust
// Similar issues to private keys
// Difficulty: ⭐⭐⭐ HARD
```

---

## Decomposition Implementation Plan

### Phase 1: TRIVIAL Patterns (2-3 hours)
1. adafruitio (aio_)
2. age-secret-key (AGE-SECRET-KEY-1)
3. slack-bot-token (xoxb-)
4. slack-app-token (xoxp-)

**Expected**: +4 new PREFIX_VALIDATION patterns
**Impact**: 10-15% speedup on Slack + AdaFruit + AGE logs

### Phase 2: EASY Patterns (3-4 hours)
5. github-oauth-token (gho_)
6. github-fine-grained-token (github_pat_)
7. npm-token (npm_)
8. stripe-api-key (sk_live_, sk_test_)
9. sendgrid-token (SG.)

**Expected**: +9 new PREFIX_VALIDATION patterns (stripe creates 2)
**Impact**: 25-30% speedup on GitHub + NPM + Stripe + SendGrid logs

### Phase 3: MEDIUM Patterns (2-3 hours)
10. AWS-Managed-Temporary (ASIA)
11. databricks-token (dapi)

**Expected**: +2 new PREFIX_VALIDATION patterns
**Impact**: 5-10% speedup on AWS logs

### Total: 7-10 hours → +15 new patterns
### From 45 → 60 PREFIX_VALIDATION patterns
### Estimated speedup: 1.5-2x on prefix matching phase

---

## Charset Definitions Needed

```zig
pub const Charset = enum {
    alphanumeric,           // [a-zA-Z0-9]
    base64,                 // [a-zA-Z0-9+/=]
    base64url,              // [a-zA-Z0-9_-=]
    hex,                    // [0-9a-fA-F]
    hex_lowercase,          // [0-9a-f]
    any,                    // [^whitespace/special]
    alphanumeric_underscore, // [a-zA-Z0-9_]
    alphanumeric_dash,      // [a-zA-Z0-9-]
    alphanumeric_underscore_dash, // [a-zA-Z0-9_-]
};
```

---

## Test Strategy

For each decomposed pattern:
1. Verify same prefix matching
2. Compare results to original REGEX
3. Test edge cases (min/max length)
4. Benchmark SIMD vs REGEX

Example test:
```rust
#[test]
fn test_decomposed_adafruitio_matches_original() {
    let input = "key: aio_abcd1234efgh5678ijkl90mn";
    
    // Original REGEX finds it
    let regex_matches = find_regex_matches("adafruitio", input);
    
    // Decomposed PREFIX_VALIDATION finds it
    let prefix_matches = find_prefix_validation_matches("adafruitio", input);
    
    assert_eq!(regex_matches.len(), prefix_matches.len());
    assert_eq!(regex_matches[0].start, prefix_matches[0].start);
}
```

---

## Implementation Checklist

- [ ] Add decomposed patterns to patterns.zig
- [ ] Update PREFIX_VALIDATION_PATTERNS array
- [ ] Add charset enum variants if needed
- [ ] Add test cases for each new pattern
- [ ] Verify 100% pattern parity with REGEX
- [ ] Benchmark improvement on real data
- [ ] Update documentation
- [ ] Commit with evidence of speedup

---

## Expected Results

**Before**:
- PREFIX_VALIDATION: 45 patterns
- Coverage: ~40% of common secrets

**After Phase 1-3**:
- PREFIX_VALIDATION: 60+ patterns
- Coverage: ~55% of common secrets
- Speedup: 1.5-2x on prefix matching
- Throughput: 63 MB/s → ~80-90 MB/s (estimated)

---

## Next Steps

1. Implement Phase 1 patterns (TRIVIAL) - Quick wins
2. Add comprehensive tests
3. Benchmark SIMD improvement
4. Document actual performance gains
5. If Phase 1 successful, proceed to Phase 2

