# 🏗️ New Crate Architecture: scred-http-detector & scred-http-redactor

## Current State Analysis

### Current Architecture
```
scred-redactor
  ├─ RedactionEngine (core redaction)
  ├─ StreamingRedactor (bounded memory redaction)
  └─ Pattern detection (Zig-based, compiled FFI)

scred-http
  ├─ h2_adapter/ ← PROBLEM: H2-specific redaction logic here
  ├─ h2/
  ├─ Protocol handlers (HTTP/1.1, H2, h2c)
  └─ Proxy handlers

scred-mitm
  └─ Uses: scred-http.h2_adapter, scred-redactor

scred-proxy
  └─ Uses: scred-http.h2_adapter, scred-redactor
```

### Your Constraint
✋ **"I'm OK with everything, EXCEPT having h2 specific in redactor"**

This means:
- ❌ Don't put h2_adapter in scred-redactor (wrong ownership)
- ✅ Create domain-specific detection & redaction crates for HTTP
- ✅ Separate concerns properly

---

## Proposed Architecture

### New Crates

```
scred-http-detector/          ← NEW
├── Content detection
├── Pattern matching
├── Redaction requirement assessment
├── Header/body analysis
└── (Protocol-agnostic)

scred-http-redactor/          ← NEW  
├── HTTP-specific redaction logic
├── H2-specific redaction logic (moved from h2_adapter)
├── HTTP/1.1-specific redaction logic
├── Header redaction strategies
└── Body redaction strategies

scred-http/                   ← REFACTORED
├── Protocol handling (parser, encoder, reader)
├── Proxy handlers
└── Utilities

scred-redactor/               ← UNCHANGED
├── RedactionEngine (core)
├── StreamingRedactor (streaming)
└── Pattern detection (FFI)

scred-mitm/                   ← SIMPLIFIED
└─ Uses: scred-http, scred-http-redactor, scred-redactor

scred-proxy/                  ← SIMPLIFIED
└─ Uses: scred-http, scred-http-redactor, scred-redactor
```

---

## Detailed Design

### 1. scred-http-detector (NEW)

**Purpose**: Content analysis and redaction requirement detection

**Responsibilities**:
- Analyze incoming HTTP request/response
- Detect what needs redaction
- Classify sensitivity levels
- Suggest redaction strategies
- No actual redaction (just analysis)

**Modules**:

```rust
pub mod analyzer {
    // Core detection logic
    pub trait ContentAnalyzer {
        fn analyze_headers(&self, headers: &[HttpHeader]) -> AnalysisResult;
        fn analyze_body(&self, body: &str) -> AnalysisResult;
    }
    
    pub struct HttpContentAnalyzer {
        patterns: PatternMatcher,  // From scred-pattern-detector
    }
}

pub mod classification {
    // Sensitivity classification
    pub enum Sensitivity {
        Public,           // No redaction needed
        Internal,         // Partial redaction
        Confidential,     // Full redaction
        Secret,           // Custom redaction rules
    }
    
    pub struct RedactionStrategy {
        pub fields: Vec<FieldRedaction>,
        pub patterns: Vec<PatternRedaction>,
        pub custom: Option<CustomRedaction>,
    }
}

pub mod header_analysis {
    // Header-specific detection
    pub struct HeaderAnalyzer;
    
    impl HeaderAnalyzer {
        pub fn detect_sensitive_headers(&self, headers: &[HttpHeader]) -> Vec<(String, Sensitivity)>;
        pub fn classify_header(&self, name: &str, value: &str) -> Sensitivity;
    }
}

pub mod body_analysis {
    // Body content detection
    pub struct BodyAnalyzer;
    
    impl BodyAnalyzer {
        pub fn detect_content_type(&self, body: &[u8], headers: &[HttpHeader]) -> ContentType;
        pub fn analyze_json(&self, body: &str) -> JsonAnalysis;
        pub fn analyze_xml(&self, body: &str) -> XmlAnalysis;
        pub fn analyze_form(&self, body: &str) -> FormAnalysis;
    }
}

pub mod models {
    pub struct AnalysisResult {
        pub content_type: ContentType,
        pub sensitivity: Sensitivity,
        pub redaction_strategy: RedactionStrategy,
        pub findings: Vec<Finding>,
    }
    
    pub struct Finding {
        pub path: String,  // JSON path, XPath, etc.
        pub value: String,
        pub pattern_id: Option<String>,
        pub sensitivity: Sensitivity,
    }
}
```

