# 🚀 Implementation Plan: scred-http-detector & scred-http-redactor

## Phase Overview

**Total Duration**: 12-17 hours (estimated 2 working days)
**Start**: 2026-03-22 20:40 UTC
**Phases**: 5 (sequential, each builds on previous)

---

## Phase 1: Crate Scaffolding & Structure (2-3 hours)

### Objectives
- Create two new crate directories
- Set up Cargo.toml files with proper dependencies
- Create module structure
- Verify compilation

### Tasks

#### 1.1: Create crate directories
```bash
mkdir -p crates/scred-http-detector/src
mkdir -p crates/scred-http-redactor/src
```

#### 1.2: Create Cargo.toml files

**scred-http-detector/Cargo.toml**:
- name: scred_http_detector
- version: 0.1.0
- Dependencies: 
  - scred-pattern-detector
  - http crate
  - anyhow, tracing, serde

**scred-http-redactor/Cargo.toml**:
- name: scred_http_redactor
- version: 0.1.0
- Dependencies:
  - scred-redactor
  - http crate
  - h2 crate
  - tokio (async)
  - anyhow, tracing, serde

#### 1.3: Create module structure

**scred-http-detector/src/lib.rs**:
```
pub mod analyzer;
pub mod classification;
pub mod header_analysis;
pub mod body_analysis;
pub mod models;

pub use analyzer::ContentAnalyzer;
pub use classification::{Sensitivity, RedactionStrategy};
pub use models::{AnalysisResult, Finding};
```

**scred-http-redactor/src/lib.rs**:
```
pub mod core;
pub mod header_redaction;
pub mod body_redaction;
pub mod streaming_redaction;
pub mod protocol;
pub mod models;

pub use core::HttpRedactor;
pub use protocol::{Http11Redactor, H2Redactor};
```

#### 1.4: Create placeholder modules
- Empty files for all modules (will fill in phases 2-3)
- Add module declarations to lib.rs

#### 1.5: Update root Cargo.toml
```toml
[workspace]
members = [
    "crates/scred-cli",
    "crates/scred-http",
    "crates/scred-http-detector",    # NEW
    "crates/scred-http-redactor",    # NEW
    "crates/scred-mitm",
    "crates/scred-proxy",
    "crates/scred-pattern-detector",
    "crates/scred-redactor",
]
```

#### 1.6: Verify compilation
```bash
cargo check
```

### Deliverable
- ✅ Two new crates with proper structure
- ✅ All dependencies resolved
- ✅ Compilation succeeds (with warnings about unused code is fine)

---

## Phase 2: Detection Layer Implementation (3-4 hours)

### Objectives
- Implement scred-http-detector
- Core analysis functionality
- Content classification
- Header & body analysis

### Tasks

#### 2.1: Implement models (models.rs, ~100 LOC)

```rust
pub enum Sensitivity {
    Public,           // No redaction needed
    Internal,         // Partial redaction
    Confidential,     // Full redaction
    Secret,           // Custom rules
}

pub struct AnalysisResult {
    pub content_type: ContentType,
    pub sensitivity: Sensitivity,
    pub redaction_strategy: RedactionStrategy,
    pub findings: Vec<Finding>,
}

pub struct Finding {
    pub path: String,
    pub value: String,
    pub pattern_id: Option<String>,
    pub sensitivity: Sensitivity,
}

pub struct RedactionStrategy {
    pub fields: Vec<FieldRedaction>,
    pub patterns: Vec<PatternRedaction>,
}
```

#### 2.2: Implement ContentAnalyzer trait (analyzer.rs, ~150 LOC)

```rust
pub trait ContentAnalyzer {
    fn analyze_headers(&self, headers: &[HttpHeader]) -> AnalysisResult;
    fn analyze_body(&self, body: &str, content_type: &str) -> AnalysisResult;
}

pub struct HttpContentAnalyzer {
    patterns: Arc<PatternMatcher>,
    header_analyzer: HeaderAnalyzer,
    body_analyzer: BodyAnalyzer,
}

impl ContentAnalyzer for HttpContentAnalyzer {
    // Implementations
}
```

#### 2.3: Implement HeaderAnalyzer (header_analysis.rs, ~80 LOC)

