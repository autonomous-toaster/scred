# COMPREHENSIVE IMPLEMENTATION ASSESSMENT

**Goal**: Make MITM and Proxy consistent with CLI while keeping streaming first-class
**Approach**: Fix selector enforcement without breaking streaming
**Constraint**: Streaming must remain the only/primary way for HTTP

---

## CURRENT STATE ANALYSIS

### Three Redaction Engines

1. **RedactionEngine** (scred-redactor/src/redactor.rs)
   - Method: `pub fn redact(&self, text: &str) -> RedactionResult`
   - Behavior: Applies ALL patterns regardless of selector
   - Usage: Proxy/MITM HTTP, MITM H2
   - Problem: NO selector support

2. **ConfigurableEngine** (scred-http/src/configurable_engine.rs)
   - Methods: `detect_only()`, `redact_only()`, `detect_and_redact()`
   - Behavior: Respects selectors
   - Usage: CLI only
   - Problem: NOT integrated into Proxy/MITM

3. **StreamingRedactor** (scred-redactor/src/streaming.rs)
   - Methods: `redact_buffer()`, `process_chunk()`
   - Behavior: Streaming optimized, NO selector support
   - Usage: Proxy/MITM streaming
   - Problem: NO selector support

### Three Code Paths Using HTTP Redaction

1. **Proxy Streaming** (scred-proxy/src/main.rs)
   - Uses: StreamingRedactor (no selectors)
   - Status: Broken

2. **Proxy HTTP Handler** (scred-http/src/http_proxy_handler.rs)
   - Uses: RedactionEngine (no selectors)
   - Status: Broken
   - Note: Also used by MITM HTTP

3. **MITM H2/HTTPS** (scred-mitm/src/mitm/h2_mitm_handler.rs)
   - Uses: RedactionEngine (no selectors)
   - Status: Broken

---

## THE FUNDAMENTAL PROBLEM

**Issue**: RedactionEngine and StreamingRedactor don't support selectors
**Root Cause**: They were designed before ConfigurableEngine
**Current Solution**: ConfigurableEngine wraps RedactionEngine but adds complexity

**The Challenge**: 
- ConfigurableEngine works with whole strings: `detect_only(&str) -> Vec<Warning>`
- StreamingRedactor works with chunks: `process_chunk(&[u8]) -> (Vec<u8>, Stats)`
- These are fundamentally different patterns

---

## SOLUTION ARCHITECTURE

### Core Principle: ONE Engine, Selector-Aware from Start

**Idea**: Enhance RedactionEngine and StreamingRedactor to support selectors directly

This avoids:
- Wrapper complexity (ConfigurableEngine)
- Type system conflicts
- Three parallel implementations
- Dead code and unused variables

### Step 1: Add Selector Support to RedactionEngine

```rust
impl RedactionEngine {
    pub fn new(config: RedactionConfig) -> Self { ... }
    
    // NEW: With selector support
    pub fn with_selector(config: RedactionConfig, selector: PatternSelector) -> Self {
        Self {
            config,
            selector: Some(selector),
            ...
        }
    }
    
    // EXISTING: Works as before (all patterns)
    pub fn redact(&self, text: &str) -> RedactionResult { ... }
    
    // NEW: Respects selector
    pub fn redact_with_selector(&self, text: &str) -> RedactionResult {
        // Use self.selector to filter patterns before redacting
    }
}
```

### Step 2: Add Selector Support to StreamingRedactor

```rust
impl StreamingRedactor {
    pub fn new(engine: Arc<RedactionEngine>, config: StreamingConfig) -> Self { ... }
    
    // NEW: With selector support
    pub fn with_selector(
        engine: Arc<RedactionEngine>,
        config: StreamingConfig,
        selector: PatternSelector,
    ) -> Self {
        Self {
            engine,
            config,
            selector: Some(selector),
            ...
        }
    }
    
    // EXISTING: Works as before
    pub fn redact_buffer(&self, data: &[u8]) -> (String, StreamingStats) { ... }
    
    // NEW: Respects selector
    pub fn redact_buffer_with_selector(&self, data: &[u8]) -> (String, StreamingStats) {
        // Use self.selector to filter patterns
    }
}
```

### Step 3: Update http_proxy_handler to Use Selector-Aware Engine

```rust
pub async fn handle_http_proxy(
    ...
    redaction_engine: Arc<RedactionEngine>,
    detect_selector: PatternSelector,
    redact_selector: PatternSelector,
    ...
) {
    let redacted_request = redaction_engine.redact_with_selector_pair(
        &full_request,
        RedactionMode::Redact,
        &redact_selector,
    );
    ...
}
```

### Step 4: Update Proxy/MITM to Pass Selectors Through

