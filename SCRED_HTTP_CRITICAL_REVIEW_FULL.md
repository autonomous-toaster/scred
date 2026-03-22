# ⚠️ CRITICAL REVIEW: scred-http Structure & Organization

## Executive Summary

**Rating: 4/10** - Functional but poorly organized
- **Works**: Core functionality is there
- **Problem**: Chaotic module organization, unclear separation of concerns, mixed responsibilities
- **Risk**: Hard to maintain, difficult to extend, prone to bugs

---

## 1. ORGANIZATIONAL CHAOS - The Main Problem

### Current Structure (Bad)
```
scred-http/src/
├── alpn.rs                    (Protocol detection)
├── chunked_parser.rs          (HTTP chunked encoding)
├── config.rs                  (Configuration)
├── connect.rs                 (CONNECT tunneling)
├── dns_resolver.rs            (DNS lookups)
├── duplex.rs                  (Socket wrapper)
├── fixed_upstream.rs          (Upstream config)
├── h2/                        (HTTP/2 - mostly delegated to h2 crate)
├── h2_adapter/                (HTTP/2 redaction - SHOULD BE IN REDACTOR!)
├── header_rewriter.rs         (HTTP header rewriting)
├── host_identification.rs     (Extract hostname from request)
├── http_headers.rs            (Parse HTTP headers)
├── http_line_reader.rs        (Read HTTP request line)
├── http_proxy_handler.rs      (HTTP proxy logic - 497 LOC MONSTER)
├── location_rewriter.rs       (Rewrite Location header - 432 LOC)
├── logging.rs                 (JSON structured logging)
├── models.rs                  (HttpRequest, HttpResponse)
├── parser.rs                  (HTTP/1.1 parsing)
├── proxy_resolver.rs          (System proxy detection)
├── response_reader.rs         (Read HTTP response)
├── secrets.rs                 (Secret patterns)
├── streaming_request.rs       (Stream HTTP requests)
├── streaming_response.rs      (Stream HTTP responses)
├── tcp_relay.rs               (TCP relay)
├── upstream_h2_client.rs      (HTTP/2 client - UNUSED!)
└── lib.rs                     (Re-exports all)
```

**Problem**: No clear separation of concerns. Everything thrown together.

---

## 2. TOP ISSUES

### Issue #1: No Layering / Domain Separation (CRITICAL)

**What we have**:
- Protocol parsing mixed with proxy logic
- Redaction logic spread across 3+ crates
- HTTP/2 adapter in wrong crate (should be in scred-redactor)
- DNS/proxy/upstream concerns tangled together

**Should be**:
```
scred-http/
├── core/
│   ├── models.rs          (HttpRequest, HttpResponse - pure data)
│   ├── headers.rs         (Header parsing - pure parsing)
│   └── constants.rs       (HTTP constants)
│
├── protocol/
│   ├── http1/
│   │   ├── parser.rs      (HTTP/1.1 parsing only)
│   │   ├── encoder.rs     (HTTP/1.1 encoding)
│   │   └── reader.rs      (Read HTTP/1.1 responses)
│   └── h2/
│       ├── alpn.rs        (ALPN only, minimal)
│       └── adapter.rs     (REDACTION - should move to scred-redactor)
│
├── proxy/
│   ├── models.rs          (ProxyConfig, ProxyRequest)
│   ├── handler.rs         (Proxy logic)
│   ├── upstream.rs        (Upstream connection)
│   └── dns.rs             (DNS resolution)
│
├── streaming/
│   ├── request.rs         (Stream HTTP requests)
│   ├── response.rs        (Stream HTTP responses)
│   └── chunked.rs         (Chunked encoding)
│
└── utils/
    ├── logging.rs
    ├── duplex.rs
    └── tcp_relay.rs
```

### Issue #2: 497-Line `http_proxy_handler.rs` (TOO LARGE)

**Current file** does:
- HTTP request parsing ✓
- Header rewriting ✓
- DNS resolution ✓
- Proxy chain resolution ✓
- Upstream connection ✓
- Request forwarding ✓
- Response parsing ✓
- Response rewriting ✓
- Error handling ✓
- Logging ✓

**Every single responsibility mixed in one file!**