```rust
pub struct HeaderAnalyzer;

impl HeaderAnalyzer {
    pub fn detect_sensitive_headers(&self, headers: &[HttpHeader]) 
        -> Vec<(String, Sensitivity)>;
    
    pub fn classify_header(&self, name: &str, value: &str) 
        -> Sensitivity;
}
```

Known sensitive headers:
- Authorization, Cookie, Set-Cookie
- X-API-Key, X-Auth-Token, X-Access-Token
- X-CSRF-Token, X-Session-ID
- WWW-Authenticate, Proxy-Authenticate

#### 2.4: Implement BodyAnalyzer (body_analysis.rs, ~120 LOC)

```rust
pub struct BodyAnalyzer;

impl BodyAnalyzer {
    pub fn detect_content_type(&self, body: &[u8], headers: &[HttpHeader]) 
        -> ContentType;
    
    pub fn analyze_json(&self, body: &str) -> JsonAnalysis;
    pub fn analyze_xml(&self, body: &str) -> XmlAnalysis;
    pub fn analyze_form(&self, body: &str) -> FormAnalysis;
}
```

#### 2.5: Implement classification logic (classification.rs, ~80 LOC)

```rust
pub fn classify_sensitivity(
    header_name: &str,
    body_path: &str,
    pattern_match: Option<&str>,
) -> Sensitivity;
```

#### 2.6: Tests (detector/tests/, ~100 LOC)

```rust
#[test]
fn test_authorization_header_detected() {}

#[test]
fn test_json_password_field_detected() {}

#[test]
fn test_xml_sensitive_element_detected() {}
```

### Deliverable
- ✅ scred-http-detector compiles
- ✅ Core analysis functionality working
- ✅ Tests passing (minimal coverage)

---

## Phase 3: Redaction Layer Implementation (4-5 hours)

### Objectives
- Implement scred-http-redactor
- Header redaction strategies
- Body redaction strategies
- Protocol-specific redactors (Http11Redactor, H2Redactor)

### Tasks

#### 3.1: Implement core trait (core.rs, ~50 LOC)

```rust
pub trait HttpRedactor {
    fn redact_request(&self, request: &mut HttpRequest) -> Result<()>;
    fn redact_response(&self, response: &mut HttpResponse) -> Result<()>;
    fn redact_headers(&self, headers: &mut [HttpHeader]) -> Result<()>;
    fn redact_body(&self, body: &mut String) -> Result<()>;
}
```

#### 3.2: Implement HeaderRedactor (header_redaction.rs, ~120 LOC)

```rust
pub struct HeaderRedactor {
    engine: Arc<RedactionEngine>,
}

impl HeaderRedactor {
    pub fn redact_headers(&self, headers: &mut [HttpHeader]) -> Result<()>;
    
    pub fn redact_authorization(&self, value: &str) -> String;
    pub fn redact_cookie(&self, value: &str) -> String;
    pub fn redact_custom_header(&self, name: &str, value: &str) -> String;
}
```

#### 3.3: Implement BodyRedactor (body_redaction.rs, ~150 LOC)

```rust
pub struct BodyRedactor {
    engine: Arc<RedactionEngine>,
}

impl BodyRedactor {
    pub fn redact_json(&self, body: &str) -> Result<String>;
    pub fn redact_xml(&self, body: &str) -> Result<String>;
    pub fn redact_form(&self, body: &str) -> Result<String>;
    pub fn redact_plain(&self, body: &str) -> Result<String>;
    pub fn redact_binary(&self, body: &[u8]) -> Result<Vec<u8>>;
}
```

#### 3.4: Implement StreamingBodyRedactor (streaming_redaction.rs, ~80 LOC)

```rust
pub struct StreamingBodyRedactor {
    redactor: StreamingRedactor,
    chunk_size: usize,
}

impl StreamingBodyRedactor {
    pub async fn redact_stream<R: AsyncRead, W: AsyncWrite>(
        &self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<RedactionStats>;
}
```

#### 3.5: Implement Http11Redactor (protocol.rs, ~100 LOC)

```rust
pub struct Http11Redactor {
    header_redactor: HeaderRedactor,
    body_redactor: BodyRedactor,
}

impl Http11Redactor {
    pub fn redact_request_line(&self, line: &str) -> String;
    pub fn redact_request(&self, request: &mut HttpRequest) -> Result<()>;
    pub fn redact_response(&self, response: &mut HttpResponse) -> Result<()>;
}
```

