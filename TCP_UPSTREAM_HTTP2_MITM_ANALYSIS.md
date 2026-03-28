# Raw TCP vs Upstream Proxy vs HTTP/2 vs MITM: Comprehensive Analysis

## Question
"Would switching to raw TCP prevent: using an upstream proxy, redacting in HTTP/2, prevent our MITM proxy to work?"

## Answer: YES - Raw TCP breaks ALL of these

| Feature | Raw TCP Works? | Reason |
|---------|---|---|
| Upstream proxy | ❌ | Can't parse Host header to route |
| HTTP/2 redaction | ❌ | Can't parse binary HPACK frames |
| MITM HTTPS | ❌ | Can't parse CONNECT requests |
| Direct pass-through | ✓ | Just forward bytes blindly |

---

## Why Each Breaks

### 1. Raw TCP + Upstream Proxy: ❌ BROKEN

**Problem**: Need to parse HTTP to route correctly

```
Client: "GET / HTTP/1.1\r\nHost: example.com\r\n"
        └─ Raw TCP doesn't parse this
        └─ Doesn't know Host: example.com
        └─ Blindly forwards to upstream
        
Upstream: "Here's a request from our proxy, but where does it go?"
          └─ Can't determine destination
          └─ Returns 400 Bad Gateway
          └─ All requests fail
```

**Specific issue**:
- Raw TCP sees bytes: `[0x47 0x45 0x54 ...]` (GET in hex)
- Doesn't recognize this is HTTP
- Doesn't extract Host header
- Can't route to correct upstream destination

### 2. Raw TCP + HTTP/2: ❌ BROKEN

**Problem**: HTTP/2 is binary + compressed, SIMD can't find patterns

```
HTTP/1.1 (plaintext):
GET / HTTP/1.1\r\nAuthorization: Bearer secret-token\r\n

HTTP/2 (binary):
[0x00 0x00 0x26 0x04 0x25 0x00 0x00 0x00 0x01 ...]
[0x82 0x86 0x84 0x41 0x0f 0x77 0x77 ...]
(all compressed with HPACK)
```

Raw TCP + SIMD:
- Sees bytes: `[0x00 0x00 0x26 ...]`
- Can't decompress HPACK
- SIMD pattern matching on compressed garbage
- No secrets found → **Secrets leak**

**Specific issue**:
- HPACK compression is stateful
- Can't pattern-match compressed data
- Must decompress to find headers
- Raw TCP has no decompression

### 3. Raw TCP + MITM: ❌ BROKEN

**Problem**: MITM requires parsing CONNECT method for HTTPS tunneling

```
Client (HTTPS): "CONNECT example.com:443 HTTP/1.1\r\n\r\n"
                └─ This is HTTP!
                └─ Need to parse to recognize CONNECT
                └─ Need to establish TLS tunnel
                └─ Need to generate MITM certificate

Raw TCP:
├─ Sees bytes: [0x43 0x4F 0x4E 0x4E ...] (CONNECT in hex)
├─ Doesn't recognize this
├─ Tries to forward as data
└─ Server rejects it (not HTTP)
   └─ TLS tunnel never established
   └─ MITM completely broken
```

**Specific issue**:
- CONNECT is an HTTP method, must be parsed as HTTP
- Raw TCP doesn't understand HTTP
- Can't distinguish CONNECT from normal data
- HTTPS interception impossible

---

## The Fundamental Reason

**Raw TCP is fast BECAUSE it doesn't understand HTTP.**

To support any HTTP feature:
- Must parse HTTP (slower)
- Must understand HTTP semantics (complex)
- Must do something intelligent with headers (processing)

Raw TCP avoids all of this → faster, but dumb

```
Speed vs Intelligence Trade-off:

Raw TCP:  Fast (20-50 MB/s) → Dumb (no HTTP understanding)
HTTP/1.1: Moderate (1 MB/s) → Smart (full HTTP semantics)
HTTP/2:   Slower (5-15 MB/s) → Complex (binary parsing + compression)
```

---

## Feature Compatibility

