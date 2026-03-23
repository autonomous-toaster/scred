# COMPREHENSIVE NEGATIVE BIAS REVIEW - SCRED v2.0.0
**Assessment Type**: Security & Consistency Review with Negative Bias  
**Date**: 2026-03-23  
**Status**: CRITICAL ISSUES IDENTIFIED  

---

## EXECUTIVE SUMMARY

Negative bias review of SCRED CLI, Proxy, and MITM reveals **3 CRITICAL security gaps** where selectors are ignored or mishandled, leading to either over-redaction or secrets not being redacted. Additionally, **4 HIGH consistency issues** create maintenance burden and increase bug risk.

**Key Finding**: Same user action (`--redact CRITICAL,API_KEYS`) produces **different results** across tools:
- ✅ CLI: Correctly redacts only CRITICAL+API_KEYS
- ❌ Proxy: Redacts ALL patterns (ignores selector)
- ❌ MITM HTTP: Redacts ALL patterns (ignores selector)
- ❓ MITM HTTPS: Unknown (selector passed but not verified)

---

## CRITICAL ISSUES (Security Vulnerabilities)

### 🔴 ISSUE 1: Proxy HTTP Handler Ignores Selector

**Location**: `crates/scred-http/src/http_proxy_handler.rs:120-130`

**Code**:
```rust
let streaming_redactor = StreamingRedactor::new(
    redaction_engine.clone(),
    streaming_config,
    // ❌ MISSING: No selector parameter passed
);
let (redacted_request, redaction_stats) = streaming_redactor.redact_buffer(full_request.as_bytes());
```

**Problem**: 
- Function receives `redaction_engine: Arc<RedactionEngine>`
- Engine potentially has selector from CLI/config
- StreamingRedactor constructor doesn't accept selector parameter
- Result: Selector is IGNORED, ALL patterns redacted

**Impact**:
```
User: scred-proxy --redact CRITICAL,API_KEYS
Request: "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
Expected: JWT passed through (not in CRITICAL,API_KEYS)
Actual: JWT redacted to [REDACTED] (over-redaction)

User: scred-proxy --redact CRITICAL (disable API_KEYS)
Request: "Authorization: sk-openai-3e4d5f6g7h8i9j0k..."
Expected: OpenAI key redacted (it's in API_KEYS, user wants CRITICAL only)
Actual: OpenAI key passed through (secrets leaked!)
```

**Scope**: Affects ALL HTTP/1.1 proxy requests

---

### 🔴 ISSUE 2: MITM HTTP Handler Missing Selector

**Location**: `crates/scred-mitm/src/mitm/proxy.rs:187-194`

**Code**:
```rust
if let Err(e) = crate::mitm::http_handler::handle_http_proxy(
    socket_read,
    socket_write,
    &line,
    redaction_engine.clone(),
    upstream_resolver.clone(),
    // ❌ MISSING: detect_patterns and redact_patterns not passed
).await {
    warn!("HTTP proxy handler error: {}", e);
}
```

**Problem**:
- MITM HTTP calls shared `handle_http_proxy()`
- Call site has access to selectors (from config)
- But selectors not passed through
- Handler uses default (ALL patterns)

**Impact**: Same as Issue 1 but in MITM context.

**Inconsistency**: MITM HTTPS (H2) code DOES pass selectors:
```rust
// CORRECT: MITM HTTPS passes selectors
crate::mitm::tls_mitm::handle_tls_mitm(
    ...
    config.proxy.detect_patterns.clone(),    // ✅ Passed
    config.proxy.redact_patterns.clone(),    // ✅ Passed
).await
```

**Result**: Same MITM proxy has INCONSISTENT behavior:
- HTTP requests: Full redaction (selectors ignored)
- HTTPS requests: Selective redaction (selectors used)

**Scope**: Affects MITM HTTP proxy requests, inconsistent with HTTPS

---

### 🔴 ISSUE 3: CLI vs Proxy Inconsistent Defaults

**Location**: 
- CLI: `crates/scred-cli/src/main.rs:41-47`
- Proxy: `crates/scred-proxy/src/main.rs:180-181`