**Example Usage**:
```rust
let detector = HttpContentAnalyzer::new(pattern_detector);
let request = HttpRequest { ... };
let analysis = detector.analyze_request(&request)?;

if analysis.sensitivity >= Sensitivity::Internal {
    // Need redaction
    let strategy = analysis.redaction_strategy;
    // Pass to redactor
}
```

**Dependencies**:
- `scred-pattern-detector` (for pattern matching)
- `http` crate (for Request/Response types)
- Standard: `anyhow`, `tracing`, `serde`

**LOC**: ~400-500
**Purpose**: Decouple detection from redaction (single responsibility)

---

### 2. scred-http-redactor (NEW)

**Purpose**: HTTP-specific redaction strategies and implementations

**Responsibilities**:
- Redact HTTP headers (request/response)
- Redact HTTP body (JSON, XML, form, plain)
- H2-specific redaction (moved from h2_adapter)
- HTTP/1.1-specific redaction
- Streaming redaction for large bodies
- Per-protocol redaction strategies

**Modules**:

```rust
pub mod core {
    // Core redaction interface
    pub trait HttpRedactor {
        fn redact_request(&self, request: &mut HttpRequest) -> Result<()>;
        fn redact_response(&self, response: &mut HttpResponse) -> Result<()>;
        fn redact_headers(&self, headers: &mut [HttpHeader]) -> Result<()>;
        fn redact_body(&self, body: &mut String) -> Result<()>;
    }
}

pub mod header_redaction {
    // Header-specific redaction
    pub struct HeaderRedactor {
        engine: Arc<RedactionEngine>,
        sensitive_fields: Vec<String>,
    }
    
    impl HeaderRedactor {
        pub fn redact_headers(&self, headers: &mut [HttpHeader]) -> Result<()>;
        pub fn redact_authorization(&self, value: &str) -> String;
        pub fn redact_cookie(&self, value: &str) -> String;
        pub fn redact_custom_header(&self, name: &str, value: &str) -> String;
    }
}

pub mod body_redaction {
    // Body-specific redaction
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
}

pub mod streaming_redaction {
    // Streaming redaction for large bodies
    pub struct StreamingBodyRedactor {
        redactor: StreamingRedactor,
        chunk_size: usize,
    }
    
    impl StreamingBodyRedactor {
        pub async fn redact_stream<R: AsyncRead>(
            &self,
            reader: &mut R,
            writer: &mut W,
        ) -> Result<RedactionStats>;
    }
}

pub mod protocol {
    // Protocol-specific redaction strategies
    
    pub struct Http11Redactor {
        header_redactor: HeaderRedactor,
        body_redactor: BodyRedactor,
    }
    
    impl Http11Redactor {
        pub fn redact_request_line(&self, line: &str) -> String;
        pub fn redact_request(&self, request: &mut HttpRequest) -> Result<()>;
    }
    
    pub struct H2Redactor {
        header_redactor: HeaderRedactor,
        body_redactor: BodyRedactor,
        // H2-specific concerns
        pseudo_header_rules: PseudoHeaderRules,
    }
    
    impl H2Redactor {
        pub fn redact_pseudo_headers(&self, headers: &mut [(Vec<u8>, Vec<u8>)]) -> Result<()>;
        pub fn redact_stream(&self, stream: &mut H2Stream) -> Result<()>;
    }
}

pub mod models {
    pub struct RedactionStats {
        pub headers_redacted: usize,
        pub patterns_found: usize,
        pub bytes_processed: u64,
        pub bytes_redacted: u64,
    }
}
```

**Example Usage**:
```rust
// For HTTP/1.1
let http11_redactor = Http11Redactor::new(engine);
http11_redactor.redact_request(&mut request)?;

// For HTTP/2
let h2_redactor = H2Redactor::new(engine);
h2_redactor.redact_stream(&mut stream)?;

// For streaming large bodies
let streaming_redactor = StreamingBodyRedactor::new(engine, 64*1024);
streaming_redactor.redact_stream(&mut reader, &mut writer).await?;
```