#### 3.6: Move h2_adapter → H2Redactor (protocol.rs, ~150 LOC)

**Key steps**:
1. Copy existing h2_adapter/mod.rs code
2. Rename H2MitmAdapter → H2Redactor
3. Refactor to use HeaderRedactor + BodyRedactor composition
4. Adapt per-stream state management
5. Update imports (scred-redactor, http, h2)

**H2Redactor structure**:
```rust
pub struct H2Redactor {
    header_redactor: HeaderRedactor,
    body_redactor: BodyRedactor,
    pseudo_header_rules: PseudoHeaderRules,
}

impl H2Redactor {
    pub fn redact_pseudo_headers(&self, headers: &mut [(Vec<u8>, Vec<u8>)]) 
        -> Result<()>;
    pub fn redact_stream(&self, stream: &mut H2Stream) -> Result<()>;
}
```

#### 3.7: Implement models (models.rs, ~30 LOC)

```rust
pub struct RedactionStats {
    pub headers_redacted: usize,
    pub patterns_found: usize,
    pub bytes_processed: u64,
    pub bytes_redacted: u64,
}
```

#### 3.8: Tests (redactor/tests/, ~150 LOC)

```rust
#[test]
fn test_authorization_header_redacted() {}

#[test]
fn test_json_password_redacted() {}

#[test]
fn test_streaming_redaction() {}

#[test]
fn test_h2_pseudo_headers_redacted() {}
```

### Deliverable
- ✅ scred-http-redactor compiles
- ✅ All redaction strategies working
- ✅ H2Redactor moved and functional
- ✅ Tests passing

---

## Phase 4: Update Existing Crates (2-3 hours)

### Objectives
- Update scred-http (remove h2_adapter, add new dependencies)
- Update scred-mitm (use new redactor)
- Update scred-proxy (use new redactor)
- Ensure all imports work

### Tasks

#### 4.1: Update scred-http/Cargo.toml

```toml
[dependencies]
scred-http-detector = { path = "../scred-http-detector" }
scred-http-redactor = { path = "../scred-http-redactor" }
```

#### 4.2: Update scred-http/src/lib.rs

Remove:
```rust
// pub mod h2_adapter;  // REMOVE THIS
```

Add:
```rust
pub use scred_http_detector::ContentAnalyzer;
pub use scred_http_redactor::HttpRedactor;
```

#### 4.3: Remove h2_adapter directory

```bash
rm -rf crates/scred-http/src/h2_adapter/
```

#### 4.4: Update scred-mitm imports

**In scred-mitm/src/mitm/h2_mitm_handler.rs**:

Before:
```rust
use scred_http::h2_adapter::H2MitmAdapter;
```

After:
```rust
use scred_http_redactor::H2Redactor;
```

Update all references:
- H2MitmAdapter → H2Redactor
- .redact_stream() calls updated if signature changed

#### 4.5: Update scred-proxy imports

**In scred-proxy/src/main.rs**:

Before:
```rust
use scred_http::h2_adapter::H2MitmAdapter;
```

After:
```rust
use scred_http_redactor::H2Redactor;
```

#### 4.6: Verify all crates compile

```bash
cargo check --all
cargo build --release --all
```

#### 4.7: Update Cargo.toml in root

Add new crates to workspace members (if not already done in Phase 1)

### Deliverable
- ✅ All crates compile cleanly
- ✅ No unused imports warnings
- ✅ All imports resolved correctly

---

## Phase 5: Testing & Verification (1-2 hours)

### Objectives
- Full integration testing
- Verify functionality unchanged
- Run all existing tests
- Document any API changes

### Tasks

#### 5.1: Unit tests for new crates

```bash
cargo test -p scred_http_detector
cargo test -p scred_http_redactor
```

Expected: All tests pass

#### 5.2: Integration tests

```bash
cargo test --all
```

Expected: All existing tests still pass

#### 5.3: Manual functionality verification

**Test HTTP/1.1 proxy**:
```bash
cd crates/scred-proxy
cargo run -- --detect
# curl -x 127.0.0.1:8080 http://httpbin.org/anything
```

**Test MITM proxy**:
```bash
cd crates/scred-mitm
cargo run -- 127.0.0.1:8080
# curl --insecure -x https://127.0.0.1:8080 https://httpbin.org/anything
```

