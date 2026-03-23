# Phase 6: Unify Detection/Redaction Across All Binaries - Implementation Guide

## 🎯 Goal

Make detection and redaction work consistently across **scred-cli**, **scred-proxy**, and **scred-mitm** by creating a common `ConfigurableEngine` layer.

---

## Current Problem: Three Different Code Paths

| Binary | Detection | Redaction | Tiers | CLI Flags | Env Vars |
|--------|-----------|-----------|-------|-----------|----------|
| scred-cli | ZigAnalyzer | ZigAnalyzer | ❌ | ❌ | ❌ |
| scred-proxy | RedactionEngine | RedactionEngine | ❌ | ❌ | ❌ |
| scred-mitm | RedactionEngine | StreamingRedactor | ✅ | ✅ | ✅ |

**Problem:** Pattern tier support only in scred-mitm. CLI doesn't support env vars or flags.

---

## Solution: ConfigurableEngine

**Create a common layer that wraps detection + redaction + pattern selection:**

```rust
// scred-http/src/configurable_engine.rs (NEW FILE)

pub struct ConfigurableEngine {
    engine: RedactionEngine,
    detect_selector: PatternSelector,
    redact_selector: PatternSelector,
}

impl ConfigurableEngine {
    // Filter detection by selector (log only selected patterns)
    pub fn detect_only(&self, text: &str) -> Vec<DetectionWarning> { ... }

    // Redact text (respecting selector)
    pub fn redact_only(&self, text: &str) -> String { ... }

    // Do both: detect + redact with filtering
    pub fn detect_and_redact(&self, text: &str) -> FilteredRedactionResult { ... }
}
```

---

## Phase 6 Roadmap

### Phase 6a: Create ConfigurableEngine (1-2 hours)

**What:** Create unified detection/redaction wrapper in scred-http

**File:** `crates/scred-http/src/configurable_engine.rs` (NEW)

**Tasks:**
1. Create ConfigurableEngine struct
2. Implement detect_only(), redact_only(), detect_and_redact()
3. Add unit tests (5+ tests)
4. Export from scred-http/lib.rs

**Result:** All binaries can now use ConfigurableEngine

---

### Phase 6b: Migrate scred-cli (4-5 hours)

**What:** Add tier support to scred-cli using ConfigurableEngine

**Steps:**
1. Add `scred-http` dependency to Cargo.toml
2. Parse `--detect`, `--redact`, `--list-tiers` CLI flags
3. Parse `SCRED_DETECT_PATTERNS`, `SCRED_REDACT_PATTERNS` env vars
4. Replace `ZigAnalyzer` with `ConfigurableEngine`
5. Update tests

**Result:** scred-cli now has feature parity with scred-mitm

---

### Phase 6c: Migrate scred-proxy (4-5 hours)

**What:** Add tier support to scred-proxy using ConfigurableEngine

**Steps:**
1. Add `--detect`, `--redact`, `--list-tiers` CLI flags
2. Parse `SCRED_DETECT_PATTERNS`, `SCRED_REDACT_PATTERNS` env vars
3. Replace raw `engine.redact()` with `ConfigurableEngine`
4. Add tests

**Result:** scred-proxy now has feature parity with scred-mitm

---

### Phase 6d: Cleanup (1-2 hours, optional)

**What:** Extract common utilities and standardize across binaries

**Steps:**
1. Extract env_detection.rs to scred-http (optional)
2. Standardize logging format across all binaries
3. Final testing and verification

---

## Total Effort

| Phase | Duration | Status |
|-------|----------|--------|
| 6a | 1-2 hours | Ready |
| 6b | 4-5 hours | Depends on 6a |
| 6c | 4-5 hours | Depends on 6a |
| 6d | 1-2 hours | Optional |
| **Total** | **10-14 hours** | Parallelizable after 6a |

---

## Benefits

✅ **One code path** for detection/redaction (not three)
✅ **Pattern tiers in ALL binaries** (not just scred-mitm)
✅ **Consistent CLI flags** across scred-cli, scred-proxy, scred-mitm
✅ **Consistent env vars** across all binaries
✅ **Easier maintenance** (one implementation to fix)
✅ **Better testability** (test once, works everywhere)
✅ **Zero duplication** in core logic

