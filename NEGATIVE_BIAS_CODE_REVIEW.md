# NEGATIVE BIAS CODE REVIEW - SCRED Secret Detection/Redaction

**Approach**: Assume code is broken, compare implementations, find gaps
**Date**: 2026-03-23
**Scope**: CLI, Proxy, MITM - focus on inconsistencies

---

## 🚨 CRITICAL FINDINGS

### 1. PROXY HTTP HANDLER IGNORES SELECTORS - BUG CONFIRMED

**File**: `crates/scred-http/src/http_proxy_handler.rs`
**Lines**: 132, 183

```rust
let redacted_request_result = redaction_engine.redact(&full_request);
// ...
let redacted_response_result = redaction_engine.redact(&response_str);
```

**Issue**: Uses `RedactionEngine::redact()` directly, NOT `ConfigurableEngine`
- RedactionEngine redacts ALL patterns (no selector support)
- ConfigurableEngine respects selectors
- Result: Selectors configured in CLI are IGNORED in HTTP handler

**Impact**: SECRETS MAY NOT BE REDACTED OR DETECTED CORRECTLY
- User sets `--redact CRITICAL` (only AWS keys)
- HTTP handler redacts ALL 244 patterns anyway
- User gets less redaction than expected (or more, depending on perspective)

**Affected Tools**:
- ✅ scred-proxy: Uses http_proxy_handler → IGNORES SELECTORS
- ✅ scred-mitm: Uses http_proxy_handler → IGNORES SELECTORS

