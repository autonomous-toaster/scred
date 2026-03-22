# Pattern Rationalization Status

**Date**: 2026-03-21  
**Objective**: Clean up pattern definitions - Single Source of Truth  
**Status**: IN PROGRESS (Step 1 complete, 4 steps remaining)

---

## ✅ STEP 1: COMPLETE (Committed: d7a5a73)

### Created: `crates/scred-pattern-detector/src/patterns.zig` (374 lines)

**Pattern Consolidation**:
- ✅ 26 SIMPLE_PREFIX_PATTERNS (from TIER1_PATTERNS)
- ✅ 1 JWT_PATTERNS (generic JWT detector)
- ✅ 45 PREFIX_VALIDATION_PATTERNS (from TIER2_PATTERNS)
- ✅ 198 REGEX_PATTERNS (extracted from redactor.rs get_all_patterns())
- ✅ **Total: 270 patterns in single file**

**Struct Definitions**:
- ✅ SimplePrefixPattern { name, prefix }
- ✅ JwtPattern { name }
- ✅ PrefixValidation { name, prefix, min_len, max_len, charset }
- ✅ RegexPattern { name, pattern }
- ✅ Charset enum { alphanumeric, base64, base64url, hex, hex_lowercase, any }

**Pattern Counts**:
- ✅ SIMPLE_PREFIX_COUNT = 26
- ✅ JWT_COUNT = 1
- ✅ PREFIX_VALIDATION_COUNT = 45
- ✅ REGEX_COUNT = 198
- ✅ TOTAL_PATTERNS = 270

---

## ⏳ STEP 2: Create detectors.zig (1-1.5 hours)

### Planned: `crates/scred-pattern-detector/src/detectors.zig` (~250 lines)

**Detection Functions** (to be moved from lib.zig):
- `detect_simple_prefix(input) -> bool` (renamed from detect_tier1)
- `detect_jwt(input) -> bool` (keep as is)
- `detect_prefix_validation(input) -> bool` (renamed from detect_tier2)
- `detect_regex(input, pattern) -> bool` (skeleton - regex engine TBD)
- `detect_all_patterns(input) -> bool` (renamed from detect_all_streaming_patterns)

**Helper Functions**:
- `is_jwt_delimiter(byte) -> bool`
- `extract_jwt_token(input, start) -> []const u8`
- `has_valid_jwt_structure(token) -> bool`
- `is_valid_char_in_charset(byte, charset) -> bool`

---

## ⏳ STEP 3: Update lib.zig (30 min)

### Changes to `crates/scred-pattern-detector/src/lib.zig`:

**Removals**:
- ❌ Remove ALL_PATTERNS (286 lines) - Legacy, overlapping
- ❌ Remove FirstCharLookup - Only used for ALL_PATTERNS
- ❌ Remove Tier1Pattern struct - Replaced by SimplePrefixPattern
- ❌ Remove TIER1_PATTERNS - Moved to patterns.zig
- ❌ Remove Tier2Charset enum - Moved to patterns.zig as Charset
- ❌ Remove Tier2Pattern struct - Replaced by PrefixValidation
- ❌ Remove TIER2_PATTERNS - Moved to patterns.zig
- ❌ Remove detect_tier1() - Moved to detectors.zig
- ❌ Remove detect_tier2() - Moved to detectors.zig
- ❌ Remove JWT helper functions - Moved to detectors.zig
- ❌ Remove detect_all_streaming_patterns() - Moved to detectors.zig

**Additions**:
- ✅ `pub const patterns = @import("patterns.zig");`
- ✅ `pub const detectors = @import("detectors.zig");`
- ✅ Update FFI exports to use new names
- ✅ Re-export public APIs for backward compatibility where possible

**FFI Exports** (Update):
- `scred_detector_simple_prefix(text, len)` (was phase2_tier1)
- `scred_detector_jwt(text, len)` (was phase2_jwt)
- `scred_detector_prefix_validation(text, len)` (was phase2_tier2)
- `scred_detector_all(text, len)` (was phase2_all)
- (NEW) `scred_detector_regex(text, len, pattern)` (skeleton)

---

## ⏳ STEP 4: Update analyzer.rs (30 min)

### Changes to `crates/scred-redactor/src/analyzer.rs`:

**Rust FFI Bindings**:
```rust
extern "C" {
    fn scred_detector_simple_prefix(text: *const u8, len: usize) -> c_int;
    fn scred_detector_jwt(text: *const u8, len: usize) -> c_int;
    fn scred_detector_prefix_validation(text: *const u8, len: usize) -> c_int;
    fn scred_detector_all(text: *const u8, len: usize) -> c_int;
    fn scred_detector_regex(text: *const u8, len: usize, pattern: *const u8, pattern_len: usize) -> c_int;
}

pub struct ZigAnalyzer;

impl ZigAnalyzer {
    pub fn has_simple_prefix_pattern(text: &str) -> bool { ... }
    pub fn has_jwt_pattern(text: &str) -> bool { ... }
    pub fn has_prefix_validation_pattern(text: &str) -> bool { ... }
    pub fn has_all_patterns(text: &str) -> bool { ... }
    pub fn has_regex_pattern(text: &str, pattern: &str) -> bool { ... }
}
```

