# Bug Assessment: Multiline Secrets & Env Parser Prefix Loss

## Bug #1: Multiline Secrets Not Detected ❌

### Description
When a secret spans multiple lines, it's not detected or redacted.

### Test Cases

```bash
# Works (single line)
AWS_KEY=AKIA1234567890ABCDEF
→ AWS_KEY=AKIAxxxxxxxxxxxxxxxx ✅

# Broken (multiline)
AWS_KEY=AKIA123456
7890ABCDEF
→ AWS_KEY=AKIA123456
  7890ABCDEF ❌ NOT REDACTED

# Broken (continuation)
AWS_KEY=AKIA123456\
7890ABCDEF
→ AWS_KEY=AKIA123456\n7890ABCDEF ❌ NOT REDACTED

# Broken (real multiline like JSON)
API_TOKEN=ghp_0123456789ABCDEFGHIJ
0123456789ABCDEFGHIJ
→ API_TOKEN=xxxxxxxxxxxxxxxxxxxxxxxx
  0123456789ABCDEFGHIJ
  (partial redaction, second line not touched) ❌
```

### Root Cause
The env parser processes input line-by-line. The RedactionEngine works on single lines only:
1. Line 1: `API_TOKEN=ghp_0123456789ABCDEFGHIJ` 
   - Pattern: `ghp_[a-zA-Z0-9_]{36,}` - requires 36+ chars after prefix
   - Actual: `ghp_0123456789ABCDEFGHIJ` is only 27 chars total (23 after prefix)
   - Result: NO MATCH ❌

2. Line 2: `0123456789ABCDEFGHIJ` 
   - No prefix, not a valid pattern by itself
   - Result: NOT PROCESSED ❌

### Impact
- AWS/GitHub tokens split across logs/config files not caught
- PEM keys spanning multiple lines not caught
- JSON/multiline values not caught
- **Security risk**: Logs with multiline secrets will leak tokens

---

## Bug #2: Env Parser Loses Prefix When Key is Secret Variable ❌

### Description
When a variable name contains a secret keyword (e.g., `AWS_SECRET_ACCESS_KEY`), the env parser replaces the ENTIRE value with x's, losing the prefix that should be preserved.

### Test Cases

```bash
# Works: Non-secret variable name
TEST_VAR=AKIA1234567890ABCDEF
→ TEST_VAR=AKIAxxxxxxxxxxxxxxxx ✅
  (Prefix AKIA preserved correctly)

# Broken: Secret variable name
AWS_SECRET_ACCESS_KEY=AKIA1234567890ABCDEF
→ AWS_SECRET_ACCESS_KEY=xxxxxxxxxxxxxxxxxxxx ❌
  (Prefix AKIA lost! Should be: AKIA...)

# Broken: Secret variable name with GitHub token
API_TOKEN=ghp_0123456789ABCDEFGHIJ0123456789ABCDEFGHIJ
→ API_TOKEN=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ❌
  (Should preserve: ghp_xxx...)
```

### Root Cause
**File**: `crates/scred-cli/src/env_mode.rs` Lines 126-127

```rust
if is_secret_variable(key) {
    // Redact entire value  ← BUG: Doesn't use regex redactor!
    result.push_str(&"x".repeat(value.len()));
} else {
    // Scan value for patterns using ConfigurableEngine
    result.push_str(&config_engine.redact_only(value));  ← Uses redactor
}
```

**Problem**: 
- When key contains secret keywords, hardcoded replacement ignores the redactor
- Loses all pattern detection benefits (prefix preservation, selective redaction)
- Inconsistent behavior (same token different redaction based on variable name!)

### Impact
- Inconsistent redaction behavior confuses users
- Prefixes lost = harder to correlate logs
- Defeats purpose of character-preservation feature
- Inconsistent with `--text-mode` behavior (preserves prefix)

---

## Bug #3: Code Duplication ❌

### Description
The env_mode.rs has two nearly-identical functions that should be one.

### Current Code

```rust
// Two separate implementations:
pub fn redact_env_line(line: &str, redact_fn: impl Fn(&str) -> String) -> String { ... }
pub fn redact_env_line_configurable(line: &str, config_engine: &ConfigurableEngine) -> String { ... }
```

**Lines**: ~40 lines duplicated (lines 38-87 vs 89-131)