**Dependencies**:
- `scred-redactor` (RedactionEngine, StreamingRedactor)
- `scred-http-detector` (for detection results)
- `http` crate
- `h2` crate (for H2Stream types)
- Standard: `tokio`, `anyhow`, `tracing`

**LOC**: ~600-800
**Purpose**: Central HTTP redaction logic (was scattered in h2_adapter + http_proxy_handler)

---

### 3. scred-http (REFACTORED)

**Purpose**: HTTP protocol handling and proxy utilities

**Changes**:
- Remove h2_adapter (moved to scred-http-redactor)
- Keep protocol handlers (parser, encoder, reader)
- Keep proxy handler orchestration
- Integrate with scred-http-detector and scred-http-redactor

**New structure**:

```rust
pub mod core {
    // Protocol-agnostic HTTP models
    pub use http::{Request, Response, Header, ...};
}

pub mod protocol {
    // Protocol implementations
    pub mod http1 {
        pub struct Http11Parser;
        pub struct Http11Encoder;
        pub struct Http11Reader;
    }
    
    pub mod h2 {
        pub mod alpn;  // Keep minimal
    }
}

pub mod proxy {
    // Proxy handler orchestration
    pub async fn handle_http_proxy_request(
        client: &mut TcpStream,
        detector: &HttpContentAnalyzer,
        redactor: &HttpRedactor,
        upstream: &str,
    ) -> Result<()>;
}

pub mod utils {
    // Utilities (unchanged)
    pub mod duplex;
    pub mod dns_resolver;
    pub mod logging;
}
```

**Dependencies**:
- `scred-http-detector` (NEW)
- `scred-http-redactor` (NEW)
- Remove direct dependency on detailed redaction logic

**Changes**: 
- Remove h2_adapter re-export
- Add detector/redactor re-exports
- Simplify imports/dependencies

---

## Data Flow (Updated)

### Request Processing

```
Client Request
    ↓
HTTP Parser (scred-http::protocol)
    ↓
Content Analyzer (scred-http-detector::analyzer)
    ├─ Detect sensitive data
    ├─ Classify sensitivity
    └─ Suggest redaction strategy
    ↓
HTTP Redactor (scred-http-redactor)
    ├─ Redact headers
    ├─ Redact body (detect format)
    └─ Apply detection results
    ↓
Proxy Handler (scred-http::proxy)
    ├─ Forward redacted request
    └─ Get upstream response
    ↓
Response Processor
    ├─ Parse response
    ├─ Detect sensitive data
    └─ Redact similarly
    ↓
Client Response
```

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    scred-mitm / scred-proxy             │
│                     (Applications)                      │
└────────────┬──────────────┬───────────────────────────┘
             │              │
             ↓              ↓
      ┌─────────────┐   ┌──────────────┐
      │ scred-http  │   │ scred-http   │
      │  (protocol) │   │ -detector    │
      └──────┬──────┘   └──────┬───────┘
             │                 │
             └────────┬────────┘
                      ↓
           ┌──────────────────────┐
           │ scred-http-redactor  │
           │ (protocol-specific   │
           │  redaction strategies)
           └──────────┬───────────┘
                      ↓
           ┌──────────────────────┐
           │  scred-redactor      │
           │ (core redaction,     │
           │  pattern detection)  │
           └──────────────────────┘
```

---

## Crate Dependencies (New)

```
scred-http-detector
  ├─ scred-pattern-detector
  └─ http crate

scred-http-redactor
  ├─ scred-redactor
  ├─ scred-http-detector (optional, for analysis results)
  ├─ http crate
  └─ h2 crate

scred-http
  ├─ scred-http-detector
  ├─ scred-http-redactor
  └─ http crate

scred-mitm
  ├─ scred-http
  ├─ scred-http-redactor  (for direct access if needed)
  └─ scred-redactor

scred-proxy
  ├─ scred-http
  ├─ scred-http-redactor  (for direct access if needed)
  └─ scred-redactor
```

---

## Integration Points

### scred-mitm Integration

```rust
// scred-mitm/src/mitm/h2_mitm_handler.rs (UPDATED)

