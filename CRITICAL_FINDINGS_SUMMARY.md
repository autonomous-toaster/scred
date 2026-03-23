# CRITICAL FINDINGS - NEGATIVE BIAS CODE REVIEW

**Scope**: SCRED CLI, Proxy, and MITM selector enforcement
**Approach**: Negative bias (assume broken), compare implementations
**Date**: 2026-03-23

---

## EXECUTIVE SUMMARY

**Status**: CRITICAL SECURITY GAP IDENTIFIED

Negative bias code review revealed that **pattern selectors are silently ignored in all production tools (Proxy and MITM)**, despite working correctly in CLI.

### The Gap
- **CLI**: Selectors work ✅ (configurable_engine.rs)
- **Proxy**: Selectors ignored ❌ (streaming_redactor + redaction_engine)
- **MITM**: Selectors ignored ❌ (redaction_engine only)

### The Problem
Users configure `--redact CRITICAL` expecting compliance (only redact critical secrets). 
Instead, ALL 244 patterns are redacted silently without error.

### Scope
- Affects ALL HTTP/HTTPS redaction (requests + responses)
- Affects both Proxy and MITM tools
- Affects streaming and buffered code paths
- Affects all users relying on selectors for compliance

---

## DETAILED FINDINGS

### Critical Bug #1: HTTP Handler Uses Wrong Engine

**File**: `crates/scred-http/src/http_proxy_handler.rs` (lines 132, 183)

```rust
let redacted_request_result = redaction_engine.redact(&full_request);
```

- ❌ Uses `RedactionEngine.redact()` (applies ALL 244 patterns)
- ✅ Should use `ConfigurableEngine.redact_only()` (respects selectors)
- **Impact**: Proxy + MITM HTTP redaction ignores selectors

### Critical Bug #2: Proxy Streaming Creates Unused ConfigurableEngine

**File**: `crates/scred-proxy/src/main.rs` (lines 424-445)

```rust
let _config_engine = Arc::new(ConfigurableEngine::new(...));  // ← NOT USED
// ...
let redactor = Arc::new(StreamingRedactor::with_defaults(...));  // ← USED INSTEAD
```