**Status**: P0 CRITICAL BUG (not just P0#1)

---

### 2. PROXY USES STREAMING REDACTOR FOR STREAMING HANDLER - SELECTORS IGNORED

**File**: `crates/scred-proxy/src/main.rs`
**Lines**: 424-445

```rust
let _config_engine = Arc::new(ConfigurableEngine::new(
    redaction_engine.clone(),
    config.detect_selector.clone(),
    config.redact_selector.clone(),
));

// ...but it's not used!

let redactor = Arc::new(StreamingRedactor::with_defaults(redaction_engine.clone()));

// Selectors configured here: ✅
// Selectors USED here: ❌ IGNORED
```

**Issue**: ConfigurableEngine created but never used
- Variable named `_config_engine` (underscore signals it's unused)
- StreamingRedactor created instead (ignores selectors)
- Comments explicitly document the problem

**Impact**: STREAMING REDACTION IGNORES SELECTORS
- Request/response bodies redacted with ALL patterns
- Selector configuration is SILENT IGNORED (no error)
- Users think selectors work, but they don't

**Status**: P0#1 - PARTIALLY ACKNOWLEDGED (but code still broken)

---

### 3. MITM HTTP HANDLER DOESN'T ACCEPT SELECTORS AT ALL

**File**: `crates/scred-mitm/src/mitm/http_handler.rs`
**Lines**: All

The function signature:
```rust
pub async fn handle_http_proxy(
    client_read: ...,
    client_write: ...,
    first_line: &str,
    redaction_engine: Arc<RedactionEngine>,  // ← No selectors!
    upstream_resolver: ...,
) -> Result<()>
```

**Issue**: Doesn't accept detect_patterns or redact_patterns
- MITM main.rs configures selectors: `config.proxy.redact_patterns`
- But never passes them to http_handler
- http_handler can't even use them if it wanted to

**Impact**: MITM SELECTORS ARE SILENTLY IGNORED
- User sets `--redact CRITICAL`  
- MITM HTTP requests use ALL patterns
- Configuration completely ignored

**Related**: main.rs line 178 configures redact_patterns but line 189 doesn't use it

---

### 4. PROXY AND MITM USE DIFFERENT CODE PATHS FOR HTTP REDACTION

**Proxy HTTP Path**:
```
handle_connection()
  → stream_request_to_upstream() [StreamingRedactor, no selectors]
  → stream_response_to_client() [StreamingRedactor, no selectors]
```

**MITM HTTP Path**:
```
handle_http_proxy() [in proxy.rs]
  → handle_http_proxy() [in http_handler.rs, delegates to scred-http]
  → shared_handle_http_proxy() [redaction_engine.redact(), no selectors]
```

**Issue**: TWO DIFFERENT IMPLEMENTATIONS
- Proxy: Uses StreamingRedactor (slower, streaming, ignores selectors)
- MITM: Uses shared handler (faster, buffered, ignores selectors)
- Both ignore selectors, but through different mechanisms

**Impact**: Inconsistent behavior and hard to debug

---

### 5. CLI WORKS CORRECTLY - BUT DIFFERENT CODE PATH

**File**: `crates/scred-cli/src/main.rs`

CLI correctly uses ConfigurableEngine:
```rust
let config_engine = ConfigurableEngine::new(
    redaction_engine,
    detect_selector,
    redact_selector,
);

// Uses it:
config_engine.redact_only(&line)
config_engine.detect_only(&line)
```

**But**: CLI is DIFFERENT from Proxy and MITM
- CLI: Full text, uses ConfigurableEngine ✅
- Proxy: Streaming, uses StreamingRedactor ❌
- MITM: Full text, uses RedactionEngine ❌

**Impact**: Security behavior differs between tools
- Same secret may be redacted in CLI but not Proxy
- Same selector may work in CLI but not MITM

---

## 🔍 COMPARISON TABLE

| Component | HTTP Redaction | Selectors Supported | Code Path |
|-----------|-----------------|-------------------|-----------|
| CLI (file) | ConfigurableEngine | ✅ YES | Full text |
| CLI (stdin) | ConfigurableEngine | ✅ YES | Per-line |
| Proxy (streaming) | StreamingRedactor | ❌ NO | Streaming chunks |
| Proxy (HTTP handler) | RedactionEngine | ❌ NO | Full buffer |
| MITM (HTTP) | RedactionEngine | ❌ NO | Full buffer |
| MITM (HTTPS/H2) | ? Unknown | ? Unknown | TLS MITM stream |

---

## 📊 SELECTORS ENFORCEMENT AUDIT

### Where Selectors Are Parsed
1. ✅ CLI: `main.rs` lines 41-65
2. ✅ Proxy: `main.rs` lines 264-285 (env) + 53-109 (file)
3. ✅ MITM: `main.rs` lines 114-118 (CLI) + 134-136 (env)

### Where Selectors Are USED
1. ✅ CLI: `main.rs` lines 401, 455, 573, 638 (ConfigurableEngine::new)
2. ❌ Proxy: `main.rs` line 424 (_config_engine unused)
3. ❌ MITM: Never passed to handlers

### Where Selectors Should Be Used But Aren't
1. `http_proxy_handler.rs`: Uses `redaction_engine.redact()`, not selectors
2. `streaming_request.rs`: Uses `StreamingRedactor`, not selectors
3. `streaming_response.rs`: Uses `StreamingRedactor`, not selectors
4. MITM `http_handler.rs`: Doesn't even accept selectors as parameter

---

## 🐛 FAILURE MODES

### Mode 1: Selector Silently Ignored in HTTP Proxy
```bash
$ scred-proxy --redact CRITICAL  # User only wants AWS keys redacted
# Request comes in with GitHub token + AWS key
# Result: BOTH redacted (selector ignored, ALL patterns used)
# Expected: ONLY AWS key redacted
# User has NO ERROR and no way to know selector didn't work
```

### Mode 2: Selector Silently Ignored in MITM
```bash
$ scred-mitm --redact CRITICAL --detect API_KEYS
# HTTPS request has GitHub token
# Result: May be redacted anyway (selector ignored)
# Expected: Should NOT be redacted
# User has NO ERROR and no way to know selector didn't work
```

### Mode 3: Inconsistent Behavior Across Tools
```bash
# Same file processed three ways:
$ scred --redact CRITICAL < secret.txt       # ✅ WORKS (only redacts CRITICAL)
$ echo "..." | scred-proxy ... --redact CRITICAL  # ❌ FAILS (redacts all)
$ scred-mitm --redact CRITICAL < secrets    # ❌ FAILS (redacts all)

# Same secret, different outcomes
# SECURITY GAP: Can't rely on MITM/Proxy to respect config
```

---

## 🔐 SECURITY IMPLICATIONS

### Problem 1: Configuration Not Enforced
- User configures `--redact CRITICAL,API_KEYS`
- User assumes only those patterns are redacted
- Reality: ALL patterns may be redacted anyway
- No error, no warning - silent failure

### Problem 2: Inconsistent Across Tools
- CLI respects selectors
- Proxy doesn't
- MITM doesn't
- Same deployment-wide config, different behavior per tool

### Problem 3: No Visibility
- No errors logged
- No warnings shown
- ConfigurableEngine created but unused (variable starts with `_`)
- Comments acknowledge the problem but code doesn't fix it

### Problem 4: Defense in Depth Broken
- If user wants ONLY critical secrets redacted (to preserve log context)
- Must use CLI
- Can't use Proxy or MITM
- Forces architectural choice

---

## 📋 DETAILED CODE PATHS

### CLI (WORKS ✅)
```
main()
  → parse_detect_patterns() [from CLI/ENV/file]
  → parse_redact_patterns() [from CLI/ENV/file]
  → create ConfigurableEngine(engine, detect_selector, redact_selector)
  → for each line/chunk:
      → config_engine.redact_only(line)  ← SELECTORS APPLIED
```

### Proxy Streaming (BROKEN ❌)
```
main()
  → from_config_file() → detect_selector, redact_selector
  → from_env() → detect_selector, redact_selector  
  → create ConfigurableEngine (line 424) ← CREATED BUT UNUSED
  → create StreamingRedactor (line 445) ← USED INSTEAD
  → handle_connection()
    → stream_request_to_upstream(redactor) ← NO SELECTORS
      → redactor.redact_buffer(chunk) ← IGNORES SELECTORS
    → stream_response_to_client(redactor) ← NO SELECTORS
```

### Proxy HTTP (BROKEN ❌)
```
handle_connection()
  → parse HTTP request
  → http_proxy_handler::handle_http_proxy(
      redaction_engine,  ← NO SELECTORS PASSED
    )
    → redaction_engine.redact(full_request) ← IGNORES SELECTORS
```

### MITM HTTP (BROKEN ❌)
```
main()
  → set_redact_patterns("CRITICAL") ← CONFIGURED
  → handle_http_proxy()  ← SELECTORS NOT PASSED
    → http_handler::handle_http_proxy(
        redaction_engine,  ← NO SELECTORS
      )
      → redaction_engine.redact(...) ← IGNORES SELECTORS
```

---

## 🎯 ROOT CAUSES

### Root Cause 1: Type System Mismatch
- Streaming functions expect `Arc<StreamingRedactor>`
- ConfigurableEngine is a different type
- Can't pass ConfigurableEngine through existing functions
- Would require trait objects or refactoring

### Root Cause 2: No Unified Redaction Engine
- RedactionEngine: Doesn't support selectors
- ConfigurableEngine: Wraps RedactionEngine with selector support
- StreamingRedactor: Doesn't support selectors, streaming-optimized
- Some code uses each, no consistency

### Root Cause 3: Code Duplication
- HTTP proxy logic exists in three places:
  - scred-proxy/src/main.rs (handle_connection)
  - scred-http/src/http_proxy_handler.rs (shared)
  - scred-mitm/src/mitm/http_handler.rs (wrapper)
- No consistent way to pass selectors through all of them

### Root Cause 4: No Integration Tests
- Test suite doesn't verify selector enforcement
- Doesn't verify consistency across tools
- P0#1 was partially acknowledged but not fully fixed
- Code comments admit selector logic isn't used

---

## 🚩 AFFECTED CODE LOCATIONS

### Must Fix (Critical)
1. `crates/scred-http/src/http_proxy_handler.rs` (lines 132, 183)
   - Change: Use ConfigurableEngine instead of RedactionEngine
   - Impact: Proxy and MITM HTTP handlers

2. `crates/scred-proxy/src/main.rs` (lines 424-445)
   - Change: Use _config_engine instead of StreamingRedactor
   - Impact: Proxy streaming handlers

3. `crates/scred-mitm/src/mitm/http_handler.rs` (signature)
   - Change: Accept detect_patterns, redact_patterns as parameters
   - Impact: MITM HTTP handler

### Should Fix (Important)
4. `crates/scred-mitm/src/mitm/proxy.rs` (line 178)
   - Change: Pass redact_patterns to handle_http_proxy
   - Impact: MITM selector enforcement

5. `crates/scred-http/src/streaming_request.rs`
   - Change: Support selectors (requires trait redesign)
   - Impact: Proxy streaming

6. `crates/scred-http/src/streaming_response.rs`
   - Change: Support selectors (requires trait redesign)
   - Impact: Proxy streaming

---

## ⚠️ HIDDEN ASSUMPTIONS

### Assumption 1: All Patterns Should Be Used
- Code assumes using all patterns is "safe" (detects more)
- But user configured selectors to EXCLUDE some patterns
- Using excluded patterns violates user intent

### Assumption 2: More Redaction Is Better
- If selector says CRITICAL only, some code redacts all patterns
- User might be OK with GitHub tokens being visible
- Redacting everything is overzealous

### Assumption 3: Code Comments = Fixed
- Code has TODO and comments explaining selector issues
- But selectors still aren't actually used
- Comments don't prevent bugs

### Assumption 4: One Implementation Can't Break Others
- CLI works with ConfigurableEngine
- Proxy/MITM work with RedactionEngine
- If one breaks, thought others would work
- But they're supposed to behave the same way

---

## 🔬 VERIFICATION CHECKLIST

To confirm these bugs, test:

- [ ] CLI: `scred --redact CRITICAL < file.txt` - only redacts CRITICAL? 
- [ ] Proxy: Same file through proxy with `--redact CRITICAL` - same output?
- [ ] MITM: Same file through MITM with `--redact CRITICAL` - same output?
- [ ] All three should produce identical output (they don't)
- [ ] CLI redacts fewer patterns than Proxy (it does)
- [ ] CLI redacts fewer patterns than MITM (it does)

---

## 📈 SEVERITY ASSESSMENT

### Security Impact: HIGH
- Selectors not enforced in production tools
- Users rely on selectors for compliance
- May expose more secrets than configured

### Consistency Impact: CRITICAL  
- Same code, different behavior per tool
- Impossible to predict which secrets will be redacted
- Defense in depth strategy fails

### User Impact: HIGH
- Silent failure (no error message)
- Users unaware selectors don't work
- Can't debug or report issues

### Scope: VERY WIDE
- Affects all HTTP redaction (requests and responses)
- Affects both MITM and Proxy
- Affects streaming and buffered paths
- Affects all pattern tiers

