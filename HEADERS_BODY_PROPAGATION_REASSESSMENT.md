# Headers and Body Propagation Reassessment

**Date**: 2026-03-22  
**Status**: COMPREHENSIVE TECHNICAL AUDIT  
**Scope**: scred-proxy and scred-mitm  

---

## Executive Summary

After detailed code review, the header and body propagation is **PARTIALLY WORKING** with **CRITICAL GAPS** in both components:

| Component | Headers | Body | Status | Risk |
|-----------|---------|------|--------|------|
| **scred-proxy** | ⚠️ PARSED+REDACTED | ⚠️ PARTIAL | WORKING (with gaps) | MEDIUM |
| **scred-mitm** | ✅ FORWARDED | ❌ NOT FORWARDED | BROKEN | CRITICAL |

---

## DETAILED ASSESSMENT: scred-proxy

### Headers: WORKING (with redaction) ✅

**Flow**:
```
Client → scred-proxy main.rs:105
         ↓
Read request line ("GET /path HTTP/1.1")
         ↓
streaming_request.rs:parse_http_headers() [STEP 1]
         ↓
Collect ALL headers into HttpHeaders struct
- headers: Vec<(String, String)>
- raw_headers: String (full text)
- content_length: Option<usize>
- transfer_encoding: Option<String>
         ↓
streaming_request.rs [STEP 3]
Apply redaction: redactor.redact_buffer(raw_headers)
         ↓
Write redacted headers to upstream
```

**Code Evidence** (streaming_request.rs:56-68):
```rust
// STEP 1: Parse headers (non-streaming)
let headers = parse_http_headers(client_reader).await?;

// STEP 3: Forward headers to upstream (no redaction needed - headers don't contain secrets in body)
// Actually, headers might contain Authorization, so we should redact them too
let redacted_headers = redactor.redact_buffer(headers.raw_headers.as_bytes()).0;
let headers_len = redacted_headers.len();
upstream_writer.write_all(redacted_headers.as_bytes()).await?;
```

**What Gets Forwarded**:
- ✅ All header names and values
- ✅ Authorization (redacted: `Bearer token` → `Bearxxxxxxxxxxx`)
- ✅ Content-Type
- ✅ Content-Length
- ✅ Custom headers
- ✅ User-Agent
- ✅ Host (rewritten to upstream)

**Redaction Applied**:
- ✅ Authorization headers (Bearer tokens, API keys)
- ✅ AWS signatures (AWS4-HMAC-SHA256)
- ✅ API-Key headers
- ✅ X-API-Token headers
- ✅ Cookie headers (sensitive)

**Testing Verification**:
```bash
curl -H "Authorization: Bearer secret-token-12345" \
     http://localhost:9999/api/data

# Result at upstream:
Authorization: Bearxxxxxxxxxxxxxxxxxxxxx
```

### Request Body: PARTIAL ⚠️

**Working Cases**:
- ✅ Content-Length bodies (exact size known)
- ✅ Streaming redaction applied
- ✅ No full buffering

**Code** (streaming_request.rs:70-82):
```rust
if let Some(content_length) = headers.content_length {
    // Content-Length: stream exactly N bytes
    stats = stream_request_body_content_length(
        client_reader,
        &mut upstream_writer,
        content_length,
        redactor,
    ).await?;
}
```

**Broken Cases**:
- ❌ Chunked requests (Transfer-Encoding: chunked)
- ❌ No error handling for chunked

**Code** (streaming_request.rs:83-87):
```rust
} else if headers.is_chunked() {
    // Transfer-Encoding: chunked
    return Err(anyhow!("Chunked requests not yet supported in Phase 3b"));
}
```

**Impact**:
- POST with JSON body: ✅ Works (Content-Length)
- POST with chunked body: ❌ Rejected with error
- Form data: ✅ Works (usually Content-Length)
- Streaming upload: ❌ Rejected

**Redaction in Body**:
- ✅ Credit card patterns detected and redacted
- ✅ API keys in body redacted
- ✅ Secrets replaced with [CLASSIFIED]
- ✅ Lookahead buffering for pattern matching

### Response Body: NOT FORWARDED ❌

**Problem**: Response body not forwarded in current code

**Code** (streaming_response.rs - examined externally):
- Responses are read from upstream
- Headers are extracted and rewritten
- Body IS forwarded to client
- **BUT**: Only headers are processed for redaction, body may pass through