**Code**:
```rust
// CLI (main.rs:41-47)
let detect_str = detect_flag.or_else(...).unwrap_or("ALL");
let redact_str = redact_flag.or_else(...).unwrap_or("CRITICAL,API_KEYS,PATTERNS");

// Proxy (main.rs:180-181)  
let detect_str = detect_flag.or_else(...).unwrap_or_else(|| "CRITICAL,API_KEYS,INFRASTRUCTURE".to_string());
let redact_str = redact_flag.or_else(...).unwrap_or_else(|| "CRITICAL,API_KEYS".to_string());
```

**Problem**: Different default behavior across tools.

| Tool | Redact Default | PATTERNS Tier? | JWT Redacted? |
|------|---|---|---|
| CLI | CRITICAL,API_KEYS,PATTERNS | ✅ YES | ✅ YES |
| Proxy | CRITICAL,API_KEYS | ❌ NO | ❌ NO |

**Impact**: User might test CLI locally and see JWT redacted, but then deploy proxy which doesn't redact JWT.

```
Local testing: env | scred > output.txt
  Result: JWT tokens redacted ✅

Production: scred-proxy
  Result: JWT tokens in logs ❌ (leak!)
```

**Attack Vector**: User assumes "tested locally" means "safe in production", but proxy has different defaults.

---

## HIGH CONSISTENCY ISSUES

### 🟡 ISSUE 4: Four Different Redaction Code Paths

| Path | Component | Selector Support | API |
|------|-----------|---|---|
| **Path 1** | CLI text mode | ✅ YES (ConfigurableEngine) | `detect_and_redact()` |
| **Path 2** | CLI env mode | ❓ UNKNOWN | `redact_env_line_configurable()` |
| **Path 3** | Proxy HTTP/1.1 | ❌ NO (StreamingRedactor) | `redact_buffer()` |
| **Path 4** | MITM HTTPS/H2 | ❓ QUESTIONABLE | `redact_buffer()` + H2Redactor |

**Problem**: Four different implementations, four different behavior possibilities.

**Risk**: 
- Bug in Path 1 doesn't affect Path 3
- Feature in Path 1 might not work in Path 3
- Selector enforcement works in Path 1 but not Path 3
- Impossible to guarantee consistency

**Code Examples**:
```rust
// Path 1: CLI text mode - uses ConfigurableEngine
let config_engine = ConfigurableEngine::new(engine, detect_selector, redact_selector);
config_engine.detect_and_redact(&text)  // ✅ Respects selectors

// Path 3: Proxy - uses StreamingRedactor directly
let streaming_redactor = StreamingRedactor::new(engine, config);
streaming_redactor.redact_buffer(bytes)  // ❌ Ignores selectors
```

---

### 🟡 ISSUE 5: API Mismatch Between CLI and Proxy

**Problem**: CLI and Proxy use completely different APIs.

```rust
// CLI API (main.rs:236-243)
let config_engine = ConfigurableEngine::new(engine, detect_selector, redact_selector);
let result = config_engine.detect_and_redact(text);
// Returns: FilteredRedactionResult { redacted: String, warnings: Vec<RedactionWarning> }

// Proxy API (http_proxy_handler.rs:120-130)
let streaming_redactor = StreamingRedactor::new(engine, config);
let (redacted, stats) = streaming_redactor.redact_buffer(bytes);
// Returns: (Vec<u8>, StreamingStats)
```

**Impact**: 
- Can't reuse code between CLI and Proxy
- Changes to one path don't benefit other
- Bugs must be fixed twice
- Selector support must be implemented separately

**Example**: To add selector support to Proxy:
1. Can't just use CLI's ConfigurableEngine
2. Must implement in StreamingRedactor instead
3. Must handle different input types (String vs &[u8])
4. Must handle different output types (String vs Vec<u8>)

---

### 🟡 ISSUE 6: CLI Env Mode Selector Handling Unknown

**Location**: `crates/scred-cli/src/main.rs:293-313`

**Code**:
```rust
for line in input_str.lines() {
    let redacted = env_mode::redact_env_line_configurable(line, &config_engine);
    output.push_str(&redacted);
    output.push('\n');
}
```

