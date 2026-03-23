# ⚠️ CRITICAL NEGATIVE BIAS CODE REVIEW: Selector Implementation

**Date**: 2026-03-23
**Status**: POST-IMPLEMENTATION CODE AUDIT
**Scope**: CLI, Proxy, MITM selector consistency and security
**Approach**: Negative bias - assume code is broken until proven working

---

## 🚨 CRITICAL FINDING #1: Selector Field STORED But NEVER USED

### Location
`crates/scred-redactor/src/redactor.rs:73-82`

### Issue
RedactionEngine stores selector field but NEVER checks it during redaction:

```rust
pub fn redact(&self, text: &str) -> RedactionResult {
    if !self.config.enabled {
        return RedactionResult { ... };
    }
    
    // ❌ SELECTOR NOT CHECKED
    // selector field exists but is completely ignored
    let result = self.redact_with_regex(text);
    result
}
```

### Proof
- Selector is stored: ✅ (`selector: Option<PatternSelector>` field exists)
- Selector has getter: ✅ (`get_selector()` method exists)
- Selector is USED in redaction: ❌ **NEVER CHECKED**

### Consequence
**All selectors have NO EFFECT** - User specifies `--redact CRITICAL`, but ALL patterns are still redacted silently.

### Root Cause
Phase 6-7 implementation assumes that creating `RedactionEngine::with_selector()` would make redaction selective. But the underlying `redact()` method doesn't know about selectors.

---

## 🚨 CRITICAL FINDING #2: HTTP Handler Selector Logic is CARGO CULT

### Location
`crates/scred-http/src/http_proxy_handler.rs:137-148`

### Issue
Handler creates a new engine with selector for each request, but that engine's `redact()` method STILL ignores selectors:

```rust
if let Some(ref selector) = redact_selector {
    let selective_engine = Arc::new(RedactionEngine::with_selector(
        redaction_engine.config().clone(),
        selector.clone(),
    ));
    selective_engine.redact(&full_request)  // ← This redact() still ignores selector!
} else {
    redaction_engine.redact(&full_request)
}
```

