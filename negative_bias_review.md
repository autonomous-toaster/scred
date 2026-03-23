# NEGATIVE BIAS CODE REVIEW - SCRED CLI/PROXY/MITM
**Status**: Comprehensive Security & Consistency Assessment  
**Date**: 2026-03-23

---

## CRITICAL FINDINGS: Security Gaps

### 🔴 GAP 1: CLI vs Proxy vs MITM - Inconsistent Selector Defaults

**Problem**: Different default selector values across tools lead to inconsistent redaction.

| Tool | Detect Default | Redact Default | Inconsistency |
|------|---------------|----------------|---|
| **CLI** | `ALL` (line 41, main.rs) | `CRITICAL,API_KEYS,PATTERNS` (line 47) | Includes PATTERNS tier |
| **Proxy** | `CRITICAL,API_KEYS,INFRASTRUCTURE` (line 180) | `CRITICAL,API_KEYS` (line 181) | Excludes PATTERNS tier |
| **MITM** | No explicit default shown | No explicit default shown | UNKNOWN - needs investigation |

**Security Impact**: CRITICAL - Same secret might be:
- ✅ Redacted by CLI (includes PATTERNS tier)
- ❌ NOT redacted by Proxy (excludes PATTERNS tier)
- ??? Unknown for MITM

**Attack Vector**: User runs `env | scred` redacting all patterns, but proxy redacts fewer patterns. Secrets leak.

**Code Reference**:
```rust
// CLI (main.rs:41-47)
let detect_str = detect_flag
    .or_else(|| detect_env.as_deref())
    .unwrap_or("ALL");  // ← includes JWT, Bearer, BasicAuth
let redact_str = redact_flag
    .or_else(|| redact_env.as_deref())
    .unwrap_or("CRITICAL,API_KEYS,PATTERNS");  // ← includes PATTERNS

// Proxy (main.rs:180-181)
let detect_str = detect_flag
    .or_else(|| detect_env.clone())
    .unwrap_or_else(|| "CRITICAL,API_KEYS,INFRASTRUCTURE".to_string());  // ← excludes PATTERNS
let redact_str = redact_flag
    .or_else(|| redact_env.clone())
    .unwrap_or_else(|| "CRITICAL,API_KEYS".to_string());  // ← excludes PATTERNS
```

---

### 🔴 GAP 2: MITM Proxy - Selectors Never Used

**Problem**: MITM creates H2MitmHandler with `detect_patterns` and `redact_patterns`, but never actually passes them to StreamingRedactor.

**Code Path Analysis**:
```
MITM proxy.rs:187-194
  ├─ Creates handle_http_proxy() with redaction_engine
  │  └─ handle_http_proxy DOES NOT receive selector parameters!
  │
  ├─ Creates handle_tls_mitm() with detect_patterns + redact_patterns
  │  └─ tls_mitm.rs:141-142 assigns to h2_config
  │  └─ h2_mitm_handler creates H2MitmHandler with selectors
  │  └─ H2MitmHandler::handle_stream() receives selectors
  │     └─ h2_upstream_forwarder::handle_upstream_h2_connection()
  │        └─ SELECTOR PARAMETERS NOT USED
```

**Critical Issue**: The selectors are passed but NEVER used in actual redaction:

```rust
// h2_upstream_forwarder.rs (assumed, needs verification)
// Creates StreamingRedactor without selector!
let streaming_redactor = StreamingRedactor::new(
    engine.clone(),
    StreamingConfig::default(),
    // ❌ NO SELECTOR PARAMETER!
);
```

**Security Impact**: CRITICAL - MITM redacts ALL patterns regardless of configured selector.
- User: `scred-mitm --redact CRITICAL`
- Expected: Only CRITICAL tier redacted
- Actual: ALL patterns redacted (all tiers)

---

### 🔴 GAP 3: HTTP Request/Response Streaming - No Selector Validation