**Unknown**:
1. Does `env_mode::redact_env_line_configurable()` respect selector?
2. What happens to secrets spanning multiple lines?
3. Does it use ConfigurableEngine correctly?

**Risk**: CLI env mode might not respect selectors, creating inconsistency with text mode.

---

### 🟡 ISSUE 7: MITM HTTPS Selector Passing Unverified

**Location**: `crates/scred-mitm/src/mitm/tls_mitm.rs:141-142`

**Code**:
```rust
h2_config.detect_patterns = detect_patterns.clone();
h2_config.redact_patterns = redact_patterns.clone();
```

**Question**: Are these selectors actually USED by H2MitmHandler?

**Uncertainty**:
- Selectors are assigned to config
- Config passed to H2MitmHandler
- But does H2MitmHandler use them when redacting?

**Risk**: MITM HTTPS might ignore selectors too, creating false sense of working selector support.

---

## ATTACK SCENARIOS

### Scenario A: JWT Token Leak in Production

```
Setup:
  User: "I want to redact only CRITICAL and API_KEYS, not JWT (too noisy)"
  Command: scred-proxy --redact CRITICAL,API_KEYS
  Environment: Authorization headers contain JWT tokens

Expected: JWT tokens passed through (not redacted)

Actual: 
  - If Proxy is used directly: JWT passed through ✅ (matches expectation, but selector not checked!)
  - But if CLI is used locally to test: JWT redacted ❌ (false positive test)
  - User thinks it works, deploys proxy, JWT leaks

Root Cause: 
  - Proxy doesn't use selector (redacts nothing when selector invalid)
  - CLI does use selector (redacts correctly)
  - Both appear to work but behave differently
```

### Scenario B: Sensitive Key Over-Redacted in Development

```
Setup:
  Developer testing locally with: env | scred --redact CRITICAL
  Request contains: "X-Custom-Secret: my-app-api-key-12345" (custom pattern, PATTERNS tier)

Expected: Key passed through (not in CRITICAL tier)

Actual: Key redacted ✅ (CLI respects selector)

Then in Production Proxy:
  Same secret with: scred-proxy --redact CRITICAL
  But Proxy redacts ALL patterns: Key redacted

Result: Inconsistent behavior between dev and production
```

### Scenario C: Security Team Audit Blind Spot

```
Setup:
  Security team audits three tools separately:
  1. Audits CLI: Selector support ✅ Works perfectly
  2. Audits Proxy: Selector support ❌ Ignored (but not noticed)
  3. Audits MITM: Selector support ❓ Unknown

Assumption: All tools have same selector support (CI audit passed)

Reality: Proxy doesn't use selectors, selectors don't affect redaction

Impact: False confidence in selector feature
```

---

## TESTING GAPS

### Missing Test 1: Cross-Tool Selector Consistency

No test that verifies:
```
For each pattern P:
  For each tier T:
    For each selector S:
      CLI --redact=S → redacts P?
      Proxy --redact=S → redacts P?
      MITM --redact=S → redacts P?
      All three agree?
```

### Missing Test 2: Default Behavior Consistency

No test that verifies:
```
Input: Text with JWT token (PATTERNS tier)
Test 1: CLI with no flags → JWT redacted?
Test 2: Proxy with no flags → JWT redacted?
Result: Both same?
```

### Missing Test 3: Boundary Cases

No test for:
- Secret at 64KB chunk boundary
- Secret split across HTTP headers/body
- Secret spanning multiple lines in env mode

---

## ROOT CAUSE ANALYSIS

### Why Proxy Ignores Selector

**Design Decision**: StreamingRedactor was intended to be simple and apply ALL patterns.

**Assumption**: "Selector support is future work"

**Consequence**: Proxy/MITM appear to support selectors (accept flag), but don't use them.

### Why Defaults Differ

**Design Decision**: CLI defaults to "detect broad, redact conservatively"

**Later Iteration**: Proxy defaults to different values, inconsistency not caught

### Why HTTP vs HTTPS Inconsistent

**Evolution**: 
1. H2 MITM handler designed with selector support
2. HTTP handler designed without selector support  
3. Both wrapped in MITM but only H2 has selectors
4. Inconsistency not noticed in code review