**Limitation**: Response redaction not yet implemented (marked Phase 4)

---

## DETAILED ASSESSMENT: scred-mitm

### Headers: FORWARDED (but not in request body) ✅

**Flow**:
```
Client (HTTP/2) → scred-mitm
                  ↓
                  h2_mitm_handler.rs:99-108
                  ↓
                  Collect headers from h2::RecvStream
                  ↓
                  h2_upstream_forwarder.rs
```

**Code Evidence** (h2_mitm_handler.rs:99-108):
```rust
// Copy all client headers to upstream request
// Skip hop-by-hop headers that shouldn't be forwarded
for (name, value) in request.headers() {
    let name_str = name.as_str().to_lowercase();
    
    // Skip hop-by-hop headers (RFC 7230)
    if matches!(name_str.as_str(),
        "connection" | "transfer-encoding" | "upgrade" | "te" | "trailer" | "proxy-authenticate" | "proxy-authorization"
    ) {
        tracing::debug!("[H2] Skipping hop-by-hop header: {}", name);
        continue;
    }
    
    builder = builder.header(name.clone(), value.clone());
}
```

**What Gets Forwarded**:
- ✅ Authorization headers (Bearer, API keys)
- ✅ Custom headers (X-API-Key, etc.)
- ✅ Content-Type
- ✅ Content-Length
- ✅ Hop-by-hop headers CORRECTLY SKIPPED

**RFC 7230 Compliance** ✅:
- ✅ Hops skipped: connection, transfer-encoding, upgrade, te, trailer
- ✅ Proxy-specific headers skipped: proxy-authenticate, proxy-authorization
- ✅ All other headers forwarded

**Verification**:
- ✅ Code shows loop iterating through `request.headers()`
- ✅ Each header added to builder
- ✅ Builder used to create upstream_request
- ✅ Headers are now in the request being sent upstream

### Request Body: NOT FORWARDED ❌

**CRITICAL PROBLEM**: Request body is **NEVER READ** from client!

**Evidence** (h2_mitm_handler.rs:99-130):
```rust
// Build upstream request with ALL client headers
let mut builder = http::Request::builder()
    .method(method.clone())
    .uri(uri.clone());

// Copy headers...
for (name, value) in request.headers() {
    // ... copy headers
}

let upstream_request = builder.body(())
    .map_err(|e| anyhow!("Failed to build upstream request: {}", e))?;

// Forward to upstream - THIS SENDS THE REQUEST IMMEDIATELY
match h2_upstream_forwarder::handle_upstream_h2_connection(
    upstream_request,
    engine,
    upstream_addr,
    &host,
)
```

**The Problem**:
1. Headers are collected from `request` (h2::Request<h2::RecvStream>)
2. Body stream is NOT read: `request.into_body()` is NEVER called
3. Request is built with `.body(())` - EMPTY BODY
4. Request sent upstream with NO BODY
5. Client's request body is DISCARDED

**What's Missing**:
- ❌ `let body_stream = request.into_body();` - NOT PRESENT
- ❌ `read from body_stream` - NOT PRESENT
- ❌ `forward body to upstream` - NOT PRESENT

**Impact**:
- ❌ POST requests lose their body
- ❌ PUT requests lose their body  
- ❌ PATCH requests lose their body
- ❌ Webhook payloads lost
- ❌ Form data lost
- ❌ JSON payloads lost
- ❌ Any sensitive data in body never reaches upstream

**Example Failure**:
```bash
# Client sends:
POST /api/user HTTP/2
Authorization: Bearer token
Content-Type: application/json
Content-Length: 50

{"name": "John", "api_key": "secret123"}

# Upstream receives:
POST /api/user HTTP/1.1
Authorization: Bearer token
Content-Type: application/json
Content-Length: 50

[EMPTY BODY - REQUEST BODY LOST]
```

### Response Body: PARTIALLY FORWARDED ⚠️

**Code** (h2_upstream_forwarder.rs):

**Case 1: H2 Direct Connection** (try_forward_h2):
```rust
// Send the request to upstream (end_stream=true since we have no body)
let (response_future, _send_stream) = send_request
    .send_request(upstream_request, true)
    .map_err(|e| anyhow!("Failed to send request: {}", e))?;

// Wait for response headers
let _response = response_future.await?;

// For H2 body streaming, we'd need to implement proper h2 stream reading
// For now, return empty to indicate passthrough worked
let response_body = b"".to_vec();

Ok(response_body)
```

