# SCRED Code Pattern Assessment - Crate Organization Review

## Observation: Configuration Related Code is Scattered

### Problem: Related Functionality Split Across Crates

**Configuration Logic** (Should be together):
- CLI config parsing: `scred-cli/src/main.rs`
- Proxy config parsing: `scred-proxy/src/main.rs`
- HTTP config abstraction: `scred-http/src/` (partial)
- MITM config parsing: `scred-mitm/src/main.rs`

**Selector/Pattern Selection** (Should be together):
- PatternSelector type: `scred-http/src/pattern_selector.rs`
- Pattern metadata: `scred-http/src/pattern_metadata.rs`
- CLI selector usage: `scred-cli/src/env_mode.rs` (partial)
- Proxy selector handling: `scred-proxy/src/main.rs` (partial, Issue #1)
- MITM selector handling: `scred-mitm/src/main.rs` (partial, Issue #5)

**Redaction Engine Abstractions**:
- Raw engine: `scred-redactor/src/lib.rs` (RedactionEngine)
- Configurable wrapper: `scred-http/src/configurable_engine.rs` (NEW - ConfigurableEngine)
- CLI usage: `scred-cli/src/main.rs`
- Proxy usage: `scred-proxy/src/main.rs` (being fixed in Issue #1)
- MITM usage: `scred-mitm/src/main.rs` (Issue #5)

### Crate-Level Assessment

#### ✅ scred-http (Shared Library)
**Role**: Common HTTP handling
**Status**: Good foundation, but needs more abstractions moved here
**Should contain**:
- ✅ Pattern selector logic
- ✅ Configurable engine wrapper
- ✅ HTTP proxy handler
- ⏳ Configuration structs/traits (not yet)

#### ⏳ scred-cli (CLI Application)
**Role**: Command-line interface
**Status**: Works, but contains some config logic
**Issues**:
- Config parsing duplicated from other binaries
- Selector logic partially in main.rs

#### ⏳ scred-proxy (HTTP Proxy)
**Role**: Transparent proxy
**Status**: In progress (P0 fixes)
**Issues**:
- Configuration precedence was broken (NOW FIXED - P0#3)
- Invalid selector handling fixed (NOW FIXED - P0#2)
- Selectors not enforced (Issue #1 - PARTIAL)
- Config loading is binary-specific (should share with CLI/MITM)

#### ⏳ scred-mitm (TLS MITM Proxy)
**Role**: MITM proxy for HTTPS
**Status**: Works but may have same issues as Proxy
**Issues**:
- Unknown selector usage (Issue #5 - not reviewed yet)
- Likely has same precedence/config issues as Proxy

---

## Recommendations: Code Organization

### 1. Extract Common Configuration Abstraction
**Proposed new module**: `scred-config` (or expand scred-http)

Should contain:
```rust
// Configuration sources
trait ConfigSource {
    fn load() -> Result<ProxyConfig>;
}

impl ConfigSource for DefaultConfig { ... }
impl ConfigSource for FileConfig { ... }
impl ConfigSource for EnvConfig { ... }
impl ConfigSource for CliArgs { ... }

// Configuration builder with precedence
struct ConfigBuilder {
    layers: Vec<Box<dyn ConfigSource>>,
}

impl ConfigBuilder {
    fn load_defaults() -> Self
    fn add_file(path: &str) -> Self
    fn add_env() -> Self
    fn add_cli(args: &[String]) -> Self
    fn build() -> Result<ProxyConfig>
}
```

**Benefits**:
- Single source of truth for config logic
- Same behavior across CLI, Proxy, MITM
- Easy to test configuration in isolation
- DRY principle - no duplication

**Current Implementation**: scred-proxy/src/main.rs uses inline merging
**Refactoring**: Extract to scred-config or scred-http, import everywhere

---

### 2. Selector Usage Pattern - Ensure Consistency

**Pattern to standardize**:
```rust
// All three should use this same pattern:
let config_engine = ConfigurableEngine::new(
    redaction_engine,
    config.detect_selector.clone(),
    config.redact_selector.clone(),
);

match config.redaction_mode {
    Mode::Detect => {
        let detected = config_engine.detect_only(&text);
        log_detected_secrets(&detected);
        output_unredacted(&text);
    }
    Mode::Redact => {
        let redacted = config_engine.redact_only(&text);
        output_redacted(&redacted);
    }
    Mode::Passthrough => {
        output_unredacted(&text);
    }
}
```

**Status**:
- ✅ CLI: Using ConfigurableEngine correctly
- ⏳ Proxy: ConfigurableEngine created but not integrated (P0#1)
- ❓ MITM: Unknown - needs review (P0#5)

---

### 3. Streaming vs Non-Streaming Redaction

**Issue**: Two different interfaces that should be unified
- StreamingRedactor: For incremental/streaming (Proxy, MITM)
- RedactionEngine + ConfigurableEngine: For whole-text (CLI)

**Current Problem**:
- Streaming functions don't support selectors
- Forces Proxy to not use ConfigurableEngine

**Solutions**:
1. **Create trait** for both
   ```rust
   trait Redactor {
       fn redact(&self, text: &str) -> String;
       fn detect(&self, text: &str) -> Vec<Finding>;
   }
   ```
   
2. **Implement for both**
   - ConfigurableEngine implements Redactor
   - Streaming version of ConfigurableEngine
   
3. **Update streaming functions**
   ```rust
   async fn stream_with_selector(
       config_engine: Arc<dyn Redactor>,
       ...
   ) { ... }
   ```

---

## Current Code Organization (Actual)

```
scred/
├── crates/
│   ├── scred-cli/
│   │   └── src/main.rs          ← Config parsing (CLI specific)
│   │
│   ├── scred-proxy/
│   │   └── src/main.rs          ← Config parsing (Proxy specific)
│   │                              ← Selector application (partial)
│   │
│   ├── scred-mitm/
│   │   └── src/main.rs          ← Config parsing (MITM specific)
│   │                              ← Selector application (unknown)
│   │
│   ├── scred-http/              ← Shared library
│   │   ├── src/configurable_engine.rs    ← ConfigurableEngine ✅
│   │   ├── src/pattern_selector.rs       ← PatternSelector ✅
│   │   ├── src/http_proxy_handler.rs     ← Shared HTTP logic ✅
│   │   └── ...
│   │
│   ├── scred-redactor/
│   │   └── src/lib.rs           ← Base RedactionEngine
│   │
│   └── ...
```

---

## Proposed Code Organization (Recommended)

```
scred/
├── crates/
│   ├── scred-config/            ← NEW: Configuration management
│   │   ├── src/lib.rs
│   │   ├── src/config.rs        ← ProxyConfig struct
│   │   ├── src/sources.rs       ← ConfigSource trait
│   │   ├── src/builder.rs       ← ConfigBuilder
│   │   ├── src/precedence.rs    ← Merging logic
│   │   └── tests/               ← All config tests
│   │
│   ├── scred-http/              ← Shared HTTP
│   │   ├── src/redactor.rs      ← Redactor trait
│   │   ├── src/configurable_engine.rs
│   │   ├── src/pattern_selector.rs
│   │   ├── src/http_proxy_handler.rs
│   │   ├── src/streaming.rs     ← Streaming redactor
│   │   └── ...
│   │
│   ├── scred-cli/
│   │   └── src/main.rs          ← Uses scred-config
│   │
│   ├── scred-proxy/
│   │   └── src/main.rs          ← Uses scred-config
│   │
│   ├── scred-mitm/
│   │   └── src/main.rs          ← Uses scred-config
│   │
│   └── ...
```

---

## Migration Path (Prioritized)

### Phase 1: Extract Config (1-2 hours)
1. Create `scred-config` crate
2. Move ProxyConfig struct there
3. Add ConfigBuilder implementation
4. Update imports in proxy/cli/mitm

### Phase 2: Unify Selector Handling (1-2 hours)
1. Create Redactor trait in scred-http
2. Implement for ConfigurableEngine
3. Implement for streaming redactor
4. Update stream functions to use trait

### Phase 3: Fix P0 Issues with New Structure (1 hour)
1. Issue #1: Use ConfigurableEngine with trait
2. Issue #4: Use ConfigBuilder for detect
3. Issue #5: Review MITM with same tools

---

## Code Pattern Summary

### ✅ Good Patterns (Replicate)
- ConfigurableEngine: Clean wrapper with selector support
- PatternSelector: Type-safe selector definition
- ConfigurationMerging: Precedence-based merging (P0#3 solution)

### ⚠️ Anti-Patterns (Avoid)
- Duplicate config parsing in each binary
- Type-specific function signatures (Arc<StreamingRedactor> not trait)
- Silent fallback on errors
- Mixing selector logic with other concerns

### ⏳ In Progress
- Selector enforcement in Proxy (P0#1)
- Consistent config across all binaries (P0#3 - DONE for proxy)
- MITM selector verification (P0#5)