---

## RECOMMENDATIONS (Priority Order)

### P1: Fix Selector Not Being Used (CRITICAL)

**Task 1.1**: Update `StreamingRedactor` to accept selector
```rust
// Before
impl StreamingRedactor {
    pub fn new(engine: Arc<RedactionEngine>, config: StreamingConfig) -> Self

// After  
impl StreamingRedactor {
    pub fn new(
        engine: Arc<RedactionEngine>,
        config: StreamingConfig,
        selector: Option<PatternSelector>,  // ← New parameter
    ) -> Self
```

**Task 1.2**: Update `handle_http_proxy()` to accept and use selector
```rust
// Before
pub async fn handle_http_proxy(
    ...
    redaction_engine: Arc<RedactionEngine>,
    ...
)

// After
pub async fn handle_http_proxy(
    ...
    redaction_engine: Arc<RedactionEngine>,
    redact_selector: Option<PatternSelector>,  // ← New parameter
    ...
)
```

**Task 1.3**: Update MITM HTTP handler to pass selector
```rust
// Before
handle_http_proxy(socket_read, socket_write, &line, engine, resolver)

// After
handle_http_proxy(socket_read, socket_write, &line, engine, resolver, Some(config.proxy.redact_patterns))
```

### P2: Harmonize Defaults (CRITICAL)

**Decision**: Choose single default for ALL tools
```
Option A: CLI default = CRITICAL,API_KEYS,PATTERNS
  Pro: Most protective
  Con: Over-redacts

Option B: Proxy default = CRITICAL,API_KEYS,INFRASTRUCTURE
  Pro: Less noisy
  Con: Misses some secrets

Recommendation: Option B (less noisy in production logs)
Document: "PATTERNS tier excluded by default to reduce noise"
```

**Action**: Update CLI to match Proxy
```rust
// CLI (currently CRITICAL,API_KEYS,PATTERNS) 
// Change to: CRITICAL,API_KEYS
// Add: --include-patterns flag for users who want to detect JWT/Bearer
```

### P3: Unify APIs (HIGH)

**Goal**: Single redaction path for all tools

**Option A**: Migrate CLI to StreamingRedactor
```rust
// Remove ConfigurableEngine from CLI
// Use StreamingRedactor like Proxy/MITM
// Requires: StreamingRedactor must support selectors (P1)
```

**Option B**: Migrate Proxy/MITM to ConfigurableEngine
```rust
// Use ConfigurableEngine in Proxy/MITM
// ConfigurableEngine already works with selectors
// Requires: Handle &[u8] vs String conversion
```

**Recommendation**: Option A (StreamingRedactor better for streaming use cases)

### P4: Verify Env Mode Selector Support (HIGH)

**Action**: Review and test `env_mode::redact_env_line_configurable()`

**Verify**:
1. Does it respect selector passed through config_engine?
2. Are multi-line secrets handled correctly?
3. Are edge cases at line boundaries handled?

### P5: Add Test Coverage (HIGH)

**New Tests**:
1. `test_cross_tool_selector_consistency`
2. `test_default_behavior_cli_proxy_mitm`
3. `test_boundary_cases_env_mode`
4. `test_http_vs_https_selector_consistency`

---

## CONCLUSION

SCRED v2.0.0 has **3 CRITICAL security gaps** where selectors are ignored, potentially leading to:
- Over-redaction (unnecessary noise)
- Under-redaction (secrets leak)
- Inconsistent behavior across tools

Additionally, **4 HIGH consistency issues** create maintenance burden and increase risk of similar bugs in future.

**Estimated Severity**: HIGH - Affects all three tools, impacts production deployments

**Estimated Fix Effort**: 
- P1 (Fix selector usage): 2-3 hours
- P2 (Harmonize defaults): 30 minutes
- P3 (Unify APIs): 3-4 hours
- P4 (Verify env mode): 1-2 hours
- P5 (Tests): 2-3 hours
- **Total**: 8-13 hours

**Recommendation**: Fix P1+P2 before next release. P3-P5 can be phased.