**Should split into**:
```
proxy/
├── handler.rs            (Orchestration only - 50-80 LOC)
├── request_handler.rs    (Parse & forward request - 100 LOC)
├── response_handler.rs   (Parse & return response - 100 LOC)
├── upstream.rs           (Upstream connection logic - 80 LOC)
├── rewriter.rs           (Header rewriting - 80 LOC)
└── errors.rs             (Proxy-specific errors)
```

### Issue #3: Unclear Module Dependencies

**Problem**: No clear import hierarchy
- `http_proxy_handler` imports from `header_rewriter` & `location_rewriter`
- But `location_rewriter` also parses headers?
- Both do similar things but different

**Questions**:
- Why is `header_rewriter` separate from `location_rewriter`?
- Why does `streaming_response` import `header_rewriter`?
- What's the difference between `parser.rs` and `http_headers.rs`?

### Issue #4: Redaction Logic in Wrong Crate

**Current**: 
- `h2_adapter/mod.rs` (250 LOC) in scred-http
- Should be: in `scred-redactor`

**Why**: 
- It's redaction-specific, not HTTP-specific
- Both MITM and Proxy use it
- Creates circular dependency risk

**Fix**:
```
scred-redactor/src/
├── core.rs         (RedactionEngine)
├── patterns.rs     (Pattern matching)
├── streaming.rs    (Streaming redactor)
├── h2_adapter.rs   (H2-specific redaction) ← MOVE HERE
└── ...
```

### Issue #5: Unused/Dead Code

**Files to question**:
- `upstream_h2_client.rs` - Is this used? (SCRED uses h2 crate now)
- `fixed_upstream.rs` - What uses this?
- `tcp_relay.rs` - Generic implementation, but is it used?
- `secrets.rs` - Seems like dead code from old redaction

**Need**: Dependency analysis to find dead code

### Issue #6: Configuration Scattered

**Configs in multiple places**:
- `config.rs` - General config
- `http_proxy_handler.rs` - `HttpProxyConfig` 
- `secrets.rs` - Secret patterns config
- Various hardcoded constants in files

**Should consolidate** into:
```
config/
├── http.rs       (HTTP protocol config)
├── proxy.rs      (Proxy behavior config)
├── redaction.rs  (Redaction config)
├── logging.rs    (Logging config)
└── root.rs       (Root AppConfig)
```

### Issue #7: No Error Handling Strategy

**Problem**:
- Each module returns `anyhow::Result`
- No custom error types
- Can't distinguish error causes
- Difficult to test error paths

**Should have**:
```
errors.rs
├── HttpParseError
├── ProxyError
├── UpstreamError
├── DnsError
└── impl From<X> for ProxyError
```

### Issue #8: Naming is Confusing

**Confusing names**:
- `http_proxy_handler` - Not just HTTP, handles CONNECT too?
- `streaming_request` / `streaming_response` - What does "streaming" mean?
- `header_rewriter` vs `location_rewriter` - Why separate?
- `response_reader` vs `streaming_response` - Difference?
- `host_identification` vs `dns_resolver` - Same concern?

**Better names**:
```
http_proxy_handler      → forward_proxy_handler
streaming_request       → chunked_request_reader
streaming_response      → chunked_response_reader
header_rewriter         → header_transformer
location_rewriter       → redirect_rewriter
host_identification     → hostname_extractor
response_reader         → response_parser
```

### Issue #9: Missing Documentation

**Problem**: 
- No clear README explaining module relationships
- No architecture diagram
- Comments explain "what" not "why"
- No examples of common usage patterns

**Should have**:
- `ARCHITECTURE.md` - Explain module organization
- `MODULES.md` - Document each module's responsibility
- `EXAMPLES.md` - Show how to use the crate
- Architecture diagrams

### Issue #10: Testing Strategy Unclear

**Problem**:
- No clear unit test organization
- Integration tests mixed with unit tests
- Hard to test individual components
- Mock objects scattered throughout

**Should have**:
```
tests/
├── integration/
│   ├── proxy_e2e.rs
│   └── protocol_e2e.rs
└── unit/
    ├── parser_tests.rs
    ├── models_tests.rs
    └── handler_tests.rs
```

