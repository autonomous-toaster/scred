# Phase 2: Final Assessment & Corrected Plan

## ✅ Assessment Complete

### Key Realization

**The Zig build is WORKING.** Previous concerns were false alarms.

```
✅ cargo build --lib -p scred-pattern-detector
✅ cargo test (all 35 tests pass)
✅ detector_ffi.zig compiles
✅ cargo build --bin scred
```

### Pattern Decomposition Opportunity

patterns.zig already marks 18 patterns as decomposable:

```
1. adafruitio (aio_*) → prefix + 28 alphanumeric
2. age-secret-key → prefix + 58 custom-charset
3. anthropic (sk-ant-*) → prefix + 95
4. apideck (sk_live_*) → prefix + 93
5. apify → 2 prefixes + 36
6. clojars-api-token → prefix + 60
7. contentfulpersonalaccesstoken (CFPAT-*) → prefix + 43
8. databrickstoken-1 (dapi*) → prefix + 32 hex
9. deno (dd[pw]_*) → 2 prefixes + 36
10. dfuse (web_*) → prefix + 32
11. digitaloceanv2 → 3 prefixes + 64 hex
12. doppler-api-token (dp.pt.*) → prefix + 43
13. duffel-api-token → prefix + 43
14. dynatrace-api-token (dt0c01.*) → prefix + 88+64
15. easypost-api-token (EZAK*) → prefix + 54
16. fleetbase (flb_live_*) → prefix + 20
17. github-pat/oauth/user/server → DUPLICATED (already PREFIX_VALIDATION!)
18. gitlab-cicd-job-token → prefix + validation
```

### Pattern Distribution

**Before Refactoring:**
```
26  SIMPLE_PREFIX_PATTERNS
45  PREFIX_VALIDATION_PATTERNS  
220 REGEX_PATTERNS (includes 18 decomposable + 6 duplicates)
───────────────────────────────
291 Total
```

**After Refactoring:**
```
26  SIMPLE_PREFIX_PATTERNS (unchanged)
51  PREFIX_VALIDATION_PATTERNS (+12 decomposed, -6 duplicates)
203 REGEX_PATTERNS (cleaner: -18 decomposed patterns)
───────────────────────────────
280 Total (no duplicates)

Fast-path efficiency: 217/280 (77%) patterns avoid regex!
```

## 📋 Phase 2 Implementation Plan

### Step 1: Audit & Remove Duplicates (5 min)
**Goal:** Ensure no pattern defined in both REGEX and PREFIX_VALIDATION

**Action:**
- Find github patterns (ghp_, gho_, ghu_, ghs_, ghr_) in REGEX_PATTERNS → DELETE
- Find gitlab patterns (glpat-, glcbt-) in REGEX_PATTERNS → DELETE
- Run tests to verify

**Files:** patterns.zig

### Step 2: Move Decomposable Patterns (20 min)
**Goal:** Convert 18 marked patterns from REGEX to PREFIX_VALIDATION

**For each pattern:**
1. Extract prefix (e.g., "aio_" from "\\b(aio\\_[a-zA-Z0-9]{28})\\b")
2. Calculate min_len and max_len from regex quantifiers
3. Infer charset (alphanumeric, hex, base64, base64url, any)
4. Add to PREFIX_VALIDATION_PATTERNS
5. Remove from REGEX_PATTERNS

**Example:**
```zig
// Remove from REGEX_PATTERNS
.{ .name = "adafruitio", .pattern = "\\b(aio\\_[a-zA-Z0-9]{28})\\b", .tier = .api_keys }

// Add to PREFIX_VALIDATION_PATTERNS
.{ .name = "adafruitio", .prefix = "aio_", .tier = .api_keys, .min_len = 32, .max_len = 0, .charset = .alphanumeric }
```

**Files:** patterns.zig

### Step 3: Add FFI Bindings (10 min)
**Goal:** Define Rust FFI types for Zig redaction function

**Add to analyzer.rs:**
```rust
#[repr(C)]
pub struct RedactionResult {
    pub output: *mut u8,
    pub output_len: usize,
    pub match_count: u32,
}

#[link(name = "scred_pattern_detector")]
extern "C" {
    pub fn scred_redact_text_optimized(text: *const u8, len: usize) -> RedactionResult;
    pub fn scred_free_redaction_result(result: RedactionResult) -> std::os::raw::c_void;
}
```

**Files:** analyzer.rs

### Step 4: Integrate RedactionEngine (15 min)
**Goal:** Call Zig FFI instead of Rust regex