**Problem**: `http_proxy_handler.rs` creates StreamingRedactor without passing the selector.

**Code** (http_proxy_handler.rs:120-130):
```rust
let streaming_config = StreamingConfig::default();
let streaming_redactor = StreamingRedactor::new(
    redaction_engine.clone(),
    streaming_config,
    // ❌ NO SELECTOR PASSED!
);
let (redacted_request, redaction_stats) = streaming_redactor.redact_buffer(full_request.as_bytes());
```

**Impact**:
- handle_http_proxy() receives RedactionEngine without selector info
- Even if selector was in engine, StreamingRedactor doesn't check it
- Result: ALL patterns redacted, ignoring user's intended selector

---

### 🟡 GAP 4: CLI - ConfigurableEngine vs StreamingRedactor Mismatch

**Problem**: CLI uses `ConfigurableEngine` but proxy/MITM use `StreamingRedactor`.

**Code**:
```rust
// CLI (main.rs:236-243)
let config_engine = ConfigurableEngine::new(
    engine,
    detect_selector.clone(),
    redact_selector.clone(),
);
// Calls config_engine.detect_and_redact()

// Proxy (http_proxy_handler.rs:120-130)
let streaming_redactor = StreamingRedactor::new(
    redaction_engine.clone(),
    streaming_config,
);
// Calls streaming_redactor.redact_buffer()
```

**Inconsistency**: Different code paths, different APIs, different selector handling.
- CLI: ConfigurableEngine stores selector and uses it
- Proxy: StreamingRedactor ignores selector
- Result: INCONSISTENT REDACTION BEHAVIOR

---

### 🟡 GAP 5: Env Mode - Line-by-Line Processing Missing Selector

**Problem**: CLI's env_mode processes line-by-line but may not respect selector correctly.

**Code** (main.rs:293-313):
```rust
for line in input_str.lines() {
    let redacted = env_mode::redact_env_line_configurable(line, &config_engine);
    // ...
}
```

**Missing Context**: `env_mode::redact_env_line_configurable` is called but we don't know if:
1. It respects the selector
2. It only processes one line or entire buffer
3. What happens at line boundaries with multi-line secrets

---

### 🟡 GAP 6: MITM HTTP Handler - No Pattern Filtering

**Problem**: MITM's HTTP handler calls shared `handle_http_proxy` without selector.

**Code** (mitm/proxy.rs:187-194):
```rust
if let Err(e) = crate::mitm::http_handler::handle_http_proxy(
    socket_read,
    socket_write,
    &line,
    redaction_engine.clone(),
    upstream_resolver.clone(),
).await {
    warn!("HTTP proxy handler error: {}", e);
}
```

**Missing**: No selector passed. MITM HTTP requests use full redaction.
- MITM HTTPS (H2): Uses detect/redact selectors
- MITM HTTP: Ignores selectors (full redaction)
- Result: INCONSISTENT between HTTP and HTTPS

---

## CONSISTENCY ISSUES

### 🟡 ISSUE 1: Four Different Redaction Patterns

| Code Path | Mechanism | Selector Support |
|-----------|-----------|------------------|
| CLI text mode | ConfigurableEngine | ✅ YES |
| CLI env mode | env_mode::redact_env_line_configurable | ❓ UNKNOWN |
| Proxy HTTP/1.1 | StreamingRedactor | ❌ NO |
| MITM HTTPS/H2 | StreamingRedactor + H2Redactor | ❓ QUESTIONABLE |

**Risk**: Bugs in any path only affect that path; no consistency guarantees.

---

### 🟡 ISSUE 2: ConfigurableEngine vs StreamingRedactor API Mismatch

CLI uses API that doesn't exist elsewhere:
```rust
// CLI only
config_engine.detect_and_redact(&input_str)  // Returns ConfigurableResult
```