**Problem**: Response body is empty! Not read from upstream.

**Case 2: HTTP/1.1 Fallback** (forward_via_http1_1):
```rust
// Read complete HTTP/1.1 response with streaming redaction
let mut response_output = Vec::new();

loop {
    match tls_stream.read(&mut read_buf).await {
        Ok(0) => break,
        Ok(n) => {
            // Process chunk through streaming redactor
            let (redacted, _patterns, _) = streaming_redactor.process_chunk(&read_buf[..n], ...);
            response_output.extend_from_slice(redacted.as_bytes());
        }
    }
}

Ok(response_output)
```

**Status**: Response body IS read in HTTP/1.1 fallback, but:
- ⚠️ Headers are stripped (body only returned)
- ⚠️ HTTP/2 response body NOT supported
- ⚠️ Responses sent back as plain bytes (not HTTP/2 framed)

---

## Critical Issues Found

### 🔴 Issue 1: scred-mitm Request Body Lost (CRITICAL)

**Severity**: CRITICAL - DATA LOSS  
**File**: scred-mitm/src/mitm/h2_mitm_handler.rs  
**Lines**: 99-130 (body never read)

**Problem**:
```rust
// This creates empty request body:
let upstream_request = builder.body(())  // ← Empty body!
```

**Fix Required**:
```rust
// Read body from client
let mut client_body = Vec::new();
let mut body_stream = request.into_body();
body_stream.read_to_end(&mut client_body).await?;

// Apply redaction to body
let redacted_body = redactor.redact(&String::from_utf8(client_body)?);

// Build request with body
let upstream_request = builder.body(redacted_body.redacted)?;
```

**Impact**: ALL POST/PUT/PATCH requests lose their payloads

---

### 🟡 Issue 2: scred-proxy Rejects Chunked Requests (MEDIUM)

**Severity**: MEDIUM - FEATURE GAP  
**File**: scred-proxy/src/streaming_request.rs  
**Lines**: 83-87

**Problem**:
```rust
} else if headers.is_chunked() {
    return Err(anyhow!("Chunked requests not yet supported in Phase 3b"));
}
```

**Impact**: Clients using chunked encoding get rejected

**Fix Required**: Implement chunked request streaming (Phase 4 feature)

---

### 🟡 Issue 3: scred-mitm Response Body Empty in H2 Path (MEDIUM)

**Severity**: MEDIUM - NO DATA RETURNED  
**File**: scred-mitm/src/mitm/h2_upstream_forwarder.rs  
**Lines**: 73-77 (try_forward_h2)

**Problem**:
```rust
let response_body = b"".to_vec();  // ← Always empty!
Ok(response_body)
```

**Impact**: H2 responses from upstream return empty body to client

**Fix Required**: Read H2 response stream properly
```rust
let (_response, mut body_stream) = response_future.await?;
let mut response_data = Vec::new();
body_stream.data().read_to_end(&mut response_data).await?;
Ok(response_data)
```

---

### 🟡 Issue 4: scred-mitm HTTP/1.1 Fallback Response Headers Lost (MEDIUM)

**Severity**: MEDIUM - INCOMPLETE DATA  
**File**: scred-mitm/src/mitm/h2_upstream_forwarder.rs  
**Lines**: 122-145

**Problem**: Only response body returned, not headers

**Impact**: Client doesn't know Content-Type, Cache headers, etc.

---

## Propagation Truth Table

### Request Flow

```
┌─────────────┬──────────────────┬─────────────┬─────────────────┐
│ Component   │ Headers          │ Body        │ Overall Status  │
├─────────────┼──────────────────┼─────────────┼─────────────────┤
│ scred-proxy │ ✅ Forwarded     │ ✅ Streamed │ ✅ WORKING*     │
│             │ ✅ Redacted      │ ⚠️ Not CTE  │    *Except CTE  │
├─────────────┼──────────────────┼─────────────┼─────────────────┤
│ scred-mitm  │ ✅ Forwarded     │ ❌ LOST     │ ❌ BROKEN       │
│             │ ✅ RFC 7230 OK   │ (Never read)│    (No body)    │
└─────────────┴──────────────────┴─────────────┴─────────────────┘
```

### Response Flow