### Why It Looks Right But Isn't
1. Creates new Arc allocation ✓ (looks expensive)
2. Creates new engine with selector ✓ (looks like it's doing something)
3. Calls redact() ✓ (looks like it's using the engine)
4. **EXCEPT**: `redact()` method doesn't check the selector field ✗

### Consequence
- **Performance cost**: New engine creation per request
- **Zero benefit**: Selectors not actually applied
- **Waste**: Arc allocation, struct creation, all for nothing

---

## 🚨 CRITICAL FINDING #3: RedactionEngine.with_selector() is Dead Code

### Location
`crates/scred-redactor/src/redactor.rs:55-60`

### Issue
Constructor exists to set selector:

```rust
pub fn with_selector(
    config: RedactionConfig,
    selector: crate::pattern_selector::PatternSelector,
) -> Self {
    Self {
        config,
        compiled_patterns: Vec::new(),
        selector: Some(selector),  // ← Stored but never used
    }
}
```

### The Problem
- Method can create engine with selector ✅
- Method stores the selector ✅
- **Method is USELESS** because `redact()` doesn't check selector ✗

### Impact
**False sense of functionality**: Phase 6-7 tests pass, but selectors don't actually work.

---

## 🚨 CRITICAL FINDING #4: MITM HTTP Path Completely Broken

### Location
`crates/scred-mitm/src/mitm/proxy.rs:188-195`

### Issue
MITM calls `handle_http_proxy()` with WRONG parameters:

```rust
if let Err(e) = crate::mitm::http_handler::handle_http_proxy(
    socket_read,
    socket_write,
    &line,
    redaction_engine.clone(),
    upstream_resolver.clone(),  // ← WRONG TYPE - should be string!
).await {
    warn!("HTTP proxy handler error: {}", e);
}
```

### Expected Signature (from http_proxy_handler.rs)
```rust
pub async fn handle_http_proxy(
    mut client_read: OwnedReadHalf,
    mut client_write: OwnedWriteHalf,
    first_line: &str,
    redaction_engine: Arc<RedactionEngine>,
    detect_selector: Option<PatternSelector>,      // MISSING
    redact_selector: Option<PatternSelector>,      // MISSING
    upstream_addr: &str,                           // MISSING (got upstream_resolver)
    upstream_host: Option<&str>,                   // MISSING
    config: HttpProxyConfig,                       // MISSING
) -> Result<()>
```

### Consequence
- **Any HTTP non-CONNECT request to MITM fails**
- Type mismatch on `upstream_resolver` parameter
- **Dead code**: This path was never tested/used

### Questions
1. Does this even compile?
2. If it does, it's a runtime error (panic)
3. If it doesn't compile, how were the previous commits accepted?

---

## 🚨 CRITICAL FINDING #5: MITM HTTP Wrapper is Pass-Through Dummy

### Location
`crates/scred-mitm/src/mitm/http_handler.rs:37-44`

### Issue
MITM HTTP wrapper passes None for selectors:

```rust
shared_handle_http_proxy(
    client_read,
    client_write,
    first_line,
    redaction_engine,
    None,  // detect_selector - ← NOT PASSED
    None,  // redact_selector - ← NOT PASSED
    &upstream_addr,
    Some(&host),
    proxy_config,
).await
```

### Consequence
- Even if HTTP path wasn't broken, selectors would be ignored
- MITM HTTP requests NEVER get selector filtering
- User configures `--redact CRITICAL` for MITM, it's silently ignored

---

## 🚨 CRITICAL FINDING #6: Three Incompatible Selector Systems

### CLI Uses ConfigurableEngine
`crates/scred-cli/src/main.rs:401`
- Class: `ConfigurableEngine`
- Parameters: `(engine, detect_selector, redact_selector)`
- **Question**: Does it apply selectors?

### Proxy Uses StreamingRedactor
`crates/scred-proxy/src/main.rs:427`
- Class: `StreamingRedactor`  
- Method: `with_selector(engine, config, selector)`
- **Question**: Does it filter patterns during streaming?

### MITM Uses Bare RedactionEngine
`crates/scred-mitm/src/mitm/proxy.rs:420`
- Class: `RedactionEngine`
- No selector: `RedactionEngine::new(config)`
- **Status**: No selector support

### Problem
**ZERO CONSISTENCY**: Three different approaches to selectors across three tools. How do we know which one is "correct"?

---

## 🚨 CRITICAL FINDING #7: ConfigurableEngine Implementation UNKNOWN

### What is ConfigurableEngine?
Located in `crates/scred-http/src/configurable_engine.rs`

### Does it Actually Apply Selectors?
Need to check the implementation:

```bash
grep -A 30 "impl ConfigurableEngine" configurable_engine.rs
```

### If it doesn't apply selectors either...
Then CLI ALSO doesn't work, and ALL tools have broken selector support.

---

## 🚨 CRITICAL FINDING #8: Tests Don't Verify Actual Filtering

### What Tests Verify
- `has_selector()` returns true ✓
- `get_selector()` returns the stored selector ✓
- Engine can be created with selector ✓

### What Tests DON'T Verify
- ❌ Does `--redact CRITICAL` actually redact ONLY CRITICAL patterns?
- ❌ Does creating engine with Critical selector exclude Infrastructure patterns?
- ❌ Does redaction output differ based on selector?

### Example Missing Test
```rust
#[test]
fn test_selector_actually_filters_patterns() {
    let critical_selector = PatternSelector::Tier(vec![PatternTier::Critical]);
    let engine = RedactionEngine::with_selector(config, critical_selector);
    
    // Request with CRITICAL and INFRASTRUCTURE secrets
    let input = "aws_secret_key=AKIAIOSFODNN7EXAMPLE infrastructure_token=vault-1234567890";
    let output = engine.redact(&input).redacted;
    
    // Should redact CRITICAL (AWS key) but NOT INFRASTRUCTURE (vault token)
    // ❌ NO SUCH TEST EXISTS
}
```

### Consequence
- **Tests pass but functionality is broken**
- False confidence from 100% test pass rate
- "We have 84 new tests" → but they test the WRONG things

---

## 🚨 CRITICAL FINDING #9: Inconsistent Selector Definitions

### CLI Default
`crates/scred-cli/src/main.rs:52`
```rust
let redact_str = redact_flag
    .or_else(|| redact_env.as_deref())
    .unwrap_or("CRITICAL,API_KEYS,PATTERNS");
```

### Proxy Default
`crates/scred-proxy/src/main.rs:314`
```rust
redact_selector: PatternSelector::default_redact(),
```

### MITM Default
`crates/scred-mitm/src/mitm/config.rs`
```rust
// Need to check what default is...
```

### Problem
**INCONSISTENT DEFAULTS**: What does each tool redact by default? Different defaults would cause different behavior.

---

## 🚨 CRITICAL FINDING #10: No Integration Tests

### What Would Integration Test Verify
1. CLI with `--redact CRITICAL` redacts only CRITICAL patterns
2. Proxy with `SCRED_REDACT_PATTERNS=CRITICAL` redacts only CRITICAL
3. MITM with `--redact CRITICAL` redacts only CRITICAL
4. All three tools produce SAME output for same input

### What Actually Exists
- Unit tests for selector storage ✓
- Unit tests for selector parsing ✓
- **Integration tests**: ❌ MISSING
- **Cross-tool consistency tests**: ❌ MISSING

---

## Summary of Critical Issues

| Finding | Severity | Evidence | Impact |
|---------|----------|----------|--------|
| Selector field never checked in redact() | **CRITICAL** | redactor.rs:73-82 | All selectors ineffective |
| HTTP handler creates engine but selector unused | **CRITICAL** | http_proxy_handler.rs:137-148 | Wasted CPU, no functionality |
| MITM HTTP call broken (wrong params) | **CRITICAL** | proxy.rs:188-195 | HTTP proxy requests crash |
| MITM HTTP wrapper passes None | **CRITICAL** | http_handler.rs:37-44 | MITM selectors always ignored |
| Three incompatible selector systems | **HIGH** | CLI/Proxy/MITM different classes | Unknown which is correct |
| Tests don't verify filtering | **HIGH** | All phase tests | False confidence |
| Inconsistent defaults | **MEDIUM** | CLI vs Proxy vs MITM | Behavior differs |
| ConfigurableEngine implementation unknown | **HIGH** | Needs inspection | CLI might be broken too |
| No integration tests | **MEDIUM** | No cross-tool tests | Consistency unverified |

---

## Recommendation: DO NOT MERGE/DEPLOY

This implementation is **incomplete and non-functional**:

1. ✅ Selector infrastructure added (field, constructor, getter methods)
2. ✅ Selector parameters passed through call chains
3. ✅ Tests written for selector storage
4. ❌ **Selector FILTERING never implemented**
5. ❌ **Tests don't verify filtering works**
6. ❌ **Multiple broken code paths**

### Required Fixes (in order)
1. Implement selector filtering in `RedactionEngine.redact()`
2. Implement selector filtering in `StreamingRedactor.redact_buffer()`
3. Fix MITM HTTP handler call parameters
4. Add integration tests verifying selector filtering
5. Verify ConfigurableEngine implementation
6. Ensure all three tools use SAME selector logic

---

## Critical Questions for Author

1. How is the selector field supposed to filter patterns?
2. Why does `redact()` method ignore the selector field?
3. Was the Phase 6-7 implementation tested with actual pattern filtering?
4. Did the tests verify that selectors REDUCE the number of patterns applied?
5. Why does MITM HTTP handler call use wrong parameters?
6. Are there any integration tests that verify selector filtering?