use scred_http::protocol::h2::{alpn, ...};
use scred_http_detector::HttpContentAnalyzer;
use scred_http_redactor::H2Redactor;

async fn handle_h2_stream(
    mut stream: h2::server::SendResponse<Bytes>,
    body: impl Stream<Item = Result<Bytes>>,
) -> Result<()> {
    let detector = HttpContentAnalyzer::new(patterns);
    let redactor = H2Redactor::new(engine);
    
    // Analyze incoming request
    let analysis = detector.analyze_headers(&request.headers())?;
    
    if analysis.sensitivity >= Sensitivity::Internal {
        // Redact before processing
        redactor.redact_headers(&mut headers)?;
    }
    
    // Process request...
    // Redact response before sending back
    let response_analysis = detector.analyze_response(&response)?;
    redactor.redact_response(&mut response)?;
    
    stream.send_response(response)?;
}
```

### scred-proxy Integration

```rust
// scred-proxy/src/main.rs (UPDATED)

use scred_http_detector::HttpContentAnalyzer;
use scred_http_redactor::Http11Redactor;

async fn handle_http_request(
    mut client: TcpStream,
) -> Result<()> {
    let detector = HttpContentAnalyzer::new(patterns);
    let redactor = Http11Redactor::new(engine);
    
    // Parse request
    let mut request = parse_http_request(&mut client)?;
    
    // Analyze
    let analysis = detector.analyze_request(&request)?;
    
    // Redact
    if analysis.sensitivity >= Sensitivity::Internal {
        redactor.redact_request(&mut request)?;
    }
    
    // Forward...
}
```

---

## Benefits

### 1. Separation of Concerns ✅
- **Detection** (scred-http-detector): Analyze what needs redaction
- **Redaction** (scred-http-redactor): Apply redaction strategies
- **Protocol** (scred-http): Handle protocol specifics
- **Core** (scred-redactor): Core redaction engine

### 2. Reusability 🔄
- Other HTTP proxies can use scred-http-redactor
- Other analyzers can use scred-http-detector
- No need for custom integration

### 3. Testability 🧪
- Mock HttpContentAnalyzer for testing redactor
- Mock HttpRedactor for testing detector
- Unit test each layer independently

### 4. Protocol Flexibility 🔀
- Easy to add HTTP/3 support
- Easy to add WebSocket support
- Each protocol gets own redaction strategy

### 5. Performance 🚀
- Detector can be fast (detection only)
- Redactor can be optimized (redaction strategies)
- Streaming redaction stays in redactor

---

## Migration Plan

### Phase 1: Create New Crates (2-3 hours)
- [ ] Create scred-http-detector directory
- [ ] Create scred-http-redactor directory
- [ ] Set up Cargo.toml files
- [ ] Move h2_adapter to scred-http-redactor

### Phase 2: Implement Detection (3-4 hours)
- [ ] ContentAnalyzer trait
- [ ] HeaderAnalyzer
- [ ] BodyAnalyzer
- [ ] Classification logic
- [ ] Tests

### Phase 3: Implement HTTP-Specific Redaction (4-5 hours)
- [ ] HeaderRedactor
- [ ] BodyRedactor
- [ ] Http11Redactor
- [ ] H2Redactor (move from h2_adapter)
- [ ] Tests

### Phase 4: Update Existing Crates (2-3 hours)
- [ ] scred-http: remove h2_adapter, add new dependencies
- [ ] scred-mitm: update imports
- [ ] scred-proxy: update imports
- [ ] Test everything

### Phase 5: Verify & Document (1-2 hours)
- [ ] Build & verify no errors
- [ ] Integration tests
- [ ] Update documentation

**Total**: ~12-17 hours

---

## Code Metrics (Projected)

| Crate | Current | New | Delta | Purpose |
|-------|---------|-----|-------|---------|
| scred-http | ~5K LOC | ~3.5K LOC | -30% | Simpler (redaction moved) |
| scred-http-detector | — | ~500 LOC | NEW | Content analysis |
| scred-http-redactor | — | ~700 LOC | NEW | HTTP redaction |
| scred-redactor | ~3K LOC | ~3K LOC | — | Unchanged (core) |
| **Total** | — | +1.2K LOC | +25% | But organized! |

---

## What About h2_adapter?

### Current (in scred-http)
```rust
pub struct H2MitmAdapter {
    engine: Arc<RedactionEngine>,
    // Per-stream state
}
```

### After (in scred-http-redactor)
```rust
pub struct H2Redactor {
    engine: Arc<RedactionEngine>,
    header_redactor: HeaderRedactor,
    body_redactor: BodyRedactor,
    // Per-stream state
}
```

**Changes**:
- Move to scred-http-redactor (proper domain)
- Rename to H2Redactor (more specific)
- Compose HeaderRedactor + BodyRedactor (better structure)
- Re-export from scred-http for backward compatibility (if needed)

---

## Example: Putting It All Together

```rust
// Application (scred-proxy)

