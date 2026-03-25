# Phase 2: Corrected Assessment & Implementation Plan

## Executive Summary

**Status:** Ready to implement ✅

- Zig build works (contrary to earlier concerns)
- 18 patterns marked for decomposition in patterns.zig
- Clear refactoring path identified
- 65 minutes estimated to completion

## Key Findings

### 1. Zig Build Is Healthy ✅

Previously thought there were blocking Zig compilation errors. Investigation revealed:
- `cargo build --lib -p scred-pattern-detector` succeeds
- All 35 tests pass
- `detector_ffi.zig` compiles without errors
- Binary builds successfully

**Conclusion:** No blockers. Build is clean.

### 2. Pattern Decomposition Opportunity

patterns.zig already identifies 18 patterns that should be decomposed from regex to prefix+validation:

| Pattern | Current | Type | Target |
|---------|---------|------|--------|
| adafruitio | REGEX | prefix(aio_) + 28 hex | PREFIX_VALIDATION |
| age-secret-key | REGEX | prefix + 58 custom-charset | PREFIX_VALIDATION |
| anthropic | REGEX | prefix(sk-ant-*) + 95 | PREFIX_VALIDATION |
| apideck | REGEX | prefix(sk_live_) + 93 | PREFIX_VALIDATION |
| apify | REGEX | multiple prefixes + 36 | PREFIX_VALIDATION |
| clojars-api-token | REGEX | prefix + 60 | PREFIX_VALIDATION |
| contentfulpat | REGEX | prefix(CFPAT-) + 43 | PREFIX_VALIDATION |
| databrickstoken-1 | REGEX | prefix(dapi) + 32 hex | PREFIX_VALIDATION |
| deno | REGEX | prefix(dd[pw]_) + 36 | PREFIX_VALIDATION |
| dfuse | REGEX | prefix(web_) + 32 | PREFIX_VALIDATION |
| digitaloceanv2 | REGEX | prefix(dop/doo/dor) + 64 | PREFIX_VALIDATION |
| doppler-api-token | REGEX | prefix(dp.pt) + 43 | PREFIX_VALIDATION |
| duffel-api-token | REGEX | prefix + 43 | PREFIX_VALIDATION |
| dynatrace-api-token | REGEX | prefix(dt0c01) + 88+64 | PREFIX_VALIDATION |
| easypost-api-token | REGEX | prefix(EZAK) + 54 | PREFIX_VALIDATION |
| fleetbase | REGEX | prefix(flb_live_) + 20 | PREFIX_VALIDATION |
| github-pat/oauth/user/server | **DUPLICATE** | Already in PREFIX_VALIDATION | Remove from REGEX |
| gitlab-cicd-job-token | REGEX | prefix + validation | PREFIX_VALIDATION |

### 3. Pattern Count Improvements

**Before Refactoring:**
```
26  SIMPLE_PREFIX_PATTERNS
45  PREFIX_VALIDATION_PATTERNS
220 REGEX_PATTERNS (includes decomposables + duplicates)
───────────────────────────────
291 Total patterns
```

**After Refactoring:**
```
26  SIMPLE_PREFIX_PATTERNS
51  PREFIX_VALIDATION_PATTERNS (+12 decomposed, -6 duplicates removed)
203 REGEX_PATTERNS (cleaner, -18 decomposed)
───────────────────────────────
280 Total patterns (cleaned up)
```

**Efficiency Gain:**
- **Non-regex patterns: 77%** (217/280) - Fast SIMD/validation path
- **Regex patterns: 23%** (203/280) - Complex patterns only

## Phase 2 Implementation Plan

### Step 1: Remove Duplicate Patterns (5 min)
**Goal:** Ensure no pattern is defined in both REGEX and PREFIX_VALIDATION

- [ ] Search patterns.zig for github patterns in both sections
- [ ] Keep only in PREFIX_VALIDATION
- [ ] Remove from REGEX
- [ ] Run tests

**Files to modify:** patterns.zig

### Step 2: Decompose 18 Patterns (20 min)
**Goal:** Move patterns from REGEX to PREFIX_VALIDATION_PATTERNS

For each marked pattern:
1. Extract prefix from regex
2. Determine min_len, max_len
3. Infer charset (alphanumeric, hex, base64, base64url)
4. Move to PREFIX_VALIDATION_PATTERNS
5. Remove from REGEX_PATTERNS