**Verify redaction working**:
- Send requests with sensitive headers (Authorization, Cookie)
- Verify they're redacted in logs/output
- Verify upstream doesn't receive redacted data

#### 5.4: Build release binaries

```bash
cargo build --release --all
ls -lh target/release/scred-*
```

Expected:
- scred-mitm compiles to ~4-5M
- scred-proxy compiles to ~4-5M
- No errors, minimal warnings

#### 5.5: Documentation updates

- Update scred-http README to mention detector/redactor
- Add quick start guide for new API usage
- Update examples in detector/redactor crates

### Deliverable
- ✅ All tests passing
- ✅ Functionality verified working
- ✅ Release binaries built successfully
- ✅ Documentation updated

---

## Implementation Checklist

### Phase 1: Scaffolding
- [ ] Create crates/scred-http-detector directory
- [ ] Create crates/scred-http-redactor directory
- [ ] Write Cargo.toml for both crates
- [ ] Create module structure (lib.rs, mod.rs files)
- [ ] Update root Cargo.toml workspace members
- [ ] `cargo check` succeeds

### Phase 2: Detection Layer
- [ ] models.rs: AnalysisResult, Finding, Sensitivity (100 LOC)
- [ ] analyzer.rs: ContentAnalyzer trait & implementation (150 LOC)
- [ ] header_analysis.rs: HeaderAnalyzer (80 LOC)
- [ ] body_analysis.rs: BodyAnalyzer (120 LOC)
- [ ] classification.rs: Sensitivity classification (80 LOC)
- [ ] Add tests (100 LOC)
- [ ] `cargo test -p scred_http_detector` passes

### Phase 3: Redaction Layer
- [ ] core.rs: HttpRedactor trait (50 LOC)
- [ ] header_redaction.rs: HeaderRedactor (120 LOC)
- [ ] body_redaction.rs: BodyRedactor (150 LOC)
- [ ] streaming_redaction.rs: StreamingBodyRedactor (80 LOC)
- [ ] protocol.rs: Http11Redactor (100 LOC)
- [ ] protocol.rs: H2Redactor moved from h2_adapter (150 LOC)
- [ ] models.rs: RedactionStats (30 LOC)
- [ ] Add tests (150 LOC)
- [ ] `cargo test -p scred_http_redactor` passes

### Phase 4: Update Existing Crates
- [ ] Update scred-http/Cargo.toml
- [ ] Update scred-http/src/lib.rs
- [ ] Remove h2_adapter directory
- [ ] Update scred-mitm imports
- [ ] Update scred-proxy imports
- [ ] Update root Cargo.toml
- [ ] `cargo check --all` passes
- [ ] `cargo build --release --all` succeeds

### Phase 5: Testing & Verification
- [ ] `cargo test -p scred_http_detector` passes
- [ ] `cargo test -p scred_http_redactor` passes
- [ ] `cargo test --all` passes (all existing tests)
- [ ] Manual HTTP/1.1 proxy test
- [ ] Manual MITM proxy test
- [ ] Release binaries built
- [ ] Documentation updated
- [ ] Final commit with all changes

---

## Estimated Timeline

| Phase | Task | Time | Status |
|-------|------|------|--------|
| 1 | Scaffolding | 2-3h | 🔵 Ready |
| 2 | Detection | 3-4h | 🔵 Ready |
| 3 | Redaction | 4-5h | 🔵 Ready |
| 4 | Update crates | 2-3h | 🔵 Ready |
| 5 | Testing | 1-2h | 🔵 Ready |
| **Total** | | **12-17h** | 🔵 Ready |

---

## Critical Success Factors

1. ✅ **No H2 in scred-redactor**: Verify zero h2 imports in core redactor module
2. ✅ **Clean compilation**: All crates must compile with zero errors
3. ✅ **Tests passing**: All new AND existing tests must pass
4. ✅ **Functionality preserved**: Same behavior, better organized
5. ✅ **No cycles**: All dependencies flow downward only

---

## Rollback Plan

If critical issues arise:
1. Keep phase-based commits for easy revert
2. Each phase = separate commit
3. Can rollback to Phase N-1 if needed

---

## Starting Now

Ready to begin Phase 1 immediately after approval ✅