use scred_http_detector::{HttpContentAnalyzer, Sensitivity};
use scred_http_redactor::Http11Redactor;
use scred_http::protocol::http1;
use scred_redactor::RedactionEngine;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize
    let engine = Arc::new(RedactionEngine::new(patterns)?);
    let detector = HttpContentAnalyzer::new(pattern_detector);
    let redactor = Http11Redactor::new(engine.clone());
    
    // Accept connection
    let mut client = accept_connection().await?;
    
    // 1. PARSE
    let mut request = http1::parse_request(&mut client)?;
    
    // 2. DETECT
    let analysis = detector.analyze_request(&request)?;
    println!("Sensitivity: {:?}", analysis.sensitivity);
    println!("Findings: {:?}", analysis.findings);
    
    // 3. REDACT (if needed)
    if analysis.sensitivity >= Sensitivity::Internal {
        redactor.redact_request(&mut request)?;
    }
    
    // 4. FORWARD
    let response = forward_upstream(&request).await?;
    
    // 5. ANALYZE RESPONSE
    let response_analysis = detector.analyze_response(&response)?;
    
    // 6. REDACT RESPONSE
    if response_analysis.sensitivity >= Sensitivity::Internal {
        redactor.redact_response(&mut response)?;
    }
    
    // 7. SEND TO CLIENT
    http1::encode_response(&response, &mut client)?;
    
    Ok(())
}
```

---

## Final Architecture (Visual)

```
┌──────────────────────────────────────────────────────────────┐
│                    Applications                              │
│         (scred-mitm / scred-proxy / scred-cli)               │
└──────────────┬──────────────────────┬──────────────────────┘
               │                      │
        ┌──────▼────────┐      ┌──────▼────────┐
        │  scred-http   │      │scred-http-det │
        │ (Protocols)   │      │ (Detection)   │
        └──────┬────────┘      └──────┬────────┘
               │                      │
               └──────────┬───────────┘
                          ↓
              ┌──────────────────────────┐
              │ scred-http-redactor      │
              │ (HTTP Redaction Logic)   │
              └──────────┬───────────────┘
                         ↓
              ┌──────────────────────────┐
              │  scred-redactor          │
              │  (Core Redaction)        │
              └──────────────────────────┘
                         │
                         ↓
              ┌──────────────────────────┐
              │ scred-pattern-detector   │
              │ (Pattern Matching FFI)   │
              └──────────────────────────┘

Clean dependency flow: ↓ only downward
Separation of concerns: 5 layers
Protocol independence: ✅
Reusable components: ✅
Testable architecture: ✅
```

---

## Summary

### What You're Getting
1. **scred-http-detector**: Pure analysis layer (no mutations)
2. **scred-http-redactor**: Protocol-specific redaction strategies
3. **Separated concerns**: Detection ≠ Redaction ≠ Protocol handling
4. **Better organization**: Each crate has single, clear purpose
5. **No h2 in redactor**: H2 redaction stays in http-specific layer

### What Changes
1. h2_adapter moves from scred-http to scred-http-redactor (renamed H2Redactor)
2. scred-http becomes simpler (redaction logic removed)
3. scred-mitm/proxy have cleaner interfaces (via new crates)
4. New dependencies: scred-http-detector, scred-http-redactor

### What Stays the Same
1. Core redaction engine (scred-redactor)
2. Pattern detection (scred-pattern-detector)
3. Protocol parsing (http crate, h2 crate)
4. Functionality (same behavior, better organized)

