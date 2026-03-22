# SCRED-PROXY BUG ANALYSIS & ROOT CAUSE IDENTIFICATION

**Date**: 2026-03-22  
**Status**: Bug Identified & Partially Fixed  
**Severity**: CRITICAL - Proxy Non-Functional

---

## Executive Summary

scred-proxy hangs indefinitely when attempting to proxy HTTP/1.1 requests to upstream servers. The root cause involves a fundamental architectural mismatch in how streaming request/response handlers work with buffered I/O streams.

**What Works**: ✅ Compiles, ✅ Listens on port 9999, ✅ Accepts client connections, ✅ Reads request line, ✅ Establishes upstream TLS connections

**What Fails**: ❌ Never returns response to client, ❌ Hangs forever after successful upstream connection

---

## Bug Manifest

### Error Messages (From User Report)

```
2026-03-22T21:20:40.505947Z ERROR scred_proxy: Error handling connection from 127.0.0.1:50625: 
  stream did not contain valid UTF-8

2026-03-22T21:20:44.160378Z  INFO scred_http::dns_resolver: DNS: Successfully connected to httpbin.org:443
2026-03-22T21:20:45.123906Z ERROR scred_proxy: Error handling connection from 127.0.0.1:50628: 
  EOF before end of headers
```

### Behavior

```bash
curl http://localhost:9999/anything -v

# Hangs forever, eventually times out
# curl: (52) Empty reply from server
```

---

## Root Cause Analysis

### Issue 1: ❌ FOUND - Extra `\r\n` in Header Forwarding

**Location**: `crates/scred-http/src/streaming_request.rs:74-77`

**Problem**:
```rust
let redacted_headers = redactor.redact_buffer(headers.raw_headers.as_bytes()).0;
upstream_writer.write_all(redacted_headers.as_bytes()).await?;
upstream_writer.write_all(b"\r\n").await?;  // ← EXTRA \r\n!
```

**Why It's Wrong**:
- `headers.raw_headers` includes the FULL header block with trailing blank line
- The blank line is `\r\n` at the end
- We're adding ANOTHER `\r\n`, resulting in `\r\n\r\n\r\n`
- Upstream server sees malformed HTTP request

**Status**: ✅ **FIXED** - Removed the extra `\r\n` write

---

### Issue 2: ❌ FOUND - BufReader Wrapping Mismatch

**Location**: `crates/scred-proxy/src/main.rs:115-142`

**Problem**:
```rust
let mut upstream = BufReader::new(tls_stream);  // ← Wrapped in BufReader

stream_request_to_upstream(
    &mut client_reader,
    &mut upstream,                             // ← Passing BufReader as writer!
    ...
).await?;
```

**Why It's Wrong**:
- `BufReader` only implements `AsyncRead`, NOT `AsyncWrite`
- `stream_request_to_upstream` expects parameter `W: AsyncWriteExt`
- Cannot write through a BufReader - it's read-only!
- This could cause panics or silent failures

**Result**: Writes might not actually go to the underlying socket

**Status**: ✅ **FIXED** - Don't wrap upstream in BufReader before writing
  - Write to raw TlsStream
  - Only wrap in BufReader after writing, before reading response

---

### Issue 3: ⚠️ UNCERTAIN - Parse Headers Hang

**Location**: `crates/scred-http/src/streaming_request.rs:65`

**Problem**:
```rust
let headers = parse_http_headers(client_reader).await?;
```

**Hypothesis**:
- `parse_http_headers` reads from `client_reader` (BufReader wrapping client socket)
- Uses `reader.read_line()` in a loop until blank line
- If client doesn't send blank line, loop never exits
- If buffer issues exist, might hang

**Status**: ⚠️ **UNCERTAIN** - Likely not the issue since curl sends proper HTTP headers with blank line

---

### Issue 4: ⚠️ UNCERTAIN - TLS Stream Blocking

**Location**: `crates/scred-http/src/streaming_request.rs:71-80`

**Problem**:
```rust
upstream_writer.write_all(format!("{}\r\n", request_line).as_bytes()).await?;
upstream_writer.write_all(redacted_headers.as_bytes()).await?;
upstream_writer.flush().await?;
```

**Hypothesis**:
- Writing to TLS stream might block indefinitely
- Flush might not be working on TLS connection
- TLS might be in bad state after handshake

**Status**: ⚠️ **UNCERTAIN** - TLS handshake completes successfully (dns_resolver logs), but writes might still block

---

## Current Status (After Fixes)

### Tests Performed

✅ Compilation: PASS (no errors, only pre-existing warnings)
❌ Functional test: FAIL (still hangs)

### Evidence