---

## After Phase 6: Feature Parity

**All three binaries will support:**

```bash
# CLI flags
scred --detect CRITICAL --redact CRITICAL,API_KEYS < file.txt
./scred-proxy --detect all --redact CRITICAL
./scred-mitm --list-tiers

# Environment variables
SCRED_DETECT_PATTERNS=CRITICAL scred < file.txt
SCRED_REDACT_PATTERNS=CRITICAL ./scred-proxy
SCRED_DETECT_PATTERNS=all ./scred-mitm
```

---

## Architecture After Phase 6

```
scred-redactor (CORE)
├── 272 patterns with tier metadata
├── RedactionEngine (core redaction)
└── ZigAnalyzer (fast path)

scred-http (COMMON - NEW ConfigurableEngine)
├── PatternSelector + PatternTier
├── ConfigurableEngine (unified interface) ← NEW
├── StreamingRedactor
└── (proxy_resolver, host_id, http_parser, etc.)

Applications
├── scred-cli: Uses ConfigurableEngine
├── scred-proxy: Uses ConfigurableEngine
└── scred-mitm: Uses ConfigurableEngine
```

---

## Implementation Order

1. **Phase 6a FIRST** (blocks nothing, enables everything)
   - Create ConfigurableEngine in scred-http
   - ~2 hours

2. **Phase 6b + 6c IN PARALLEL** (both depend on 6a)
   - Migrate scred-cli (~4-5 hours)
   - Migrate scred-proxy (~4-5 hours)

3. **Phase 6d OPTIONAL** (nice to have)
   - Cleanup and standardization (~1-2 hours)

---

## Success Criteria

- [ ] ConfigurableEngine compiles without warnings
- [ ] All 175+ tests pass
- [ ] scred-cli supports --detect/--redact flags
- [ ] scred-cli supports SCRED_DETECT_PATTERNS env vars
- [ ] scred-proxy supports --detect/--redact flags
- [ ] scred-proxy supports SCRED_DETECT_PATTERNS env vars
- [ ] Pattern filtering works in all binaries
- [ ] Zero regressions
- [ ] Backward compatible with existing usage

---

## Risk Assessment

- **Risk Level:** LOW
- **Breaking Changes:** NONE (backward compatible)
- **Confidence:** HIGH (building on proven patterns)
- **Effort:** Well-estimated and planned

---

## Quick Start

**To begin Phase 6a:**

1. Read: `PHASE_6_COMMON_REDACTION_ANALYSIS.md` (architecture overview)
2. Read: TODO-014e1aef (detailed Phase 6a plan)
3. Create: `crates/scred-http/src/configurable_engine.rs`
4. Implement: ConfigurableEngine struct + methods
5. Test: Unit tests + integration tests

**Expected completion time:** 2-3 hours for Phase 6a

---

## Files to Create/Modify

### Phase 6a
- **NEW:** `crates/scred-http/src/configurable_engine.rs` (150-200 LOC)
- **MODIFY:** `crates/scred-http/src/lib.rs` (add exports)

### Phase 6b
- **MODIFY:** `crates/scred-cli/Cargo.toml` (add scred-http dep)
- **MODIFY:** `crates/scred-cli/src/main.rs` (+50-100 LOC)
- **MODIFY:** `crates/scred-cli/src/env_mode.rs` (small updates)
- **NEW:** Tests in `crates/scred-cli/tests/`

### Phase 6c
- **MODIFY:** `crates/scred-proxy/src/main.rs` (+50-100 LOC)
- **NEW:** Tests in `crates/scred-proxy/tests/`

---

## Documentation

- `PHASE_6_COMMON_REDACTION_ANALYSIS.md` - Full technical analysis
- TODO-014e1aef - Phase 6a detailed plan
- TODO-4bf06464 - Phase 6b detailed plan
- TODO-1bd2e686 - Phase 6c detailed plan

---

## Questions?

See: `PHASE_6_COMMON_REDACTION_ANALYSIS.md` for:
- Current code distribution analysis
- Detailed architecture breakdown
- Why each phase is needed
- Risk assessment and benefits

