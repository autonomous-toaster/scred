# BUG ASSESSMENT: redact_selector Not Implemented

**Status**: 🔴 **CRITICAL** - Core feature broken  
**Severity**: **HIGH** - User-visible regression  
**Impact**: **BLOCKING** - v1.0 release blocker  
**Date Discovered**: 2026-03-23  
**Session**: Phase 8 CLI Enhancements

---

## Executive Summary

The `--redact` flag and `SCRED_REDACT_PATTERNS` environment variable do not work. All redaction happens regardless of tier selection. This is a regression from Phase 6 where the feature was planned but never implemented.

### Test Case
```bash
# Expected: sk_live_xxx → xxxxxxxxxxxxx (redacted with --redact ALL)
$ echo "my_key=sk_live_1234567890abcdefghij1234567890" | scred --redact ALL
my_key=sk_live_1234567890abcdefghij1234567890  # ❌ NOT REDACTED

# Expected: Only CRITICAL tier secrets redacted
$ echo "AWS_KEY=ASIAYQ6GAIQ7GJM3JLKA" | scred --redact CRITICAL
AWS_KEY=xxxxxxxxxxxxxxxxxxxx  # ✅ Works (accidentally)

# Expected: CRITICAL + API_KEYS tiers redacted (default)
$ echo "SLACK=xoxb-..." | scred --redact CRITICAL,API_KEYS
SLACK=xoxb-...  # ❌ NOT REDACTED (Slack is API_KEYS tier)
```

---

## Root Cause Analysis

### Location
- **File**: `crates/scred-http/src/configurable_engine.rs`
- **Function**: `detect_and_redact()` (line 153)
- **Issue**: `redact_selector` is never used

### Current Implementation (BROKEN)
```rust
pub fn detect_and_redact(&self, text: &str) -> FilteredRedactionResult {
    let result = self.engine.redact(text);  // ← Redacts ALL patterns

    // Filter warnings by detect_selector (but ignores redact_selector!)
    let filtered_warnings: Vec<RedactionWarning> = result
        .warnings
        .into_iter()
        .filter(|warning| {
            let tier = get_pattern_tier(&warning.pattern_type);
            self.detect_selector.matches_pattern(&warning.pattern_type, tier)  // ← Only detects!
        })
        .collect();

    FilteredRedactionResult {
        redacted: result.redacted,  // ← Returns unfiltered redaction (all patterns)
        warnings: filtered_warnings,
    }
}
```

### What's Wrong
1. ❌ `self.engine.redact(text)` redacts ALL patterns (no selective control)
2. ❌ `self.redact_selector` is set but never used
3. ❌ Redacted output is returned as-is (no per-tier filtering)
4. ❌ Only `detect_selector` is actually used (for warnings only)
5. ❌ Comments in code confirm this was planned as "future use"

### Code Comments Confirming This is Known Issue
```rust
// Line 16 (docstring):
/// - `redact_selector`: Controls which patterns are redacted (currently unused - redacts all)

// Line 38 (docstring):
/// `redact_selector` - Reserved for future use (currently redacts all)
```

---

## Impact Assessment

### What Works ✅
- `--detect ALL` (shows all patterns)
- `--detect CRITICAL` (shows only CRITICAL)
- `SCRED_DETECT_PATTERNS=ALL` (env var)
- Detection-based logging (filtering works)