---

## 3. DEBT ITEMS (TECHNICAL DEBT)

### High Priority

1. **`http_proxy_handler.rs` - SPLIT THIS** (497 LOC)
   - Extract upstream handling
   - Extract response handling
   - Extract rewriting logic

2. **Move h2_adapter to scred-redactor**
   - Remove from scred-http
   - Clean up crate boundaries

3. **Define error types**
   - Replace all `anyhow::Result` with custom types
   - Make error handling testable

4. **Remove dead code**
   - `upstream_h2_client.rs` - Not used
   - `tcp_relay.rs` - Generic but unused?
   - `secrets.rs` - Old redaction code?

### Medium Priority

5. **Reorganize into layers**
   - Protocol layer (parsing)
   - Proxy layer (forwarding)
   - Streaming layer (chunked/h2c)
   - Utils layer (logging, DNS, etc.)

6. **Consolidate config**
   - One config structure
   - Validation at entry point
   - No scattered hardcoded values

7. **Fix naming**
   - Make intent clear
   - Distinguish concerns
   - Consistent naming patterns

8. **Add documentation**
   - Architecture guide
   - Module responsibility docs
   - Usage examples

### Low Priority

9. **Add integration tests**
   - Full proxy scenarios
   - Error handling paths
   - Protocol negotiation

10. **Performance audit**
    - Buffer sizes
    - Memory allocations
    - Async/await efficiency

---

## 4. SPECIFIC FILE ISSUES

### `http_proxy_handler.rs` (497 LOC) - THE MONSTER

**Does**: Everything for proxy forwarding
**Responsibility Count**: 8+
**Testability**: Poor
**Reusability**: Low

**Split into**:
```
proxy/
├── handler.rs           (Main orchestration, 60 LOC)
├── request_pipeline.rs  (Parse + forward request, 120 LOC)
├── response_pipeline.rs (Parse + forward response, 120 LOC)
├── upstream.rs          (Connection to upstream, 80 LOC)
├── rewriting.rs         (Header/redirect rewriting, 100 LOC)
└── errors.rs            (Proxy-specific errors)
```

### `location_rewriter.rs` (432 LOC) - TOO SPECIFIC

**Problem**: Just rewrites Location header but 432 LOC
- Could be part of `header_rewriter`
- Or generalized as `response_transformer`

### `streaming_response.rs` (344 LOC) - Mixed Concerns

**Does**:
- Parse HTTP response headers
- Handle chunked encoding
- Handle Content-Length
- Buffer responses
- Parse trailers

**Should split**:
```
streaming/
├── response.rs         (Response streaming - 150 LOC)
├── chunked.rs          (Chunked encoding - 150 LOC, already exists!)
└── trailers.rs         (Trailer handling - 50 LOC)
```

### `parser.rs` (323 LOC) vs `http_headers.rs` (206 LOC)

**Confusion**: Why two header parsing modules?
- `parser.rs` - HTTP/1.1 request parsing
- `http_headers.rs` - Generic header parsing

**Should consolidate**:
```
protocol/http1/
├── parser.rs       (Request/response parsing)
├── encoder.rs      (Request/response encoding)
└── errors.rs       (Parse errors)
```

### `proxy_resolver.rs` (320 LOC) - Missing Abstraction

**Does**: Connects through HTTP proxies
**Issue**: Only one implementation type
**Should have**: Trait-based proxy resolution

```rust
pub trait ProxyResolver {
    fn resolve(&self, target: &str) -> Option<ProxyTarget>;
}

impl ProxyResolver for SystemProxyResolver { ... }
impl ProxyResolver for ConfigProxyResolver { ... }
```

---

## 5. ARCHITECTURAL PROBLEMS

### Problem A: No Clear Protocol Abstraction

**Current**:
- HTTP/1.1: `parser.rs`, `http_headers.rs`, `streaming_request.rs`, etc.
- HTTP/2: `h2/alpn.rs` + uses h2 crate
- No unified interface

**Should have**:
```rust
pub trait HttpProtocol {
    fn parse_request(&mut self) -> Result<HttpRequest>;
    fn send_response(&mut self, response: HttpResponse) -> Result<()>;
}

impl HttpProtocol for Http11Protocol { ... }
impl HttpProtocol for Http2Protocol { ... }
```

