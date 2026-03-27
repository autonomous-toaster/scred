# Compiler Warnings Analysis & Fix Plan

**Date**: March 27, 2026  
**Focus**: Dead code vs incomplete implementation  
**Risk Level**: Low (all tests passing, no functional impact yet)

---

## Critical Warnings (Hot Path - Need Fix)

### 1. Incomplete Selector Logic in `process_chunk_in_place()`

**File**: `crates/scred-redactor/src/streaming.rs:334`

**Code**:
```rust
let mut output = redacted_str.clone();  // ❌ declared mut but not modified
if let Some(selector) = &self.selector {  // ❌ selector never used
    for m in &detection.matches {
        if m.start >= output_end {
            continue;
        }
        // ❌ LOOP BODY IS EMPTY - No logic implemented
    }
}
```

**Problem**: 
- Selector feature is declared but not implemented
- Empty loop body suggests unfinished work
- `output` is cloned but never modified
- `selector` is bound but never referenced

**Status**: In hot path (`process_chunk_in_place` is streaming method)

**Fix Options**:

**Option A**: Complete the Implementation (Proper)
```rust
// Apply selective filtering if selector exists
let mut output = redacted_str.clone();
if let Some(selector) = &self.selector {
    for m in &detection.matches {
        if m.start >= output_end {
            continue; // In lookahead, will be processed next iteration
        }
        
        // Check if this pattern should be redacted based on selector
        let should_redact = match selector {
            crate::pattern_selector::PatternSelector::All => true,
            crate::pattern_selector::PatternSelector::Patterns(patterns) => {
                patterns.contains(&m.pattern_type)
            }
            crate::pattern_selector::PatternSelector::Tiers(tiers) => {
                // Check pattern tier against allowed tiers
                true // Simplified for now
            }
            _ => true,
        };
        
        if !should_redact {
            // Un-redact: replace redacted text with original
            let start = m.position;
            let end = start + m.redacted_text.len();
            
            if start < output.len() && end <= output.len() {
                output.replace_range(start..end, &m.original_text);
            }
        }
    }
}
```

**Option B**: Remove Selector Logic (Simplify)
```rust
// Selector not yet implemented in process_chunk_in_place()
// For now, always apply full redaction
let output = redacted_str.clone();  // No selector filtering
```

**Option C**: Mark as TODO (Document Intent)
```rust
// TODO: Implement selective filtering
// For now, always apply full redaction to match process_chunk()
let output = redacted_str.clone();
```

**Recommendation**: Option A (Complete implementation)
- Consistency with `process_chunk()` which already has selector logic
- Enables selective pattern redaction in streaming mode
- Takes ~15 minutes to implement correctly

---

## Non-Critical Warnings (Safe to Clean)

### 2. Unused CLI Functions

**File**: `crates/scred-cli/src/main.rs`

```rust
// Line 412: Never called
fn process_text_chunk_and_stream(...)

// Line 428: Never called  
fn process_env_chunk_and_stream(...)
```

**Status**: Experimental/legacy code paths  
**Impact**: None (not in any execution path)  
**Action**: Remove (clean up)

---

### 3. Unused env_mode Constants & Functions

**File**: `crates/scred-cli/src/env_mode.rs`

```rust
// Line 17: Unused constant
const SECRET_KEYWORDS: &[&str] = &[...];

// Line 34: Unused function
pub fn is_secret_variable(name: &str) -> bool {...}

// Line 103: Unused function
pub fn redact_env_line(line: &str, ...) -> String {...}
```

**Status**: Feature stub (environment variable detection)  
**Impact**: None (not used)  
**Action**: Remove OR document as future feature

---

### 4. Unused MITM Functions

**File**: `crates/scred-mitm/src/mitm/h2_mitm_handler.rs`

```rust
// These functions were likely refactored:
fn handle_h2_connection_bidirectional(...) - unused
fn handle_h2_with_upstream(...) - unused
fn handle_h2_with_frame_forwarding(...) - unused
fn send_h2_error_response(...) - unused
fn encode_h2_headers_frame(...) - unused
fn encode_h2_data_frame(...) - unused
```

**Status**: Refactored/deprecated code  
**Impact**: None (active code path doesn't use these)  
**Action**: Remove (clean up)

---

## Warning Categories Summary

| Type | Count | Location | Risk | Action |
|------|-------|----------|------|--------|
| Incomplete implementation | 2 | streaming.rs:334 | MEDIUM | Complete or document |
| Unused CLI functions | 2 | main.rs | LOW | Remove |
| Unused env functions | 3 | env_mode.rs | LOW | Remove |
| Unused MITM functions | 6 | h2_mitm_handler.rs | LOW | Remove |
| **TOTAL** | **13** | | | |

---

## Recommended Action Plan

### Step 1: Fix Critical (Hot Path) ✅
**Effort**: 15 minutes
- Complete selector logic in `process_chunk_in_place()`
- Test with selector-based redaction

### Step 2: Clean Safe Code (No Risk) ✅
**Effort**: 30 minutes
- Remove 2 unused CLI functions
- Remove 3 unused env_mode functions
- Remove 6 unused MITM functions
- Update imports if needed

### Step 3: Verify Zero Regressions ✅
**Effort**: 10 minutes
```bash
cargo test --release
# Should show: 368+ tests passing
```

### Total Estimated Time: 55 minutes

---

## Impact Assessment

**Current State**: 
- ✅ All 368+ tests passing
- ✅ No functional impact from warnings
- ❌ Code quality: ~13 warnings

**After Fixes**:
- ✅ All 368+ tests passing
- ✅ Complete implementation (selector working)
- ✅ Clean codebase (no dead code)
- ✅ 0 warnings (or very few)

**Regression Risk**: Very Low
- Only removing/completing code, not changing behavior
- All changes can be tested immediately

---

## Decision Tree

```
Selector logic (line 334)?
├─ Complete it (recommended)
│  ├─ Implement pattern filtering
│  ├─ Test with selector modes
│  └─ Ships with full feature
│
├─ Remove it (if not needed)
│  └─ Simplify to no-op
│
└─ Document as TODO (if planning future)
   └─ Mark clearly with #[allow(dead_code)]

Dead code (13 functions)?
├─ Remove all (recommended)
│  ├─ Clean CLI functions
│  ├─ Clean env stubs
│  ├─ Clean MITM experiments
│  └─ Result: Clean codebase
│
└─ Keep some? (only if...unclear)
   └─ Document purpose with #[allow]
```

---

## Next Steps

1. Decide: Complete selector logic or simplify?
2. Run the fixes (takes ~1 hour total)
3. Verify tests still pass
4. Commit with summary

**Recommendation**: Complete selector logic (Option A) + remove dead code.
This leaves the codebase clean, complete, and warning-free.