Logs show:
1. Proxy starts: ✅
2. Accepts connection: ✅  
3. Reads request line: ✅
4. Connects upstream: ✅
5. **Then hangs** ❌ (likely in stream_request_to_upstream)

Debug logging added to `read_response_line()` is NEVER reached, confirming hang happens before attempting to read response.

---

## Fixes Applied

### ✅ Fix 1: Removed Extra `\r\n`

**File**: `crates/scred-http/src/streaming_request.rs`

```rust
// BEFORE (buggy):
upstream_writer.write_all(redacted_headers.as_bytes()).await?;
upstream_writer.write_all(b"\r\n").await?;

// AFTER (fixed):
// NOTE: raw_headers already includes the final \r\n blank line, don't add another!
upstream_writer.write_all(redacted_headers.as_bytes()).await?;
```

### ✅ Fix 2: BufReader Wrapping

**File**: `crates/scred-proxy/src/main.rs:115-142 and 144-172`

```rust
// BEFORE (buggy):
let mut upstream = BufReader::new(tls_stream);
stream_request_to_upstream(&mut client_reader, &mut upstream, ...)?;
let response_line = read_response_line(&mut upstream)?;

// AFTER (fixed):
let mut upstream = tls_stream;
stream_request_to_upstream(&mut client_reader, &mut upstream, ...)?;
let response_line = read_response_line(&mut upstream)?;
let mut upstream_buf = BufReader::new(upstream);
stream_response_to_client(&mut upstream_buf, ...)?;
```

### ✅ Fix 3: Debug Tracing

**File**: `crates/scred-http/src/http_line_reader.rs:79`

Added detailed tracing to `read_response_line()`:
- Track bytes read
- Log response line when complete
- Show error details if hang/failure occurs

---

## Why Proxy Still Hangs (Unresolved)

Despite fixing the identified issues, proxy still hangs. This suggests:

1. **The fixes weren't the root cause** - They were correctness improvements but not the hang source
2. **Hang is still in stream_request_to_upstream** - Debug logging shows `read_response_line` never reached
3. **Likely culprits**:
   - Deadlock between client reading and upstream writing
   - Buffer exhaustion causing sync point
   - Async runtime issue with multiple mutable borrows
   - TLS stream state management

---

## Recommended Next Steps

### Short Term (30 min)

1. **Add systematic tracing to stream_request_to_upstream**:
   - Log at entry
   - Log after parse_http_headers
   - Log after each write
   - Log after flush
   - Identify exact hang point

2. **Simplify the proxy logic**:
   - Read entire request into buffer (non-streaming)
   - Forward entire request
   - Read entire response
   - Forward entire response
   - This avoids complex streaming state management

### Medium Term (1-2 hours)

1. **Redesign streaming architecture**:
   - Separate read/write phases clearly
   - Use explicit buffer management
   - Consider request/response buffering layer

2. **Add integration tests**:
   - Test with local echo server
   - Test with real upstream (httpbin)
   - Test both HTTP and HTTPS

### Long Term

1. **Consider using established proxy crate** if complexity persists
2. **Benchmark streaming vs buffering** approach
3. **Document streaming requirements** for future maintainers

---

## Code Changes Made

| File | Change | Status |
|------|--------|--------|
| streaming_request.rs:77 | Removed extra `\r\n` | ✅ COMMITTED |
| scred-proxy/main.rs:115-142 | Fixed BufReader wrapping | ✅ COMMITTED |
| http_line_reader.rs:79 | Added debug tracing | ✅ COMMITTED |

Commit: `3cc51d7` "🔧 WIP: Debug scred-proxy hanging issue"

---

## Testing Commands

```bash
# Start proxy pointing to httpbin
SCRED_PROXY_UPSTREAM_URL=https://httpbin.org/ \
  http_proxy="" https_proxy="" HTTP_PROXY="" HTTPS_PROXY="" \
  RUST_LOG=debug cargo run --bin scred-proxy

# In another terminal
curl -v http://localhost:9999/anything

# Expected (currently fails):
# 200 OK
# JSON response body
```

---

## Impact

- **Severity**: CRITICAL - Proxy completely non-functional
- **Affected Component**: scred-proxy binary
- **Workaround**: None (must fix)
- **User Impact**: Cannot use scred-proxy for HTTP/1.1 proxying

---

## Conclusion

Two bugs identified and partially fixed:
1. Extra `\r\n` in header forwarding
2. BufReader wrapping preventing writes

However, proxy still hangs. Root cause appears to be deeper architectural issue in how streaming request/response handlers interact with async I/O and buffering. Requires systematic debugging to trace exact hang point.

The fixes applied are correctness improvements but weren't sufficient to resolve the hanging. Suggest tracing approach outlined in "Recommended Next Steps" for final resolution.