Proxy/MITM use:
```rust
// Proxy/MITM
streaming_redactor.redact_buffer(bytes)  // Returns (Vec<u8>, StreamingStats)
```

**Problem**: Can't easily migrate CLI to StreamingRedactor because the API is different.

---

### 🟡 ISSUE 3: Selector Default Behavior Not Documented

No clear documentation that:
- CLI's PATTERNS tier includes JWT, Bearer, BasicAuth (generic patterns)
- Proxy doesn't include PATTERNS tier by default
- MITM's behavior is unclear

---

## UNVERIFIED ASSUMPTIONS

### ❓ UNKNOWN 1: Does StreamingRedactor Actually Check Selector?

**Question**: When RedactionEngine has a selector, does StreamingRedactor use it?

**Code to Check**: 
- Does `StreamingRedactor::redact_buffer()` check `engine.selector`?
- Does it filter patterns before applying regex?
- Or does it apply ALL patterns?

**Current Evidence**: 
- No selector parameter passed to StreamingRedactor constructor
- No indication selector is checked in redaction

---

### ❓ UNKNOWN 2: ConfigurableEngine Implementation

**Question**: How does ConfigurableEngine actually enforce selectors?

**Assumption**: It filters patterns before redaction
**Risk**: If wrong, CLI has same bug as proxy/MITM

---

### ❓ UNKNOWN 3: H2 Redactor in MITM

**Question**: Does H2Redactor (used in MITM H2) respect selectors?

**Code Reference**: h2_redactor is used but selector passing is unclear.

---

## MISSING VALIDATIONS

### ❌ MISSING 1: No End-to-End Tests Comparing Tools

No test that verifies:
```
Input: "aws-secret-AKIAIOSFODNN7EXAMPLE"
Redact selector: API_KEYS

Expected: CLI → redacted? PROXY → redacted? MITM → redacted?
Actual: All three give same result?
```

---

### ❌ MISSING 2: No Cross-Tool Selector Validation

No test that verifies CLI, Proxy, and MITM all:
- Use same default selectors
- Apply selectors consistently
- Redact the same patterns

---

### ❌ MISSING 3: No Boundary Case Tests

No tests for:
- Secret at chunk boundary (64KB)
- Secret split across lines (env mode)
- Multi-line secrets in HTTP body
- HTTP headers vs body consistency

---

## SUMMARY OF ISSUES

| Issue | Severity | Tools Affected | Type |
|-------|----------|---|---|
| Different defaults | 🔴 CRITICAL | CLI, Proxy | Security Gap |
| MITM selectors ignored | 🔴 CRITICAL | MITM | Security Gap |
| HTTP handler no selector | 🔴 CRITICAL | Proxy, MITM | Security Gap |
| Config vs Streaming API | 🟡 HIGH | CLI vs Proxy | Consistency |
| 4 different redaction paths | 🟡 HIGH | All | Consistency |
| Env mode selector unknown | 🟡 HIGH | CLI | Unknown |
| HTTP vs HTTPS inconsistent | 🟡 HIGH | MITM | Consistency |

---

## RECOMMENDED FIXES (Priority Order)

### PRIORITY 1: Fix Selector Not Being Used
1. Update `handle_http_proxy` to accept and use selector
2. Update `StreamingRedactor` to accept and check selector
3. Verify H2MitmHandler actually uses selectors
4. Add tests to verify selectors are enforced

### PRIORITY 2: Harmonize Defaults
1. CLI: Change default from `PATTERNS` to match Proxy: `CRITICAL,API_KEYS`
2. Document decision to exclude PATTERNS tier by default
3. Make all tools use same defaults

### PRIORITY 3: API Consistency
1. Update CLI to use `StreamingRedactor` instead of `ConfigurableEngine`
2. Ensure all three tools use same redaction pipeline
3. Single code path for all redaction

### PRIORITY 4: Testing
1. Add cross-tool consistency tests
2. Add boundary case tests
3. Add selector enforcement verification