```rust
// Proxy main.rs
let redactor = Arc::new(
    StreamingRedactor::with_selector(
        redaction_engine.clone(),
        streaming_config,
        config.redact_selector.clone(),
    )
);

// Use redactor.redact_buffer_with_selector()
```

---

## PHASE-BY-PHASE IMPLEMENTATION PLAN

### PHASE 1: Extend RedactionEngine (1 hour)

**Files to modify**:
- `crates/scred-redactor/src/redactor.rs`

**Changes**:
1. Add optional `selector: Option<PatternSelector>` field
2. Add `pub fn with_selector()` constructor
3. Add `pub fn redact_with_selector()` method
4. Implement selector filtering logic

**Tests**:
- Unit tests for selector-aware redaction
- Verify non-selector mode still works

**Outcome**: RedactionEngine can now enforce selectors

---

### PHASE 2: Extend StreamingRedactor (1 hour)

**Files to modify**:
- `crates/scred-redactor/src/streaming.rs`

**Changes**:
1. Add optional `selector: Option<PatternSelector>` field
2. Add `pub fn with_selector()` constructor
3. Add `pub fn redact_buffer_with_selector()` method
4. Modify `process_chunk()` to respect selector

**Tests**:
- Unit tests for streaming selector support
- Verify chunked redaction respects selector

**Outcome**: StreamingRedactor can now enforce selectors

---

### PHASE 3: Update http_proxy_handler (1.5 hours)

**Files to modify**:
- `crates/scred-http/src/http_proxy_handler.rs`

**Changes**:
1. Add parameters: `detect_selector`, `redact_selector`
2. Update handler signature:
   ```rust
   pub async fn handle_http_proxy(
       ...,
       redaction_engine: Arc<RedactionEngine>,
       detect_selector: Option<PatternSelector>,
       redact_selector: Option<PatternSelector>,
       ...,
   )
   ```
3. Create selector-aware engine:
   ```rust
   let engine = if let Some(selector) = redact_selector {
       redaction_engine.with_selector(selector)
   } else {
       // Use default (all patterns)
       redaction_engine.clone()
   };
   ```
4. Use selector-aware methods for redaction

**Tests**:
- Verify selector filtering in http_proxy_handler
- Test with and without selectors

**Outcome**: HTTP handler respects selectors

---

### PHASE 4: Update Proxy Streaming (1.5 hours)

**Files to modify**:
- `crates/scred-proxy/src/main.rs`

**Changes**:
1. Remove unused `_config_engine` variable
2. Create StreamingRedactor with selectors:
   ```rust
   let redactor = Arc::new(
       StreamingRedactor::with_selector(
           redaction_engine.clone(),
           streaming_config,
           config.redact_selector.clone(),
       )
   );
   ```
3. Update streaming functions to use selector-aware methods

**Tests**:
- Verify streaming respects selectors
- Test chunked encoding with selectors

**Outcome**: Proxy streaming respects selectors

---

### PHASE 5: Update Proxy HTTP Handler Call (0.5 hours)

**Files to modify**:
- `crates/scred-proxy/src/main.rs`

**Changes**:
1. When calling `handle_http_proxy()`, pass selectors:
   ```rust
   handle_http_proxy(
       ...,
       redaction_engine.clone(),
       Some(config.detect_selector.clone()),
       Some(config.redact_selector.clone()),
       ...,
   )
   ```

**Outcome**: Proxy HTTP handler receives selectors

---

### PHASE 6: Update MITM HTTP Handler (1 hour)

**Files to modify**:
- `crates/scred-mitm/src/mitm/http_handler.rs`
- `crates/scred-mitm/src/mitm/proxy.rs`

**Changes**:
1. Update signature to accept selectors:
   ```rust
   pub async fn handle_http_proxy(
       ...,
       detect_selector: PatternSelector,
       redact_selector: PatternSelector,
   )
   ```
2. Pass selectors to shared http_proxy_handler

**Outcome**: MITM HTTP handler passes selectors

---

### PHASE 7: Update MITM TLS/H2 Handler (1 hour)

**Files to modify**:
- `crates/scred-mitm/src/mitm/h2_mitm_handler.rs`

**Changes**:
1. Use selector-aware engine:
   ```rust
   let engine = if let Some(selector) = redact_selector {
       RedactionEngine::new(config).with_selector(selector)
   } else {
       RedactionEngine::new(config)
   };
   ```
2. Replace unused parameters with functional code
3. Actually USE detect_patterns and redact_patterns

**Outcome**: MITM H2 handler respects selectors

---

### PHASE 8: Remove Dead Code / Cleanup (1 hour)

**Files to modify**:
- `crates/scred-http/src/configurable_engine.rs` (optional: keep as legacy)
- `crates/scred-proxy/src/main.rs` (remove TODO comments)

