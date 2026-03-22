# HTTP/2 Scenarios Analysis

## Scenario 1: Client HTTP/1.1 → Upstream HTTP/2 (No Proxy, Direct MITM)

```
Client (H1.1)
    ↓ CONNECT example.com:443
SCRED MITM (TLS interception)
    ↓ Negotiates ALPN with upstream
Upstream (H2 negotiated)
```

**What MUST happen?**
- MITM is in the middle of a CONNECT tunnel
- Client sent HTTP/1.1 request
- MITM detected upstream wants HTTP/2
- **MITM MUST transcode H1.1→H2 going upstream, H2→H1.1 coming back**
- This is what the current H2UpstreamClient was trying to do

**Conclusion**: Must downgrade (transcode) ✅

---

## Scenario 2: Client HTTP/1.1 via Proxy HTTP/1.1 → Upstream HTTP/2

```
Client (H1.1)
    ↓ HTTP proxy (CONNECT example.com:443)
Proxy (HTTP/1.1 only)
    ↓ CONNECT to upstream
SCRED MITM (on proxy?)
    ↓ Negotiates ALPN
Upstream (H2)
```

**Questions:**
- Is SCRED running on the proxy?
- Or is SCRED a separate MITM intercepting the proxy's upstream connection?

**If SCRED is the upstream MITM:**
- Proxy sent HTTP/1.1 CONNECT
- SCRED MITM intercepts upstream H2
- **SCRED must transcode H1.1→H2 upstream, H2→H1.1 back to proxy**

**Conclusion**: Must downgrade (transcode) ✅

---

## Scenario 3: Client HTTP/2 via Proxy HTTP/1.1 → Upstream HTTP/2 ⚠️

This is the REAL issue.

```
Client (H2 with ALPN negotiation)
    ↓ HTTP/1.1 proxy tunnel (CONNECT example.com:443)
Proxy (HTTP/1.1 only - CANNOT speak H2)
    ↓ CONNECT tunnel (plain tunnel, no TLS/ALPN)
SCRED MITM
    ↓ Negotiates ALPN with upstream
Upstream (H2)
```

**The Problem:**
- Client wants H2 (sends "h2" in ALPN)
- Proxy is HTTP/1.1 only (doesn't support ALPN)
- Proxy downgrades to HTTP/1.1 for the tunnel
- Client sends HTTP/1.1 through proxy's CONNECT tunnel
- MITM sees HTTP/1.1 from proxy
- MITM sees HTTP/2 capability upstream
- But the original CLIENT wanted H2!

**What MUST happen?**
- MITM cannot know the client originally wanted H2 (it was negotiated before the proxy)
- From MITM's perspective: proxy sent HTTP/1.1
- So MITM must transcode H1.1→H2 upstream, H2→H1.1 back

**Conclusion**: From MITM's perspective, same as Scenario 2. Must downgrade. ✅

---

## Scenario 4: Client HTTP/2 (no proxy) → Upstream HTTP/2 ✅

```
Client (H2, ALPN "h2")
    ↓ TLS + ALPN negotiation
SCRED MITM (TLS termination)
    ↓ Negotiates ALPN "h2" with upstream
Upstream (H2)
```

**What MUST happen?**
- Client established H2 with MITM (ALPN "h2")
- MITM can speak H2 with client
- MITM can speak H2 with upstream
- **MITM should forward H2 frames bidirectionally**
- This is pure H2↔H2 forwarding with redaction

**Conclusion**: Use frame_forwarder, no transcoding needed ✅

---

## The Real Architecture Question

**Is the MITM part of the proxy chain or separate?**

### Case A: MITM is PART OF the proxy

```
Client → MITM/Proxy (same box) → Upstream
```

Then the MITM can:
- Negotiate ALPN with client
- Know what the client wanted
- Decide to support H2 or downgrade

### Case B: MITM is SEPARATE (intercepts proxy's connection)

```
Client → Proxy (H1.1 only) → MITM (TLS intercept) → Upstream
```

Then MITM only sees:
- HTTP/1.1 from proxy (CONNECT tunnel)
- Must handle upstream H2 protocol

---

## Current SCRED Architecture

Looking at the code:
- SCRED is both MITM AND proxy
- It terminates TLS and re-encrypts with upstream
- It's not in a chain with another proxy

So it's Case A: MITM can negotiate with client directly.

---

## Decision: What Should Phase 2 Do?

### Option 1: Proper H2 Support (Recommended)

When upstream is HTTP/2:
- If client sent H2 (ALPN "h2"): ✅ Use frame_forwarder for H2↔H2
- If client sent H1.1: Use H2UpstreamClient for H1.1→H2 transcoding

### Option 2: Always Downgrade (Current)

When upstream is HTTP/2:
- Always transcode to H1.1, regardless of client
- Simpler implementation
- But wastes H2 multiplexing if client wanted H2

### Option 3: Smart Routing

When upstream is HTTP/2:
- Check client protocol
- H2 client → H2↔H2 forwarding
- H1.1 client → H1.1→H2 transcoding

---

## Summary

| Scenario | Client | Upstream | MITM Must Do |
|----------|--------|----------|--------------|
| 1 | H1.1 | H2 | Transcode H1.1→H2↔H2→H1.1 |
| 2 | H1.1 | H2 (via proxy) | Transcode H1.1→H2↔H2→H1.1 |
| 3 | H2 (via proxy) | H2 | Effectively same as 1 (proxy sends H1.1) |
| 4 | H2 (direct) | H2 | Forward H2↔H2 (no transcode) |

**Key insight:** Only Scenario 4 (no proxy, both H2) can use pure frame forwarding.
All proxy scenarios MUST transcode because proxy only speaks H1.1.

**Phase 2 should:**
1. When upstream is H2:
   - Check client protocol (from ALPN)
   - If client is H2: Try frame_forwarder (Scenario 4)
   - If client is H1.1: Use H2UpstreamClient transcode (Scenarios 1-3)