**Replace in redactor.rs:**
```rust
// OLD
pub fn redact(&self, text: &str) -> RedactionResult {
    self.redact_with_regex(text)
}

// NEW
pub fn redact(&self, text: &str) -> RedactionResult {
    self.redact_with_zig_ffi(text)
}

fn redact_with_zig_ffi(&self, text: &str) -> RedactionResult {
    unsafe {
        let zig_result = crate::analyzer::scred_redact_text_optimized(
            text.as_ptr(),
            text.len(),
        );
        
        if zig_result.output.is_null() {
            return RedactionResult {
                redacted: text.to_string(),
                matches: Vec::new(),
                warnings: vec![RedactionWarning {
                    pattern_type: "zig-redaction-failed".to_string(),
                    count: 0,
                }],
            };
        }
        
        let output = String::from_utf8_lossy(
            std::slice::from_raw_parts(zig_result.output, zig_result.output_len)
        ).to_string();
        let match_count = zig_result.match_count;
        
        crate::analyzer::scred_free_redaction_result(zig_result);
        
        RedactionResult {
            redacted: output,
            matches: Vec::new(), // TODO: Extract from Zig response if needed
            warnings: vec![RedactionWarning {
                pattern_type: format!("zig-redacted-{}-patterns", match_count),
                count: match_count as usize,
            }],
        }
    }
}
```

**Files:** redactor.rs

### Step 5: Remove Rust Regex (5 min)
**Goal:** Clean up old Rust implementation

**Actions:**
- Remove `regex = "1"` from Cargo.toml (scred-redactor)
- Delete `redact_with_regex()` function from redactor.rs
- Delete 11 hardcoded patterns from redactor.rs
- Remove `use regex::Regex` imports
- Verify no compilation errors

**Files:** Cargo.toml, redactor.rs

### Step 6: Testing (10 min)
**Goal:** Verify all functionality preserved

**Commands:**
```bash
cargo test --lib -p scred-redactor
cargo test --lib -p scred-pattern-detector
cargo build --bin scred
```

**Verification:**
- All 35 tests pass ✓
- No compilation warnings ✓
- Binary builds successfully ✓
- Redaction output correct ✓

## 📊 Expected Results

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| SIMPLE_PREFIX_PATTERNS | 26 | 26 | - |
| PREFIX_VALIDATION_PATTERNS | 45 | 51 | +6 |
| REGEX_PATTERNS | 220 | 203 | -17 |
| Total Patterns | 291 | 280 | -11 (duplicates removed) |
| Non-regex Efficiency | 71/291 (24%) | 77/280 (27%) | +12% |
| Pattern Duplicates | 6 | 0 | 100% removed |

## ⏱️ Timeline

```
Step 1: Remove duplicates      5 min  ████
Step 2: Decompose patterns    20 min  ████████████████████
Step 3: FFI bindings          10 min  ██████████
Step 4: Integration           15 min  ███████████████
Step 5: Cleanup                5 min  ████
Step 6: Testing               10 min  ██████████
────────────────────────────────────────
Total:                         65 min  
```

## 🎯 What Changes & What Doesn't

**What Changes:**
- ✅ 18 patterns moved from REGEX to PREFIX_VALIDATION
- ✅ 6 duplicate patterns removed
- ✅ Rust redactor calls Zig FFI
- ✅ Rust regex dependency removed

**What Stays Same:**
- ❌ SIMPLE_PREFIX_PATTERNS (unchanged)
- ❌ JWT detection (unchanged)
- ❌ Zig detection logic (unchanged)
- ❌ Test suite structure (unchanged)
- ❌ HTTP/MITM code (unchanged)

## 🚀 Execution Checklist

- [ ] Step 1: Remove 6 duplicate patterns
- [ ] Step 2: Decompose 18 patterns from REGEX to PREFIX_VALIDATION
- [ ] Step 3: Add FFI struct and function declarations to analyzer.rs
- [ ] Step 4: Add redact_with_zig_ffi() to RedactionEngine
- [ ] Step 5: Remove regex dependency and old redact_with_regex()
- [ ] Step 6: Run tests and verify all pass
- [ ] Final: Commit with message "Phase 2: Move patterns to Zig FFI, decompose 18 patterns"

## 🔍 Risk Assessment

**Risk Level:** LOW

**Why:**
- Changes are isolated to pattern definitions and FFI layer
- No changes to core detection algorithms
- Each step can be tested independently
- Can revert if issues arise
- 35 existing tests provide safety net

**Mitigation:**
- Git commit after each step
- Run tests after each step
- Keep original Rust regex code until FFI fully integrated

---

**Status:** Ready to execute
**Blocker:** None identified
**Approval:** Proceed with implementation