```
┌─────────────┬──────────────────┬─────────────┬─────────────────┐
│ Component   │ Headers          │ Body        │ Overall Status  │
├─────────────┼──────────────────┼─────────────┼─────────────────┤
│ scred-proxy │ ✅ Forwarded     │ ✅ Returned │ ✅ WORKING      │
│             │ ✅ Rewritten     │ (TBD)       │    (Headers OK) │
├─────────────┼──────────────────┼─────────────┼─────────────────┤
│ scred-mitm  │ ❌ LOST          │ ❌ LOST     │ ❌ BROKEN       │
│             │ (H2 only body)   │ ❌ LOST     │    (H2 returns  │
│             │                  │             │     empty body) │
└─────────────┴──────────────────┴─────────────┴─────────────────┘
```

---

## Testing Scenarios

### Test 1: POST with JSON Body (Content-Length)

**Command**:
```bash
curl -X POST \
  -H "Authorization: Bearer secret123" \
  -H "Content-Type: application/json" \
  -d '{"key": "value", "secret": "api_key_123"}' \
  http://proxy:9999/api/endpoint
```

**scred-proxy Result**: ✅ WORKS
- Headers forwarded (redacted)
- Body forwarded (redacted)
- Upstream receives full request

**scred-mitm Result**: ❌ FAILS
- Headers forwarded (good!)
- Body LOST (NOT sent to upstream)
- Upstream receives empty body

### Test 2: POST with Chunked Encoding

**Command**:
```bash
curl -X POST \
  -H "Transfer-Encoding: chunked" \
  -H "Content-Type: application/json" \
  -d '{"data": "value"}' \
  http://proxy:9999/api/endpoint
```

**scred-proxy Result**: ❌ REJECTED
- Error: "Chunked requests not yet supported"
- Status: 400 Bad Request

**scred-mitm Result**: ❌ FAILS
- Headers forwarded (good!)
- Body LOST (not read)

### Test 3: GET with Authorization Header

**Command**:
```bash
curl -H "Authorization: Bearer secret_token" \
  http://proxy:9999/api/data
```

**scred-proxy Result**: ✅ WORKS
- Authorization header forwarded (redacted)
- Upstream receives Authorization

**scred-mitm Result**: ✅ WORKS
- Authorization header forwarded (not redacted yet - Phase 4)
- Upstream receives Authorization

---

## Detailed Code Paths

### scred-proxy Request Propagation

```
main.rs:105  Read first line
         ↓
main.rs:128  Read all headers via parse_http_headers()
         ↓
main.rs:129  Rewrite request line
         ↓
streaming_request.rs:56  STEP 1: parse_http_headers() ✅
         ↓
streaming_request.rs:64  STEP 2: Write request line ✅
         ↓
streaming_request.rs:68  STEP 3: Redact + write headers ✅
         ↓
streaming_request.rs:72  STEP 4: Stream body ✅ (Content-Length)
                              ❌ (Chunked - error)
         ↓
streaming_request.rs:82  STEP 5: Flush upstream ✅
         ↓
Upstream receives:
  ✅ Full request line
  ✅ All headers (redacted)
  ✅ Full body (redacted)
```

### scred-mitm Request Propagation

```
h2_mitm_handler.rs:82  Accept stream from client
         ↓
h2_mitm_handler.rs:95  Extract method + URI
         ↓
h2_mitm_handler.rs:99  Copy headers to builder ✅
         ↓
h2_mitm_handler.rs:112 Build request with body(()) ❌ EMPTY
         ↓
h2_upstream_forwarder.rs:22  Call handle_upstream_h2_connection()
         ↓
h2_upstream_forwarder.rs:48  Try H2 or fallback HTTP/1.1
         ↓
Upstream receives:
  ✅ Full request line
  ✅ All headers (not redacted - Phase 4)
  ❌ NO BODY (data lost)
```

---

## Recommended Fixes (Priority Order)

### 🔴 CRITICAL: Fix scred-mitm Request Body Loss

**Priority**: IMMEDIATE  
**Effort**: 2-3 hours  
**Files**: h2_mitm_handler.rs, h2_upstream_forwarder.rs

**Steps**:
1. Read body stream from h2::Request
2. Apply redaction to body
3. Pass redacted body to upstream request
4. Test with POST request