**Changes**:
1. Remove unused ConfigurableEngine from Proxy
2. Remove dead code markers (`_config_engine`)
3. Clean up comments about broken selector support

**Outcome**: Cleaner, no dead code

---

### PHASE 9: Integration Tests (1.5 hours)

**Files to create**:
- `crates/scred-proxy/tests/selector_consistency_tests.rs`
- `crates/scred-mitm/tests/selector_consistency_tests.rs`

**Tests**:
1. CLI vs Proxy vs MITM output comparison
   - Same input file
   - Same selector config
   - All three tools should produce identical output
2. Selector filtering verification
   - `--redact CRITICAL` should only redact CRITICAL tier
   - Should NOT redact API_KEYS, INFRASTRUCTURE, etc.
3. Streaming + selector interaction
   - Chunked encoding with selectors
   - Multiple chunks with same pattern
4. Per-tool specific tests
   - HTTP/1.1 (Proxy + MITM)
   - Streaming (Proxy)
   - H2/HTTPS (MITM)

**Outcome**: Verified consistency across all tools

---

## IMPLEMENTATION ROADMAP

```
Phase 1: RedactionEngine selector support (1h)
   ↓
Phase 2: StreamingRedactor selector support (1h)
   ↓
Phase 3: http_proxy_handler selector integration (1.5h)
   ↓
Phase 4: Proxy streaming selector integration (1.5h)
   ↓
Phase 5: Proxy HTTP handler selector passing (0.5h)
   ↓
Phase 6: MITM HTTP handler selector integration (1h)
   ↓
Phase 7: MITM H2 handler selector integration (1h)
   ↓
Phase 8: Cleanup dead code (1h)
   ↓
Phase 9: Integration tests (1.5h)
   
TOTAL: 10 hours (1.5 business days)
```

---

## VERIFICATION CHECKLIST

### Unit Tests
- [ ] RedactionEngine.redact_with_selector() filters correctly
- [ ] StreamingRedactor processes chunks with selector
- [ ] Selectors don't affect non-selector redaction

### Integration Tests
- [ ] CLI output == Proxy output (same selectors)
- [ ] CLI output == MITM output (same selectors)
- [ ] Proxy output == MITM output (same selectors)
- [ ] Streaming chunks redacted same as buffered

### Selector-Specific Tests
- [ ] --redact CRITICAL only redacts CRITICAL tier
- [ ] --redact CRITICAL,API_KEYS redacts both
- [ ] --detect and --redact are independent
- [ ] Empty selector means all patterns

### Regression Tests
- [ ] Existing 354+ tests still pass
- [ ] Non-selector mode unchanged
- [ ] Performance not degraded
- [ ] Streaming still works

---

## DEAD CODE TO REMOVE

After implementation:

1. `scred-proxy/src/main.rs` line 424-426
   ```rust
   let _config_engine = Arc::new(ConfigurableEngine::new(...));
   // REMOVE: Now using StreamingRedactor::with_selector()
   ```

2. `scred-http/src/configurable_engine.rs`
   - Optional: Keep for backward compat OR remove if no longer needed
   - Decision: Keep but mark deprecated with note

3. Comments about "selector not working" throughout
   - Remove TODO comments
   - Clean up explanatory comments

---

## SECURITY GUARANTEES AFTER FIX

After full implementation:

✅ **CLI**: Respects selectors (already working)
✅ **Proxy HTTP**: Respects selectors (fixed)
✅ **Proxy Streaming**: Respects selectors (fixed)
✅ **MITM HTTP**: Respects selectors (fixed)
✅ **MITM H2/HTTPS**: Respects selectors (fixed)

**Result**: Consistent behavior across all tools and code paths
**Guarantee**: User-configured selectors are ALWAYS enforced
**Safety**: No silent configuration bypass

---

## ARCHITECTURAL BENEFITS

1. **Single Source of Truth**
   - Selector logic in RedactionEngine/StreamingRedactor
   - No wrapper complexity

2. **Consistent API**
   - Same selector support everywhere
   - Same behavior guaranteed

3. **No Dead Code**
   - ConfigurableEngine removed or marked legacy
   - All variables used meaningfully

4. **First-Class Streaming**
   - Streaming gets same selector support as buffering
   - No second-class treatment

5. **Type Safety**
   - No trait objects needed
   - Compiler enforces selector passing

---

## RISK ASSESSMENT

**Risk Level**: LOW

Why:
- Changes are additive (add new methods, keep old ones)
- Backward compatible (optional selector parameter)
- Test coverage comprehensive
- Existing 354+ tests provide regression safety

**Mitigation**:
- Each phase tested independently
- Integration tests verify consistency
- Gradual rollout (CLI → Proxy → MITM)