**INCOMPATIBLE** with Raw TCP:
- ❌ Upstream proxy (need to route by Host header)
- ❌ HTTP/2 redaction (need HPACK decompression)
- ❌ MITM HTTPS (need to parse CONNECT)
- ❌ Proxy authentication (need to parse headers)
- ❌ Request routing (need HTTP semantics)

**COMPATIBLE** with Raw TCP:
- ✓ Blind byte forwarding
- ✓ Pattern matching on plaintext (HTTP/1.1 only)
- ✓ Simple SIMD redaction
- ✓ Maximum throughput

---

## Design Choices: You Must Pick ONE

### Path A: Raw TCP (Fastest)
```
Throughput: 20-50 MB/s
Features: NONE (just forward)
Use case: Pure monitoring gateway
Limitations: No HTTPS, no upstream, no HTTP/2
```

### Path B: HTTP/1.1 (Balanced) ← CURRENT
```
Throughput: 1 MB/s
Features: MITM HTTPS, upstream proxy, routing
Use case: Corporate gateway with full features
Limitations: No HTTP/2, slower throughput
```

### Path C: HTTP/2 (Complex)
```
Throughput: 5-15 MB/s
Features: MITM HTTPS, HTTP/2 multiplexing, upstream
Use case: Modern client support
Limitations: Complex (HPACK, binary frames), no HTTP/1.1 bypass
```

### Path D: All Features (Extremely Complex)
```
Throughput: 0.5-3 MB/s
Features: Everything (HTTP/1.1 + HTTP/2 + MITM + upstream)
Use case: Enterprise (everything possible)
Limitations: Huge codebase, hard to maintain
Code: ~5000+ lines, multiple code paths
```

---

## Why You Can't Mix Them

The problem is **protocol detection overhead**.

If you try to support both Raw TCP and HTTP/1.1:
```rust
// Pseudo-code
loop {
    // First byte determines protocol?
    let first_byte = read_byte();
    
    if first_byte == 0x47 { // 'G' in "GET"
        // HTTP detected
        parse_as_http1()
    } else if first_byte == 0x43 { // 'C' in "CONNECT"
        // HTTPS tunnel
        parse_as_http1() // WAIT, also HTTP!
    } else if first_byte == 0x50 { // 'P' in "PRI" (HTTP/2 preface)
        // HTTP/2 detected
        parse_as_http2()
    } else {
        // Raw TCP?
        forward_blindly()
    }
}
```

**The issue**: You still need to understand protocols to distinguish them!
- Once you detect protocol, you're parsing anyway
- No throughput gain from "mixed mode"
- Extra code complexity for no benefit

---

## My Recommendation

### DON'T Switch to Raw TCP if You Need Features

**Current (HTTP/1.1)**: ✓ GOOD
- 1 MB/s throughput is acceptable
- MITM works (need it for HTTPS)
- Can add upstream proxy (same codebase)
- Can add HTTP/2 later (same base, just add frames)

### If You Need More Throughput: Incremental Path
1. **Add connection pooling** (3-5 weeks) → 3-5 MB/s
2. **Add HTTP/2** (4-8 weeks) → 5-15 MB/s
3. **Add raw TCP mode** (2-4 weeks, separate code path) → 20-50 MB/s for monitoring-only use cases

### DO NOT
- ❌ Replace current HTTP/1.1 with raw TCP
- ❌ Try to mix raw TCP with MITM
- ❌ Use raw TCP with upstream proxy
- ❌ Attempt HTTP/2 with raw TCP

---

## Summary

| Feature | HTTP/1.1 | HTTP/2 | Raw TCP |
|---------|----------|--------|---------|
| Upstream proxy | ✓ | ✓ | ❌ |
| HTTP/2 redaction | ❌ | ✓ | ❌ |
| MITM HTTPS | ✓ | ✓ | ❌ |
| Direct pass-through | ✓ | ✓ | ✓ |
| Throughput | 1 MB/s | 5-15 MB/s | 20-50 MB/s |
| Complexity | Low | High | Very Low |

**Answer to your question**: Switching to raw TCP would break upstream proxy, HTTP/2 redaction, AND MITM. Choose one path and build on it.