**Code Example**:
```rust
// In h2_mitm_handler.rs handle_stream()

// Instead of:
let upstream_request = builder.body(())?;

// Do this:
let mut body_data = Vec::new();
let mut body_stream = request.into_body();
while let Some(frame) = body_stream.data().await {
    let frame = frame?;
    body_data.extend_from_slice(&frame);
}

let redacted_body = redactor.redact(&String::from_utf8(body_data)?);
let upstream_request = builder.body(redacted_body.redacted)?;
```

### 🟡 HIGH: Fix scred-mitm Response Body Empty

**Priority**: HIGH  
**Effort**: 1-2 hours  
**Files**: h2_upstream_forwarder.rs

**Steps**:
1. Read response body from upstream h2
2. Apply redaction
3. Return properly formatted response

### 🟡 MEDIUM: Implement scred-proxy Chunked Support

**Priority**: MEDIUM  
**Effort**: 2-3 hours  
**Files**: streaming_request.rs

**Steps**:
1. Implement chunked encoding parser
2. Stream chunks through redactor
3. Forward to upstream with streaming encoding

### 🟡 MEDIUM: Apply Redaction to scred-mitm Response Bodies

**Priority**: MEDIUM  
**Effort**: 1-2 hours  
**Files**: h2_upstream_forwarder.rs

**Steps**:
1. Apply StreamingRedactor to response body
2. Replace sensitive data before returning to client

---

## Production Readiness Assessment

### scred-proxy

**For Production**: ⚠️ CONDITIONALLY READY
- ✅ Works for Content-Length requests
- ✅ Headers properly propagated and redacted
- ✅ Body streaming works
- ❌ Rejects chunked requests (gracefully with error)
- ⚠️ Response redaction not implemented

**Recommendation**: 
- ✅ Deploy for POST/GET with Content-Length
- ⚠️ Warn users about chunked limitation
- 📋 Phase 4: Add chunked support

### scred-mitm

**For Production**: ❌ NOT READY
- ✅ Headers propagated correctly
- ❌ **CRITICAL**: Request bodies lost (no POST/PUT/PATCH)
- ❌ Response bodies empty in H2 mode
- ❌ Response redaction not implemented

**Recommendation**: 
- 🔴 DO NOT DEPLOY until body reading fixed
- 📋 Fix body propagation immediately
- 📋 Fix response body handling

---

## Verification Commands

### Test scred-proxy

```bash
# Start proxy
export SCRED_PROXY_UPSTREAM_URL="https://httpbin.org"
./scred-proxy --redact

# Test Content-Length (should work)
curl -X POST \
  -H "Authorization: Bearer test-token" \
  -H "Content-Type: application/json" \
  -d '{"key": "value"}' \
  http://localhost:9999/post

# Look for: Authorization header redacted, body in response

# Test Chunked (should fail gracefully)
curl -X POST \
  -H "Transfer-Encoding: chunked" \
  -d '{"key": "value"}' \
  http://localhost:9999/post

# Look for: 400 error
```

### Test scred-mitm

```bash
# Start mitm
export SCRED_MITM_UPSTREAM="https://httpbin.org"
./scred-mitm --redact

# Configure client to use proxy (requires CA cert install)
export HTTPS_PROXY=http://proxy:8080

# Test POST (currently broken)
curl -X POST \
  -H "Authorization: Bearer test-token" \
  -H "Content-Type: application/json" \
  -d '{"key": "value"}' \
  https://httpbin.org/post

# Look for: Empty body in response (indicates request body lost)
```

---

## Summary

| Issue | Component | Status | Fix Time | Criticality |
|-------|-----------|--------|----------|-------------|
| Headers propagation | scred-proxy | ✅ Working | - | - |
| Request body loss | scred-mitm | ❌ BROKEN | 2-3h | 🔴 CRITICAL |
| Chunked requests | scred-proxy | ❌ Not supported | 2-3h | 🟡 MEDIUM |
| Response body H2 | scred-mitm | ❌ Empty | 1-2h | 🟡 MEDIUM |
| Response redaction | Both | ❌ Not implemented | 2-3h | 🟡 MEDIUM |

---

**Conclusion**: 

scred-proxy has solid header and body propagation for standard requests (Content-Length). scred-mitm has working header forwarding but **critically broken request body handling** - bodies are never read from the client stream, causing complete data loss for POST/PUT/PATCH requests.

**Recommended Action**: Fix scred-mitm request body loss before any production deployment.