---

## ⏳ STEP 5: Clean redactor.rs (30 min - 1 hour)

### Changes to `crates/scred-redactor/src/redactor.rs`:

**Removals**:
- ❌ Remove `pub fn get_all_patterns()` (994 lines!)
- ❌ Remove 198 hardcoded PatternDef entries
- ❌ Remove `pub struct PatternDef`
- ❌ Remove `lazy_static` and regex compilation logic

**Result**: 
- redactor.rs becomes purely about redaction
- No pattern knowledge in Rust
- All detection happens in Zig

---

## 📊 Impact Analysis

### Lines of Code Changes

| File | Before | After | Change |
|------|--------|-------|--------|
| lib.zig | 1,170 | ~600 | -570 |
| patterns.zig | 0 | 374 | +374 |
| detectors.zig | 0 | 250 | +250 |
| analyzer.rs | 108 | 130 | +22 |
| redactor.rs | 1,050+ | 50 | -1,000 |
| **TOTAL** | **2,328** | **1,404** | **-924** |

**Net**: 924 lines removed (60% reduction in pattern/detection code)

---

## ✅ Success Criteria

- [ ] All 270 patterns in lib.zig (consolidated)
- [ ] Single source of truth (no duplication)
- [ ] No regex patterns in Rust (all in Zig)
- [ ] All detection logic in Zig (detectors.zig)
- [ ] Redactor is pure redaction (no patterns)
- [ ] All 51+ tests passing
- [ ] Build clean (0 warnings)
- [ ] Net 900+ LOC reduction

---

## 🚀 Timeline

| Step | Task | Status | Time | Total |
|------|------|--------|------|-------|
| 0 | Extract regex patterns | ✅ Done | 30m | 30m |
| 1 | Create patterns.zig | ✅ Done | 30m | 1h |
| 2 | Create detectors.zig | ⏳ Start | 1h | 2h |
| 3 | Update lib.zig | ⏳ Next | 30m | 2.5h |
| 4 | Update analyzer.rs | ⏳ Next | 30m | 3h |
| 5 | Clean redactor.rs | ⏳ Next | 30m | 3.5h |
| 6 | Testing & verification | ⏳ Final | 30m | 4h |

---

## 🎯 Architecture After Rationalization

```
scred-pattern-detector/ (Zig)
├─ lib.zig
│  ├─ Imports patterns.zig + detectors.zig
│  ├─ FFI exports (5 functions)
│  └─ Pattern counts
│
├─ patterns.zig ✅ DONE
│  ├─ SIMPLE_PREFIX_PATTERNS (26)
│  ├─ JWT_PATTERNS (1)
│  ├─ PREFIX_VALIDATION_PATTERNS (45)
│  ├─ REGEX_PATTERNS (198)
│  └─ Struct definitions + Charset enum
│
└─ detectors.zig ⏳ IN PROGRESS
   ├─ detect_simple_prefix()
   ├─ detect_jwt()
   ├─ detect_prefix_validation()
   ├─ detect_regex()
   └─ detect_all_patterns()

scred-redactor/ (Rust)
└─ analyzer.rs
   └─ ZigAnalyzer (FFI wrapper)
      ├─ has_simple_prefix_pattern()
      ├─ has_jwt_pattern()
      ├─ has_prefix_validation_pattern()
      ├─ has_all_patterns()
      └─ has_regex_pattern()
```

---

## 📝 Notes

### Key Decisions

1. **Pattern Consolidation**: Move ALL patterns to Zig (single source)
2. **Regex Patterns in Zig**: Transfer 198 regex patterns from Rust to Zig
3. **Detection in Zig**: All pattern detection logic in Zig, not Rust
4. **Redactor as Filter**: Redactor only applies redaction, doesn't know about patterns
5. **FFI Boundary**: Clean separation between Zig (detection) and Rust (redaction)

### Backward Compatibility

- Old FFI function names kept for one version
- New names in use (phase2_* → detector_*)
- Can be removed after redactor is updated

### Future Extensions

- Add regex engine choice (Oniguruma, PCRE, custom Zig impl)
- GPU acceleration for regex matching
- Pattern versioning / custom patterns API
- Consolidate to best 72 patterns from Phase 2

---

## ⚠️ Known Constraints

- 270 patterns is a lot to test - must verify each category works
- Regex patterns need engine - currently just stored as strings
- Must maintain backward compatibility during transition
- All tests must pass (51+ existing tests)

---

**Next Action**: Create detectors.zig with all detection functions
**Target Completion**: 4 hours total (3.5 hours remaining)