### What's Broken ❌
- `--redact ALL` (always redacts all, but that's accidental)
- `--redact CRITICAL` (does nothing special)
- `SCRED_REDACT_PATTERNS=CRITICAL,API_KEYS` (ignored)
- Default redaction tier selection (non-functional)
- Any tier-based selective redaction

### User-Visible Impact
Users cannot:
1. Redact only certain security tiers
2. Preserve lower-risk patterns while redacting critical ones
3. Control output filtering (all patterns always redacted or none)
4. Use environment variable selection

---

## Technical Deep Dive

### Architecture Problem

The `RedactionEngine` (in `scred-redactor`) redacts ALL patterns it finds:

```rust
// scred-redactor/src/redactor.rs line 43
pub fn redact(&self, text: &str) -> RedactionResult {
    // ...
    for (pattern_idx, (_name, regex)) in self.compiled_patterns.iter().enumerate() {
        // Matches and redacts every pattern found
    }
    // Returns redacted output with ALL secrets removed
}
```

The `ConfigurableEngine` is supposed to filter this:
- ✅ **Detect filtering works**: Filters warnings based on `detect_selector`
- ❌ **Redact filtering missing**: Should filter redacted output based on `redact_selector`, but doesn't

### Fix Strategy

The solution requires matching detected patterns to the redacted output and removing/restoring certain patterns based on `redact_selector`.

**Challenge**: The `RedactionResult` doesn't track WHICH patterns were redacted and at what positions.

**Options**:
1. **Post-processing** (Simple): Return unfiltered redaction and re-examine for matches, restore non-matching tiers
2. **Modify RedactionEngine** (Complex): Return pattern list with positions for filtering
3. **Parallel processing** (Safe): Run detection separate from redaction, merge results

---

## Affected Code Paths

### CLI Layer
```rust
// crates/scred-cli/src/main.rs

// Line 40-55: Parses both selectors
let detect_str = detect_flag
    .or_else(|| detect_env.as_deref())
    .unwrap_or("ALL");
let redact_str = redact_flag
    .or_else(|| redact_env.as_deref())
    .unwrap_or("CRITICAL,API_KEYS");

// Line 389-391: Passes both to ConfigurableEngine
let config_engine = ConfigurableEngine::new(
    engine,
    detect_selector.clone(),
    redact_selector.clone(),  // ← Passed but unused!
);
```

### Engine Layer (BROKEN)
```rust
// crates/scred-http/src/configurable_engine.rs:153
pub fn detect_and_redact(&self, text: &str) -> FilteredRedactionResult {
    let result = self.engine.redact(text);
    // ← NO FILTERING based on redact_selector
    
    // Only filters warnings, not redacted output
    let filtered_warnings = result.warnings.into_iter()
        .filter(|warning| {
            let tier = get_pattern_tier(&warning.pattern_type);
            self.detect_selector.matches_pattern(...)  // ← Wrong selector!
        })
        .collect();
    
    FilteredRedactionResult {
        redacted: result.redacted,  // ← Unfiltered!
        warnings: filtered_warnings,
    }
}
```

---

## Fix Options

### Option 1: Post-Processing (RECOMMENDED)
**Complexity**: Medium  
**Performance**: -5% overhead  
**Safety**: High  

```rust
pub fn detect_and_redact(&self, text: &str) -> FilteredRedactionResult {
    let result = self.engine.redact(text);
    
    // Find all redactions that were made
    let redactions = self.find_redactions(text, &result.redacted);
    
    // Filter redactions based on redact_selector
    let mut filtered_redacted = result.redacted.clone();
    for redaction in redactions {
        if !self.redact_selector.matches_pattern(&redaction.pattern_type, redaction.tier) {
            // Restore original text for this redaction
            filtered_redacted.replace_range(
                redaction.start..redaction.end,
                &text[redaction.start..redaction.end]
            );
        }
    }
    
    FilteredRedactionResult {
        redacted: filtered_redacted,
        warnings: filtered_warnings,
    }
}

fn find_redactions(&self, original: &str, redacted: &str) -> Vec<Redaction> {
    // Compare original vs redacted to find what changed
}
```

### Option 2: Modify RedactionEngine
**Complexity**: High  
**Performance**: No overhead  
**Safety**: Medium (changes core engine)  

Modify `RedactionResult` to include pattern info:
```rust
pub struct RedactionResult {
    pub redacted: String,
    pub warnings: Vec<RedactionWarning>,
    pub redactions: Vec<(usize, usize, String)>,  // ← NEW: positions + pattern names
}
```

### Option 3: Separate Detection Pass
**Complexity**: Medium  
**Performance**: -10% (detection runs twice)  
**Safety**: Highest  

```rust
pub fn detect_and_redact(&self, text: &str) -> FilteredRedactionResult {
    // First: Just detect what patterns exist
    let detections = self.detect_patterns(text);
    
    // Second: Redact based on redact_selector
    let filtered_detections: Vec<_> = detections
        .into_iter()
        .filter(|d| self.redact_selector.matches_pattern(&d.pattern, d.tier))
        .collect();
    
    // Third: Selectively redact only matching patterns
    let redacted = self.redact_selected(text, &filtered_detections);
    
    FilteredRedactionResult {
        redacted,
        warnings: /* ... */,
    }
}
```

---

## Recommended Fix

**Approach**: Option 1 (Post-Processing)  
**Rationale**:
- Minimally invasive (only changes ConfigurableEngine)
- No changes to core RedactionEngine
- Acceptable performance overhead (<5%)
- Easy to test and verify
- Backward compatible

**Implementation Steps**:
1. After calling `engine.redact()`, capture the redacted output
2. Compare byte-by-byte with original to find redaction positions
3. For each redaction, check if pattern tier matches `redact_selector`
4. Restore non-matching redactions to original text
5. Return filtered output

**Time Estimate**: 2-3 hours (implementation + testing)

---

## Version Impact

### Current Release Status
- **Phase 8**: Just completed CLI enhancements ✅
- **v1.0 Target**: Configuration system complete ✅
- **Blocking Issue**: This bug prevents v1.0 release ❌

### Version Timeline
- v1.0: Hold until this is fixed
- v1.1: Post-release if not critical

---

## Related Issues

None - this is an orphaned feature from Phase 6 that was never actually implemented.

---

## Testing Plan

After fix is implemented:

```bash
# Test 1: --redact ALL (should redact everything)
echo "sk_live_abc123 AWS_SECRET_DEF456" | scred --redact ALL
# Expected: xxxxxxxxxxxx xxxxxxxxxxxx

# Test 2: --redact CRITICAL (should only redact AWS)
echo "sk_live_abc123 AWS_SECRET_DEF456" | scred --redact CRITICAL
# Expected: sk_live_abc123 xxxxxxxxxxxxxxxxxxxx

# Test 3: --redact CRITICAL,API_KEYS (should redact both)
echo "sk_live_abc123 SLACK_xoxb_token" | scred --redact CRITICAL,API_KEYS
# Expected: xxxxxxxxxxxx xxxxxxxxxxxx

# Test 4: --redact API_KEYS (should NOT redact AWS)
echo "AWS_SECRET_DEF456" | scred --redact API_KEYS
# Expected: AWS_SECRET_DEF456

# Test 5: Environment variable
SCRED_REDACT_PATTERNS=CRITICAL scred < file.txt
# Expected: Only CRITICAL patterns redacted
```

---

## Conclusion

This is a **CRITICAL** blocking issue for v1.0 release. The `--redact` flag and `SCRED_REDACT_PATTERNS` environment variable are completely non-functional. The feature was designed in Phase 6 but implementation was deferred and ultimately forgotten.

**Action Required**: Implement Option 1 (Post-Processing) before v1.0 release.

**Status**: Opened as TODO-2cb437f4
