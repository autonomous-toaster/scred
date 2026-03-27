# Feature Gap Report: redact_selector Not Implemented

**Date**: March 27, 2026  
**Scope**: Feature gap in CLI pattern filtering (does NOT affect performance optimization)  
**Severity**: Medium (feature incomplete, but not breaking)

---

## Issue Summary

The `--redact` CLI flag is accepted and the code path appears to handle it, but **the redact_selector is never actually applied to filter redactions**. Users cannot selectively choose which pattern tiers to redact.

### Evidence

```bash
$ echo "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE github_token=ghp_abc123" | scred --redact CRITICAL

# Expected: AWS key redacted (CRITICAL), GitHub token NOT redacted (API_KEYS)
# Actual: Both redacted
AWS_ACCESS_KEY_ID=AKIAxxxxxxxxxxxxxxxx
github_token=ghpxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

---

## Root Cause

**File**: `crates/scred-http/src/configurable_engine.rs`  
**Line**: 255 in `selective_unredate()`

```rust
fn selective_unredate(
    &self,
    original: &str,
    fully_redacted: &str,
    _patterns_to_keep_redacted: &[String],  // ← Unused (prefix _)
) -> String {
    // ...
    // Restores ALL redacted patterns without checking selector
    for j in redaction_start..i {
        result_bytes.push(original_bytes[j]);
    }
}
```

The parameter `_patterns_to_keep_redacted` is never used. The function always restores all redactions.

---

## Code Path

1. ✅ CLI `main.rs`: Parses `--redact` flag correctly
2. ✅ CLI `streaming.rs`: Creates ConfigurableEngine with redact_selector
3. ✅ ConfigurableEngine `detect_and_redact()`: Calls `apply_redact_selector()`
4. ✅ ConfigurableEngine `apply_redact_selector()`: Calls `selective_unredate()`
5. ❌ **BUG**: `selective_unredate()` ignores the selector and restores everything

---

## Impact Assessment

### What Works ✅
- `--detect` flag works correctly (filters detection warnings)
- Default behavior works (CRITICAL + API_KEYS redacted)
- Character-preserving redaction works
- Performance optimizations unaffected

### What Doesn't Work ❌
- `--redact CRITICAL` (ignores the restriction)
- `--redact API_KEYS` (ignores the restriction)
- Custom pattern selectors (ignored)
- Any `--redact` flag with patterns not in default set

### User Impact
- Users cannot selectively redact only critical patterns
- Advanced filtering scenarios unsupported
- Current behavior: redacts default set regardless of `--redact` flag

---

## Why This Happened

The `selective_unredate()` function is complex because:

1. **Character-preserving redaction** means `x` characters replace originals
2. **No position mapping** from redacted `x` sequences back to original patterns
3. **Multiple patterns** can overlap, making reverse-mapping hard
4. **Current approach**: "Best-effort" - gave up and restores everything

### The Design Challenge

After full redaction:
- `password=MySecret123` → `password=xxxxxxxxxxxx`
- Cannot tell if `x` sequence was password or API key

Need to track:
- Which pattern detected at which position
- Match that back to redacted sequence
- Only restore patterns not in `patterns_to_keep_redacted`

---

## Solution Approaches

### Option 1: Position Tracking (Recommended)
- Modify redaction engine to return `Vec<(pos, pattern_type)>`
- Use that to identify which `x` sequences to restore
- **Pros**: Clean, correct
- **Cons**: Requires redactor API changes

### Option 2: Pattern-by-Pattern Redaction
- Instead of one full redaction, process each tier separately
- Apply selectors at detection time
- **Pros**: Simpler
- **Cons**: Slower (multiple passes)

### Option 3: Post-Processing with Heuristics
- Use pattern-specific length heuristics to guess which `x` sequence is which
- **Pros**: No API changes
- **Cons**: Error-prone, fragile

### Option 4: Accept Current Behavior
- Document that `--redact` only works on default set
- Mark feature as future work
- **Pros**: No code changes needed
- **Cons**: Limited functionality

---

## Scope vs Autoresearch Goal

**This bug is OUT OF SCOPE for the current autoresearch session**:

- ✅ Autoresearch goal: Optimize stdin throughput
- ✅ This bug does NOT affect throughput
- ✅ Current performance optimizations work correctly
- ❌ This is a feature completeness issue, not a performance issue

**Recommendation**: 
1. Document this feature gap
2. Create separate task to fix (lower priority)
3. Continue autoresearch on performance

---

## Workaround for Users

Until fixed, users can:
1. Use default behavior (redacts CRITICAL + API_KEYS)
2. Use environment variables if available
3. Use external filtering (grep, sed) before/after scred

---

## Checklist for Fix

- [ ] Implement position tracking in redaction engine
- [ ] Modify `selective_unredate()` to use selector
- [ ] Add unit tests for each pattern tier
- [ ] Test mixed inputs (multiple tiers)
- [ ] Verify character-preserving output
- [ ] Update CLI documentation
- [ ] Add integration tests