### Problem B: No Proxy Abstraction

**Current**:
- MITM proxy in scred-mitm
- Forward proxy in scred-proxy
- Common code in scred-http but not abstracted

**Should have**:
```rust
pub trait ProxyHandler {
    async fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse>;
}

pub trait RequestForwarder {
    async fn forward(&self, req: HttpRequest) -> Result<HttpResponse>;
}
```

### Problem C: Redaction Tightly Coupled

**Current**:
- Redaction engine passed everywhere as `Arc<RedactionEngine>`
- Hard to mock for testing
- Hard to compose different strategies

**Should have**:
```rust
pub trait Redactor {
    fn redact(&self, data: &str) -> String;
}

impl Redactor for RedactionEngine { ... }
impl Redactor for NoOpRedactor { ... }  // For testing
impl Redactor for CompositeRedactor { ... }
```

---

## 6. QUICK WINS (Easy Fixes)

1. **Delete `upstream_h2_client.rs`** if unused (10 min)
2. **Rename files for clarity** (20 min)
3. **Add README explaining modules** (30 min)
4. **Consolidate `header_rewriter` & `location_rewriter`** (1 hour)
5. **Extract config validation to one place** (1 hour)

---

## 7. MAJOR REFACTORING (Hard Fixes)

1. **Split `http_proxy_handler.rs` into proxy/ directory** (4-6 hours)
2. **Move h2_adapter to scred-redactor** (2 hours)
3. **Add trait-based abstraction** (6-8 hours)
4. **Reorganize into layers** (8-10 hours)
5. **Add comprehensive tests** (10-12 hours)

---

## 8. RECOMMENDED PRIORITY

### Phase 1 (Immediate): Organization
1. Delete dead code (`upstream_h2_client.rs`, verify others)
2. Rename files for clarity
3. Move h2_adapter to scred-redactor
4. Add README/architecture docs

### Phase 2 (Short-term): Refactoring
1. Split `http_proxy_handler.rs`
2. Consolidate header rewriting logic
3. Add custom error types
4. Reorganize into clear layers

### Phase 3 (Medium-term): Abstraction
1. Add trait-based protocol handling
2. Add trait-based proxy handling
3. Add trait-based redaction
4. Improve testability

---

## 9. SEVERITY SUMMARY

| Issue | Severity | Impact | Effort |
|-------|----------|--------|--------|
| No clear layering | **CRITICAL** | Hard to maintain | 8-10h |
| 497-LOC monster file | **HIGH** | Unmaintainable | 4-6h |
| Redaction in wrong crate | **HIGH** | Wrong boundaries | 2h |
| Scattered config | **MEDIUM** | Error-prone | 2h |
| Unused code | **MEDIUM** | Confusing | 1h |
| Unclear naming | **MEDIUM** | Confusing | 1h |
| No error types | **MEDIUM** | Hard to test | 2-3h |
| Missing docs | **LOW** | Hard to understand | 2h |

---

## 10. WHAT'S ACTUALLY WORKING

Despite these issues:
- ✅ HTTP/1.1 proxy works
- ✅ HTTP/2 support works
- ✅ h2c support works
- ✅ Streaming redaction works
- ✅ Protocol fallback works
- ✅ Corporate proxy support works

**The code works but it's hard to understand and maintain.**

---

## CONCLUSION

**Rating: 4/10 - Functional Chaos**

### What's wrong
- No clear separation of concerns
- Mixed responsibilities everywhere
- Poor module organization
- Scattered configuration
- Unused/dead code

### What needs fixing
1. Split large modules
2. Reorganize into layers
3. Move redaction to proper crate
4. Add trait-based abstraction
5. Add comprehensive documentation

### Time to refactor
- **Quick wins**: 2-3 hours
- **Major reorganization**: 20-30 hours
- **Full refactoring + abstraction**: 40-50 hours

### Risk of not fixing
- 😞 Hard to add features
- 😞 Difficult to test
- 😞 Confusing for new developers
- 😞 Bug-prone maintenance
- 😞 Code duplication risk
