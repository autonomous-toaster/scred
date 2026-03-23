# P0 CRITICAL FIXES - TEST-DRIVEN DEVELOPMENT PLAN

## Executive Summary
5 Critical issues, 5-7 hours to fix with TDD. Fix Proxy first (4 issues), then MITM review (1 issue).

---

## DISCOVERY

### Q1: StreamingRedactor vs ConfigurableEngine
**FINDING**: 
- `StreamingRedactor::with_defaults()` - NO selector support
- `ConfigurableEngine` - HAS selector support (new, with_defaults, set_redact_selector, set_detect_selector)

**IMPLICATION**: 
Proxy should use `ConfigurableEngine` (or CLI's pattern), NOT `StreamingRedactor::with_defaults()`

### Q2: Current Proxy Architecture
```
handle_connection() {
  RedactionEngine::new()  ← basic, no selectors
  StreamingRedactor::with_defaults(engine) ← ignores selectors!
  stream_request_to_upstream(&redactor, ...)
}
```

CLI Architecture (CORRECT):
```
ConfigurableEngine::new(engine, detect_selector, redact_selector)
engine.detect_and_redact(text) ← uses selectors
```

---

## FIX STRATEGY

### ISSUE #1: Proxy Doesn't Use Pattern Selectors
**Root Cause**: Using wrong abstraction (StreamingRedactor instead of ConfigurableEngine)

**Solution**: 
1. Create `ConfigurableEngine` with selectors (ALREADY HAS API!)
2. Use it instead of `StreamingRedactor::with_defaults()`
3. Pass selectors from `ProxyConfig` to engine

**Files to Change**:
- `crates/scred-proxy/src/main.rs` - handle_connection()

**TDD Tests**:
```rust
#[test]
fn test_proxy_redact_critical_only() {
  // Config: --redact CRITICAL
  // Request: has both AWS_KEY and API_KEY
  // Assert: only AWS_KEY redacted
}

#[test]
fn test_proxy_detect_selector_unused_no_redaction() {
  // Config: --detect CRITICAL (detect mode)
  // Request: has CRITICAL secret
  // Assert: detected but not redacted
}
```

**Estimate**: 1-2 hours (includes refactoring stream functions)

---

### ISSUE #2: Invalid Selector Silent Fallback (Proxy)
**Root Cause**: `from_env()` uses `.unwrap_or_else()` instead of exiting

**Solution**: 
Match `from_config_file()` error handling: eprintln! + exit(1)

**Files to Change**:
- `crates/scred-proxy/src/main.rs` - ProxyConfig::from_env()

**TDD Tests**:
```rust
#[test]
fn test_invalid_selector_env_exits_with_error() {
  // Env: SCRED_REDACT_PATTERNS=INVALID_TIER
  // Assert: Process exits(1)
  // Assert: stderr contains error message
}

#[test]
fn test_valid_selector_env_succeeds() {
  // Env: SCRED_REDACT_PATTERNS=CRITICAL,API_KEYS
  // Assert: Selector created successfully
}
```

**Estimate**: 0.5 hours (simple error handling change)

---

### ISSUE #3: Environment Variable Precedence Broken
**Root Cause**: Using `.or_else()` which treats absence vs failure identically

**Solution**: 
Load ALL three sources, merge with correct precedence

**Current**:
```rust
ProxyConfig::from_config_file()
  .or_else(|_| ProxyConfig::from_env())?
```

**Proposed**:
```rust
let mut config = ProxyConfig::from_defaults();  // Tier 3: Default
if let Ok(file_cfg) = ProxyConfig::from_config_file() {
  config.merge_from(file_cfg);  // Tier 2: File overrides default
}
config.merge_from_env();  // Tier 1: ENV overrides file
config.merge_from_cli(&args);  // Tier 0: CLI overrides all
```

**Files to Change**:
- `crates/scred-proxy/src/main.rs` - ProxyConfig struct + merge methods

**TDD Tests**:
```rust
#[test]
fn test_precedence_env_overrides_file() {
  // File: detect: [CRITICAL]
  // Env: SCRED_DETECT_PATTERNS=ALL
  // Assert: Uses ALL
}

#[test]
fn test_precedence_cli_overrides_all() {
  // File: redact: [CRITICAL]
  // Env: SCRED_REDACT_PATTERNS=API_KEYS
  // Arg: --redact INFRASTRUCTURE
  // Assert: Uses INFRASTRUCTURE
}
```

**Estimate**: 1-2 hours (includes Config refactoring)

---

### ISSUE #4: Detect Mode Not Logging Secrets
**Root Cause**: `RedactionMode::Detect` only sets `enabled: false`, no actual logging

**Solution**: 
When detect mode + secret found:
1. Create ConfigurableEngine with detect_selector
2. Call `engine.detect_only(text)` to get detected patterns
3. Log detected patterns with selector filtering
4. Return unredacted (enabled: false)

**Files to Change**:
- `crates/scred-proxy/src/main.rs` - handle_connection()

**TDD Tests**:
```rust
#[test]
fn test_detect_mode_logs_secrets() {
  // Mode: Detect
  // Request: has CRITICAL secret
  // Assert: secret logged with pattern name
  // Assert: secret NOT redacted in response
}

#[test]
fn test_detect_mode_uses_selector() {
  // Mode: Detect
  // Selector: CRITICAL
  // Request: has API_KEY secret
  // Assert: API_KEY NOT logged (wrong selector)
}
```

**Estimate**: 1 hour (simple logging + ConfigurableEngine integration)

---

### ISSUE #5: MITM Selector Usage Unknown
**Status**: Blocked on code review

**Action**: 
1. Read scred-mitm/src/mitm/proxy.rs
2. Determine if it uses selectors
3. If same bug as Proxy, apply same fixes

**Files to Check**: 
- `crates/scred-mitm/src/mitm/proxy.rs`

**Estimate**: 1-2 hours (review + possible fixes)

---

## HIGH-SEVERITY FIXES (P1)

### ISSUE #6: Per-Path Rules Missing Selector Support
**Current**: Only `path_pattern` + `should_redact` (binary)

**Proposed**: Add optional selectors
```rust
struct PathRedactionRule {
    path_pattern: String,
    should_redact: bool,
    detect_selector: Option<PatternSelector>,  // NEW
    redact_selector: Option<PatternSelector>,  // NEW
    reason: Option<String>,
}
```

**Estimate**: 2 hours (config parsing + logic changes)

---

### ISSUE #7: H2C Incomplete
**Current**: Has TODO, unused parameters, no real implementation

**Action**: Either complete or disable (mark as TBD)

**Estimate**: Defer to v1.1

---

### ISSUE #8: MITM Config Mode Silent Fallback
**Location**: scred-mitm/src/mitm/config.rs

**Fix**: Change match default to error case

```rust
// Current
_ => RedactionMode::Redact,

// Fixed
_ => return Err(anyhow!("Invalid mode: '{}'. Valid modes: passive, selective, strict", mode_str)),
```

**Estimate**: 0.5 hours

---

## IMPLEMENTATION SEQUENCE (5-7 hours)

### Phase 1: Issue #2 (0.5h)
Fix invalid selector error handling - simplest, unblocks later tests

### Phase 2: Issue #1 (2h)
Replace StreamingRedactor with ConfigurableEngine - core fix

### Phase 3: Issue #4 (1h)
Implement detect mode logging - depends on Phase 2

### Phase 4: Issue #3 (1.5-2h)
Fix precedence - complex config refactoring

### Phase 5: Issue #5 (1-2h)
MITM review + fixes if needed

### Total: 6-7.5 hours

---

## TEST SUITE STRUCTURE

Create new test file: `crates/scred-proxy/tests/selector_enforcement_tests.rs`

```rust
#[tokio::test]
async fn test_selector_critical_only() { /* ... */ }

#[tokio::test]
async fn test_selector_api_keys_only() { /* ... */ }

#[tokio::test]
async fn test_invalid_selector_exits() { /* ... */ }

#[tokio::test]
async fn test_precedence_env_over_file() { /* ... */ }

#[tokio::test]
async fn test_detect_mode_logs_secrets() { /* ... */ }

#[tokio::test]
async fn test_detect_mode_uses_selector() { /* ... */ }

#[tokio::test]
async fn test_redact_mode_redacts_secrets() { /* ... */ }

#[tokio::test]
async fn test_path_rule_precedence() { /* ... */ }
```

---

## VERIFICATION CHECKLIST

After all fixes:

```
[ ] Pattern selectors parsed from all sources (file, env, CLI)
[ ] Selectors passed to redaction engine
[ ] Only configured patterns redacted/detected
[ ] Invalid selectors exit with error (all sources)
[ ] Precedence: CLI > ENV > File > Default
[ ] Detect mode logs detected secrets
[ ] Redact mode redacts secrets
[ ] Per-path rules work with selectors (if P1 done)
[ ] MITM has same behavior as Proxy
[ ] 354+ existing tests still pass
[ ] All new selector tests pass
```

---

## REFERENCES

- CLI implementation (correct): crates/scred-cli/src/main.rs
- ConfigurableEngine API: crates/scred-http/src/configurable_engine.rs
- PatternSelector: crates/scred-http/src/pattern_selector.rs
- Current Proxy implementation: crates/scred-proxy/src/main.rs (BROKEN)