- ❌ ConfigurableEngine created but never used (variable starts with `_`)
- ❌ StreamingRedactor used instead (doesn't support selectors)
- **Impact**: Proxy streaming ignores selectors silently

### Critical Bug #3: MITM HTTP Handler Ignores Selectors Parameter

**File**: `crates/scred-mitm/src/mitm/http_handler.rs`

Function signature accepts only:
- `redaction_engine` (no selectors)
- Missing: `detect_patterns`, `redact_patterns`

```rust
pub async fn handle_http_proxy(
    client_read: ...,
    client_write: ...,
    first_line: &str,
    redaction_engine: Arc<RedactionEngine>,  // ← No selectors!
    upstream_resolver: ...,
) -> Result<()>
```

- ❌ Doesn't accept selectors as parameters
- ❌ MITM main.rs configures selectors but doesn't pass them
- **Impact**: MITM HTTP redaction can't use selectors even if wanted

### Critical Bug #4: MITM H2 Handler Accepts But Ignores Selectors

**File**: `crates/scred-mitm/src/mitm/h2_mitm_handler.rs` (line 143)

```rust
async fn handle_stream(
    ...
    detect_patterns: scred_http::PatternSelector,  // ← PARAMETER
    redact_patterns: scred_http::PatternSelector,   // ← PARAMETER
) -> Result<()> {
    ...
    let redacted = engine.redact(&body_str);  // ← PARAMETER IGNORED!
}
```

- ❌ Selectors passed as parameters but NEVER USED
- ❌ Dead code: parameters could be deleted with no change to behavior
- ❌ Always uses `engine.redact()` (applies ALL patterns)
- **Impact**: MITM HTTPS/H2 ignores selectors silently

---

## COMPARISON TABLE

| Tool | HTTP Redaction Engine | Selectors? | Code Status |
|------|----------------------|-----------|------------|
| CLI (file) | ConfigurableEngine | ✅ YES | WORKS |
| CLI (stdin) | ConfigurableEngine | ✅ YES | WORKS |
| **Proxy HTTP** | RedactionEngine | ❌ NO | BROKEN |
| **Proxy Streaming** | StreamingRedactor | ❌ NO | BROKEN |
| **MITM HTTP** | RedactionEngine | ❌ NO | BROKEN |
| **MITM HTTPS/H2** | RedactionEngine | ❌ NO (ignored) | BROKEN |

---

## EVIDENCE

### Code Path Analysis

**CLI Path (WORKING)**:
```
CLI main()
  → parse selectors from CLI/ENV/file
  → ConfigurableEngine::new(engine, detect_selector, redact_selector)
  → config_engine.redact_only(text)  ← SELECTORS APPLIED ✅
```

**Proxy HTTP Path (BROKEN)**:
```
Proxy main()
  → parse selectors from CLI/ENV/file  
  → ConfigurableEngine created but unused (_config_engine)
  → http_proxy_handler(redaction_engine)  ← NO SELECTORS PASSED
  → redaction_engine.redact(text)  ← IGNORES SELECTORS ❌
```

**Proxy Streaming Path (BROKEN)**:
```
Proxy main()
  → parse selectors from CLI/ENV/file
  → StreamingRedactor::with_defaults(redaction_engine)  ← NO SELECTORS
  → redactor.redact_buffer(chunk)  ← IGNORES SELECTORS ❌
```

**MITM HTTP Path (BROKEN)**:
```
MITM main()
  → set_redact_patterns("CRITICAL")
  → handle_http_proxy()  ← SELECTORS NOT PASSED
  → http_handler::handle_http_proxy(engine)  ← NO SELECTORS ACCEPTED
  → engine.redact(text)  ← IGNORES SELECTORS ❌
```

**MITM HTTPS Path (BROKEN)**:
```
MITM main()
  → set_redact_patterns("CRITICAL")
  → handle_tls_mitm(..., detect_patterns, redact_patterns)
  → h2_mitm_handler::handle_stream(..., detect_patterns, redact_patterns)
  → engine.redact(body)  ← PARAMETERS IGNORED ❌
```

---

## SECURITY IMPLICATIONS

### Problem 1: Silent Configuration Bypass
```
User intent:     --redact CRITICAL,API_KEYS
Configured at:   ✅ Proxy main(), MITM main()
Enforced at:     ❌ Redaction engine layer (ALL patterns used)
User sees:       ✅ Configuration accepted, logged
User expects:    Only CRITICAL and API_KEYS redacted
User gets:       ALL 244 patterns redacted
User knows:      ❌ NO ERROR - thinks config worked
```

### Problem 2: Compliance Failure
- Regulated environments need `--redact CRITICAL,API_KEYS` to preserve logs
- CLI correctly respects this
- Proxy doesn't - redacts everything silently
- Compliance broken, auditor doesn't know

### Problem 3: Defense in Depth Broken
- Strategy: Redact different patterns at different layers
- Example: CLI redacts CRITICAL, Proxy redacts API_KEYS
- Reality: Proxy redacts everything (selector ignored)
- Both end up redacting same patterns redundantly

### Problem 4: Inconsistent Behavior
- Same request through CLI vs Proxy produces different output
- Impossible for user to predict behavior
- Can't rely on tool consistency for security

---

## ROOT CAUSES

### Root Cause 1: Type System Mismatch
- `http_proxy_handler()` accepts `RedactionEngine`, not trait object
- Can't pass `ConfigurableEngine` without changing signature
- Would require refactoring shared code (risky)

### Root Cause 2: Three Different Redaction Engines
- **RedactionEngine**: No selector support, used in most places
- **ConfigurableEngine**: Wraps RedactionEngine with selectors, used only in CLI
- **StreamingRedactor**: No selector support, used in streaming paths
- Inconsistency encouraged by having 3 parallel implementations

### Root Cause 3: Code Duplication
- HTTP proxy logic in 3 places (proxy main, http_proxy_handler, mitm wrapper)
- No consistent interface for selectors
- Each path had to be updated separately
- Updates missed in some paths

### Root Cause 4: No Integration Tests
- Test suite doesn't verify selector consistency across tools
- Doesn't compare CLI vs Proxy vs MITM outputs
- P0#1 marked as "being fixed" but actually still broken
- Allowed this to ship

---

## WHICH PATTERNS GET THROUGH?

### What User Expects (`--redact CRITICAL`)
```
Redact:  AWS_ACCESS_KEY, AWS_SECRET_KEY, ...CRITICAL tier only
Keep:    GitHub tokens, Slack tokens, ...API_KEYS tier
Result:  Logs preserved with API_KEYS visible, CRITICAL hidden
```

### What CLI Does
```
Redact:  AWS_ACCESS_KEY, AWS_SECRET_KEY, ...CRITICAL tier ✅
Keep:    GitHub tokens, Slack tokens, ...API_KEYS tier ✅
Result:  Exactly as expected
```

### What Proxy Actually Does
```
Redact:  ALL 244 patterns (including API_KEYS) ❌
Keep:    Nothing
Result:  Logs destroyed, more than user wanted
```

### What MITM Actually Does
```
Redact:  ALL 244 patterns (including API_KEYS) ❌
Keep:    Nothing
Result:  Logs destroyed, more than user wanted
```

---

## VERIFICATION TESTS

To confirm these bugs, create a test file with:
- CRITICAL tier secret: AWS key
- API_KEYS tier secret: GitHub token
- INFRASTRUCTURE tier secret: K8s secret

Test each tool with `--redact CRITICAL`:

```bash
# CLI - should only redact AWS key
scred --redact CRITICAL < test.txt

# Proxy - currently redacts both (BUG)
scred-proxy --redact CRITICAL < test.txt

# MITM - currently redacts both (BUG)
scred-mitm --redact CRITICAL < test.txt
```

Expected: All three produce identical output with only AWS key redacted
Actual: CLI correct, Proxy and MITM redact everything

---

## IMPACT ASSESSMENT

### Who Is Affected?
- ✅ CLI users: Safe (selectors work)
- ❌ Proxy users: Not safe (selectors ignored)
- ❌ MITM users: Not safe (selectors ignored)
- ❌ Anyone using Proxy/MITM in production: High risk

### What Kind of Impact?
- Compliance: Can't meet regulations
- Security: Expose more than intended
- Audit: Silent configuration bypass (worst kind)
- Debugging: Can't troubleshoot why selector didn't work

### Severity
- **Technical**: CRITICAL
- **Security**: HIGH
- **Compliance**: HIGH
- **User**: HIGH (silent failure)

---

## RECOMMENDED ACTIONS

### Immediate (P0 - This Week)
1. Fix `http_proxy_handler.rs` to use ConfigurableEngine
2. Fix proxy streaming to use _config_engine
3. Fix MITM HTTP handler signature to accept selectors
4. Fix MITM H2 handler to use selectors

### Short Term (Next Sprint)
1. Create integration tests comparing CLI, Proxy, MITM outputs
2. Extract shared redaction logic to eliminate duplication
3. Create Redactor trait to unify interfaces

### Long Term (Architecture)
1. Extract configuration to scred-config crate
2. Implement trait-based redaction engine abstraction
3. Ensure consistency across all tools by design

---

## COMPARISON WITH P0#1

**P0#1** (Previously identified):
- Problem: Pattern selectors not used
- Scope: Partially acknowledged
- Fix: Create ConfigurableEngine (partially done)
- Status: Still broken

**This Review** (Negative Bias):
- Problem: Pattern selectors bypassed in ALL production code paths
- Scope: Much wider than initially assessed
- Fix: Needed in 4+ locations, not just streaming
- Status: CRITICAL, not just P0#1

**Relationship**: This is a SUPERSET of P0#1. P0#1 focused on streaming, but the problem is everywhere.

---

## TIMELINE

These bugs existed since:
- ✅ CLI created: Uses ConfigurableEngine (correct)
- ❌ Proxy created: Uses RedactionEngine (wrong)
- ❌ MITM created: Uses RedactionEngine (wrong)
- ❌ P0#1 created: Acknowledged but not fixed
- ❌ v1.0.0 released: With broken selector enforcement

**The bugs have been in production since before v1.0.0.**

