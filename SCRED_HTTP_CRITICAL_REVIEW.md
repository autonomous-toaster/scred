# ⚠️ CRITICAL REVIEW: scred-http Structure & Organization

**Rating: 4/10** - Functional but poorly organized

## Executive Summary

The scred-http crate works, but it's a structural mess. Module organization is chaotic, responsibilities are mixed everywhere, and the codebase is hard to maintain and extend.

### Key Problems
1. **No clear layering** - Protocol, proxy, streaming, and utils all tangled
2. **Monster file** - `http_proxy_handler.rs` with 497 LOC doing 8+ things
3. **Wrong crate boundaries** - h2_adapter (redaction) should be in scred-redactor
4. **Scattered configuration** - Config split across 4+ files/modules
5. **Unused code** - Dead code not cleaned up
6. **No error types** - All `anyhow::Result`, no custom errors
7. **Confusing names** - Module names don't match responsibilities
8. **Missing abstraction** - No protocol/proxy trait definitions

### Impact
- 😞 Hard to add features
- 😞 Difficult to test
- 😞 Confusing for new developers
- 😞 Bug-prone maintenance
- 😞 Code duplication risk

---

## What's Working Despite This

✅ HTTP/1.1 proxy works  
✅ HTTP/2 support works  
✅ h2c support works  
✅ Streaming redaction works  
✅ Protocol fallback works  
✅ Corporate proxy support works  

**The code works but it's hard to understand and maintain.**

---

## Recommended Reorganization

### Proposed Structure
```
scred-http/
├── core/                 (Pure data & constants)
│   ├── models.rs        (HttpRequest, HttpResponse)
│   ├── headers.rs       (Header parsing primitives)
│   └── constants.rs     (HTTP constants)
│
├── protocol/            (Protocol implementations)
│   ├── http1/
│   │   ├── parser.rs    (HTTP/1.1 parsing)
│   │   ├── encoder.rs   (HTTP/1.1 encoding)
│   │   └── reader.rs    (HTTP/1.1 response reading)
│   └── h2/
│       └── alpn.rs      (ALPN detection only - minimal)
│
├── proxy/               (Proxy forwarding logic)
│   ├── handler.rs       (Main orchestration)
│   ├── request.rs       (Request handling)
│   ├── response.rs      (Response handling)
│   ├── upstream.rs      (Upstream connection)
│   ├── rewriting.rs     (Header/redirect rewriting)
│   └── errors.rs        (Proxy-specific errors)
│
├── streaming/           (Chunked & streaming)
│   ├── request.rs       (Stream HTTP requests)
│   ├── response.rs      (Stream HTTP responses)
│   └── chunked.rs       (Chunked encoding)
│
└── utils/               (Utilities)
    ├── logging.rs       (Structured logging)
    ├── duplex.rs        (Socket wrapper)
    ├── tcp_relay.rs     (TCP relay)
    ├── dns.rs           (DNS resolution)
    └── config.rs        (Configuration)
```

---

## Top 5 Issues to Fix

### 1. CRITICAL: Split `http_proxy_handler.rs` (497 LOC)
**Current**: One file doing 8+ things
**Fix**: Extract into proxy/ subdirectory with separate concerns
**Effort**: 4-6 hours
**Impact**: Immediate 50% improvement in maintainability

### 2. HIGH: Move h2_adapter to scred-redactor
**Current**: Redaction adapter in wrong crate (should be with redaction)
**Fix**: Move to scred-redactor/src/h2_adapter.rs
**Effort**: 2 hours
**Impact**: Clean crate boundaries

### 3. HIGH: Add custom error types
**Current**: All `anyhow::Result`
**Fix**: Define HttpError, ProxyError, ParseError enums
**Effort**: 2-3 hours
**Impact**: Better error handling & testability

### 4. MEDIUM: Remove dead code
**Current**: `upstream_h2_client.rs`, possibly others
**Fix**: Dependency analysis + cleanup
**Effort**: 1 hour
**Impact**: Cleaner codebase

### 5. MEDIUM: Reorganize into layers
**Current**: Flat structure
**Fix**: Create core/, protocol/, proxy/, streaming/, utils/ directories
**Effort**: 8-10 hours
**Impact**: Clear separation of concerns

---

## Quick Wins (< 2 hours)

- [ ] Delete unused `upstream_h2_client.rs`
- [ ] Rename modules for clarity
- [ ] Add `ARCHITECTURE.md` explaining structure
- [ ] Consolidate scattered configuration
- [ ] Add module-level documentation

---

## See Full Review

For complete analysis including:
- Detailed issue breakdown
- File-by-file analysis
- Architectural problems
- Missing abstractions
- Testing strategy improvements
- Complete refactoring roadmap

👉 See full document for implementation details.
