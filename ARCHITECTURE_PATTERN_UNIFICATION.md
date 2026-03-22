# Architecture Improvement: Single Source of Truth for Patterns

**Status**: Proposed  
**Priority**: Medium (not blocking Phase 2 completion)  
**Scope**: Post-Phase 2 refactoring  

---

## Problem Statement

Currently, pattern definitions are split across two systems:

### 1. Zig Pattern Detector (lib.zig)
- **286 patterns** (44 real + 242 test) in `ALL_PATTERNS`
- **Prefix-based detection** (fast path)
- Used by: Zig detector, exposed via FFI
- Format: `Pattern { name, prefix, min_len }`

### 2. Rust Redactor (redactor.rs)
- **199 patterns** as hardcoded REGEX
- **Regex-based detection** (comprehensive)
- Used by: `get_all_patterns()` function
- Format: `PatternDef { name, pattern (regex), fastpath }`

### Problem
- **Duplicate pattern definitions** (different formats)
- **No single source of truth** (patterns can drift)
- **Maintenance burden** (change one, update other?)
- **Inconsistent coverage** (Zig has 286, Redactor has 199)
- **Redactor doesn't use Zig's exported patterns** (via FFI)

---

## Current Architecture

```
┌────────────────────────────────────┐
│   Pattern Detector (Zig)           │
│   - ALL_PATTERNS[286]              │
│   - Prefix-based                   │
│   - Exported via FFI               │
└────────────────────────────────────┘

┌────────────────────────────────────┐
│   Redactor (Rust)                  │
│   - get_all_patterns()[199]        │
│   - Regex-based                    │
│   - Hardcoded (not using Zig)      │
└────────────────────────────────────┘

❌ NO CONNECTION - Two separate systems
```

---

## Proposed Solution

### Option A: Unified Pattern Store in Zig (RECOMMENDED)

```
┌────────────────────────────────────────────┐
│   Pattern Store (Zig)                      │
│   - Single source of truth                 │
│   - Both prefix AND regex patterns         │
│   - Exported via FFI                       │
└────────────────────────────────────────────┘
         ↙                            ↘
┌────────────────────┐    ┌────────────────────┐
│ Detector (Zig)     │    │ Redactor (Rust)    │
│ - Fast path        │    │ - Comprehensive    │
│ - Prefix-based     │    │ - Uses Zig FFI     │
└────────────────────┘    └────────────────────┘

✅ SINGLE SOURCE OF TRUTH
```

**Implementation Steps**:
1. Add regex pattern string to Zig `Pattern` struct
2. Export `scred_detector_get_all_patterns()` FFI function
3. Create Rust wrapper: `ZigPatternProvider`
4. Refactor redactor.rs `get_all_patterns()` to use Zig patterns via FFI
5. Remove hardcoded patterns from redactor.rs

**Benefits**:
- Single source of truth (Zig)
- Patterns stay synchronized
- Easy to add/modify patterns (one place)
- FFI provides clean boundary

### Option B: Separate But Synchronized

Keep them separate but add validation:
```
- Zig patterns (prefix) in lib.zig
- Redactor patterns (regex) in redactor.rs
- Add sync check: cargo test --features validate-patterns
- Validates that all Redactor patterns have Zig counterparts
```

**Benefits**:
- Minimal refactoring
- Each system optimized for its use case
- Gradual migration path

**Drawbacks**:
- Still requires manual synchronization
- Risk of drift

---

## Recommendation

**Go with Option A** (Unified Pattern Store in Zig):

1. **Centralize** pattern definitions in Zig
2. **Export** all patterns via FFI (both prefix + regex)
3. **Use** from Rust via FFI (redactor consumes from Zig)
4. **Maintain** single source of truth
5. **Simplify** future pattern additions

---

## Implementation Complexity

| Task | Complexity | Time |
|------|-----------|------|
| Add regex to Zig Pattern struct | Low | 30m |
| Export pattern list via FFI | Medium | 1h |
| Create Rust wrapper | Low | 30m |
| Refactor redactor.rs | Medium | 1.5h |
| Remove hardcoded patterns | Low | 30m |
| Integration testing | Medium | 1h |
| **Total** | **Medium** | **~5h** |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| FFI performance for 199 patterns | Cache in Rust after first load |
| Complexity in Zig pattern struct | Clear documentation + examples |
| Breaking change to FFI | Version the FFI, maintain backward compatibility |
| Test failures during refactoring | Comprehensive test suite (51+ tests ready) |

---

## Rollout Plan

**Phase 3A (Proposed)**:
1. Add regex field to Zig patterns
2. Export new FFI function
3. Create Rust wrapper (non-breaking)
4. Tests verify both systems work

**Phase 3B (Proposed)**:
1. Refactor redactor.rs to use Zig patterns
2. Remove hardcoded patterns
3. Validation tests ensure coverage
4. Performance benchmarks

**Phase 3C (Future)**:
1. Consider removing duplicate Zig pattern definitions
2. Consolidate to 199 high-confidence patterns
3. Remove 242 test patterns (move to tests directory)

---

## Success Criteria

- ✅ Single source of truth (Zig)
- ✅ Redactor uses Zig patterns via FFI
- ✅ No hardcoded patterns in redactor.rs
- ✅ All 199 patterns still covered
- ✅ No performance regression
- ✅ Tests verify synchronization
- ✅ Documentation updated

---

## Notes

### Current State
- Zig: 286 patterns (44 real, 242 test)
- Redactor: 199 patterns (regex)
- Phase 2: 72 patterns (streaming detector, Tier 1/JWT/Tier2)

### Why This Matters
The question "why scred-redactor has its own get_all_patterns and does not reuse from zig" reveals a fundamental architectural issue. Having two separate pattern sources is a maintenance burden and source of bugs.

### Future Extensibility
Once unified, adding new patterns becomes simple:
1. Add to Zig pattern store (one place)
2. Both systems automatically use it
3. Easy versioning and compatibility

---

## Decision

**Approved for Phase 3** (post-Phase 2):
- Keep Phase 2 complete and stable
- Use Phase 3 to implement unified pattern store
- No breaking changes to Phase 2 code
- Incremental migration to Zig-based patterns

This ensures we maintain momentum on Phase 2 while planning proper architecture for Phase 3.