**Differences**: Only the redaction function:
- `redact_env_line`: Takes generic `redact_fn` closure
- `redact_env_line_configurable`: Uses `config_engine.redact_only()`

### Root Cause
Lack of abstraction - should use a single function with a trait/closure parameter.

### Impact
- Maintenance nightmare (bug fix needed in 2 places)
- Inconsistent behavior possible
- Harder to add new features (must edit 2 functions)

---

## Design Problem: Env Parser Should Be Transparent Layer

### Current Architecture (Problematic)

```
ENV_PARSER (env_mode.rs)
│
├─ Special handling for secret variables ← WRONG!
├─ Hardcoded x-replacement ← WRONG!
├─ Line-by-line processing ← LIMITS MULTILINE
└─ Duplicated logic ← CODE SMELL

REDACTOR (redactor.rs)
│
├─ Pattern detection (regex)
├─ Prefix preservation (length-aware)
└─ Character preservation
```

**Problem**: ENV_PARSER bypasses REDACTOR for secret variables, breaking features!

### Correct Architecture (Should Be)

```
ENV_PARSER (env_mode.rs) - THIN WRAPPER
│
├─ Parse KEY=VALUE format ← Only this
└─ Call REDACTOR for value ← Always use it

REDACTOR (redactor.rs) - SINGLE SOURCE OF TRUTH
│
├─ Pattern detection (regex)
├─ Prefix preservation (length-aware)
├─ Character preservation
└─ ALL redaction logic here
```

**Benefit**: 
- Consistent behavior everywhere
- Single place to fix bugs
- Easy to test
- No code duplication

---

## Detailed Fixes Required

### Fix #1: Eliminate Code Duplication ✅ EASY

**Action**: Merge `redact_env_line` and `redact_env_line_configurable` into one function

```rust
/// Generic env line parser - works with any redactor
pub fn redact_env_line_generic<F: Fn(&str) -> String>(
    line: &str,
    redact_fn: F,
) -> String {
    // Shared logic for parsing KEY=VALUE
    // Always delegate value redaction to redact_fn
}

// Convenience wrapper for ConfigurableEngine
pub fn redact_env_line_configurable(
    line: &str,
    config_engine: &ConfigurableEngine,
) -> String {
    redact_env_line_generic(line, |v| config_engine.redact_only(v))
}

// Convenience wrapper for RedactionEngine
pub fn redact_env_line(
    line: &str,
    engine: &RedactionEngine,
) -> String {
    redact_env_line_generic(line, |v| engine.redact(v).redacted)
}
```

**Effort**: 30 minutes  
**Risk**: Low (pure refactoring, same behavior)

---

### Fix #2: Remove Hardcoded Secret Variable Replacement ✅ CRITICAL

**Action**: Always use redactor, even for secret variables

```rust
// BEFORE (WRONG):
if is_secret_variable(key) {
    result.push_str(&"x".repeat(value.len()));  // ❌ Loses prefix
} else {
    result.push_str(&config_engine.redact_only(value));
}

// AFTER (CORRECT):
// Always use redactor - it knows how to preserve prefixes
result.push_str(&config_engine.redact_only(value));
```

**Why**: The redactor handles EVERYTHING correctly:
- Preserves prefixes (ghp_ → ghp_xxx...)
- Detects patterns (finds AKIA in value)
- Respects redact_selector (--redact CRITICAL)
- Consistent with --text-mode

**Effort**: 5 minutes  
**Risk**: Very Low (actually fixes bugs)

**Test**: Verify that `AWS_SECRET_ACCESS_KEY=AKIA...` now shows `AWS_SECRET_ACCESS_KEY=AKIA...xxx`

---

### Fix #3: Handle Multiline Secrets 🟡 COMPLEX

**Problem**: Current line-by-line processing can't handle multiline values.

**Possible Solutions**:

#### Option A: Streaming/Buffered Parsing (Recommended)
- Read environment format with continuation detection
- Handle shell-style line continuations (`\`)
- Buffer incomplete values across lines

**Effort**: 2-3 hours  
**Complexity**: Medium  
**Benefit**: Proper support for continuation-based formats

#### Option B: Accept as Limitation (Short-term)
- Document that multiline secrets not supported
- Works fine for 99% of cases (most secrets on one line)
- Can revisit in v1.1

**Effort**: 5 minutes  
**Complexity**: None  
**Benefit**: Unblocks v1.0 (current bugs won't be worse)

#### Option C: Hybrid - Greedy Pattern Matching
- Allow patterns to span lines if they look like continuation
- Risky: May match false positives

**Not Recommended**

### Recommendation for v1.0
**Do Fix #1 and #2 (easy, high impact)** ✅  
**Document limitation for Fix #3** (defer to v1.1)

---

## Summary of Issues

| Issue | Severity | Type | Impact | Effort |
|-------|----------|------|--------|--------|
| Multiline secrets | MEDIUM | Bug | Secrets spanning lines not caught | High |
| Prefix loss on secret vars | HIGH | Bug | Inconsistent behavior, data loss | Very Low |
| Code duplication | MEDIUM | Maintenance | Hard to maintain | Low |
| Architecture | MEDIUM | Design | Should be thin wrapper only | Medium |

**Recommended Action**: Fix #1 + #2 for v1.0, defer #3 to v1.1

---

## Proposed Implementation Plan

### Phase 1: Fix #2 (Prefix Loss) - 5 minutes ✅

```rust
// env_mode.rs - line 126-131
// Remove the special case:
- if is_secret_variable(key) {
-     result.push_str(&"x".repeat(value.len()));
- } else {
-     result.push_str(&config_engine.redact_only(value));
- }
+ // Always use redactor - it handles prefixes correctly
+ result.push_str(&config_engine.redact_only(value));
```

### Phase 2: Fix #1 (Code Duplication) - 30 minutes ✅

```rust
// Create single implementation
pub fn redact_env_line_generic<F: Fn(&str) -> String>(
    line: &str,
    redact_fn: F,
) -> String {
    // Shared parsing logic (from current redact_env_line_configurable)
}

// Remove duplicate code
// Keep convenience wrappers pointing to generic version
```

### Phase 3: Fix #3 (Multiline) - Defer to v1.1 📝

- Add test showing limitation
- Document in README
- Create TODO for v1.1

---

## Test Cases to Add

```rust
#[test]
fn test_secret_var_preserves_prefix() {
    // AWS_SECRET_ACCESS_KEY=AKIA... should preserve AKIA prefix
    let result = redact_env_line_configurable(
        "AWS_SECRET_ACCESS_KEY=AKIA1234567890ABCDEF",
        &engine
    );
    assert!(result.contains("AKIA"));  // Prefix preserved!
    assert!(result.contains("x"));     // And redacted!
}

#[test]
fn test_consistency_secret_vs_non_secret_var() {
    // Same token, different variable names, should have same redaction
    let secret_var_result = redact_env_line_configurable(
        "SECRET_KEY=AKIA1234567890ABCDEF",
        &engine
    );
    let normal_var_result = redact_env_line_configurable(
        "MY_VAR=AKIA1234567890ABCDEF",
        &engine
    );
    
    // Both should preserve AKIA prefix
    assert!(secret_var_result.contains("AKIA"));
    assert!(normal_var_result.contains("AKIA"));
}

#[test]
#[ignore]  // Defer to v1.1
fn test_multiline_secret_not_yet_supported() {
    // Document that this doesn't work in v1.0
    let result = redact_env_line_configurable(
        "API_TOKEN=ghp_0123456789ABCDEFGHIJ\n0123456789ABCDEFGHIJ",
        &engine
    );
    // Currently fails to detect multiline tokens
    // TODO: v1.1 enhancement
}
```

---

## Conclusion

**Status**: 2 bugs + 1 design issue identified

**Severity**:
- 🔴 BUG #2 (prefix loss) = HIGH - inconsistent behavior, data loss
- 🟡 BUG #1 (multiline) = MEDIUM - edge case but security relevant
- 🟡 BUG #3 (code duplication) = MEDIUM - maintenance risk

**Recommendation for v1.0**:
1. ✅ Fix Bug #2 immediately (5 min, high impact)
2. ✅ Fix Bug #3 immediately (30 min, high impact)
3. 📝 Document Bug #1 as v1.1 enhancement (defer)

**After fixes**: 
- Env parser becomes thin wrapper (correct architecture)
- Consistent behavior everywhere
- No code duplication
- v1.0 ready with known limitation on multiline (acceptable)