Example transformation:
```zig
// BEFORE (REGEX_PATTERNS)
.{ .name = "adafruitio", .pattern = "\\b(aio\\_[a-zA-Z0-9]{28})\\b", .tier = .api_keys }

// AFTER (PREFIX_VALIDATION_PATTERNS)
.{ .name = "adafruitio", .prefix = "aio_", .tier = .api_keys, .min_len = 32, .max_len = 0, .charset = .alphanumeric }
```

**Files to modify:** patterns.zig

### Step 3: Add FFI Bindings (10 min)
**Goal:** Define Rust types for Zig FFI communication

Add to `analyzer.rs`:
```rust
#[repr(C)]
pub struct RedactionResult {
    pub output: *mut u8,
    pub output_len: usize,
    pub match_count: u32,
}

extern "C" {
    pub fn scred_redact_text_optimized(text: *const u8, len: usize) -> RedactionResult;
    pub fn scred_free_redaction_result(result: RedactionResult) -> std::os::raw::c_void;
}
```

**Files to modify:** analyzer.rs

### Step 4: Integrate RedactionEngine (15 min)
**Goal:** Call Zig redaction function instead of Rust regex

In `redactor.rs`:
1. Replace `redact_with_regex()` call with Zig FFI call
2. Convert Zig output to Rust RedactionResult
3. Handle memory cleanup

```rust
fn redact_with_zig_ffi(&self, text: &str) -> RedactionResult {
    unsafe {
        let zig_result = scred_redact_text_optimized(text.as_ptr(), text.len());
        let output = String::from_utf8_lossy(
            std::slice::from_raw_parts(zig_result.output, zig_result.output_len)
        ).to_string();
        scred_free_redaction_result(zig_result);
        // Convert to Rust RedactionResult
    }
}
```

**Files to modify:** redactor.rs

### Step 5: Remove Rust Regex (5 min)
**Goal:** Clean up old Rust implementation

- [ ] Remove `regex` from Cargo.toml dependencies
- [ ] Delete 11 hardcoded patterns from redactor.rs
- [ ] Remove regex imports
- [ ] Verify no unused code

**Files to modify:** Cargo.toml, redactor.rs

### Step 6: Testing (10 min)
**Goal:** Verify all functionality preserved

- [ ] `cargo test --lib -p scred-redactor` - All 35 tests pass
- [ ] `cargo test --lib -p scred-pattern-detector` - Pattern tests pass
- [ ] `cargo build --bin scred` - Binary builds
- [ ] Verify redaction output is correct

## Expected Outcomes

✅ **All 280 patterns owned by Zig**
- 26 simple prefix (SIMD fast-path)
- 51 prefix+validation (fast validation path)
- 1 JWT structure (lightweight check)
- 203 true regex patterns (PCRE2/RE2)

✅ **Rust delegates all pattern detection to Zig**
- No duplicate pattern definitions
- No regex dependency
- FFI-based integration

✅ **Test suite maintained**
- All 35 tests pass
- Character-preserving redaction works
- Metadata collection works

✅ **Code quality improved**
- 77% of patterns avoid regex computation
- Cleaner patterns.zig
- No duplicates

## Time Breakdown

| Step | Task | Duration |
|------|------|----------|
| 1 | Remove duplicates | 5 min |
| 2 | Decompose patterns | 20 min |
| 3 | FFI bindings | 10 min |
| 4 | Integration | 15 min |
| 5 | Cleanup | 5 min |
| 6 | Testing | 10 min |
| **Total** | | **65 minutes** |

## Execution Strategy

1. **Safe:** Each step is self-contained and can be tested independently
2. **Reversible:** Git commits at each step allow rolling back if issues arise
3. **Verifiable:** Tests confirm functionality at each stage

## What We're NOT Doing

- ❌ Not rewriting the Zig pattern detection logic
- ❌ Not changing SIMPLE_PREFIX_PATTERNS or JWT detection
- ❌ Not replacing PCRE2/RE2 engine
- ❌ Not affecting HTTP/MITM code

## What We ARE Doing

- ✅ Moving patterns from REGEX to PREFIX_VALIDATION
- ✅ Removing duplicate pattern definitions
- ✅ Adding FFI boundary for Rust↔Zig communication
- ✅ Removing Rust regex dependency
- ✅ Improving architecture clarity

---

**Status:** Ready to begin
**Risk Level:** Low (changes are isolated to patterns and FFI layer)
**Estimated Completion:** 1 hour
