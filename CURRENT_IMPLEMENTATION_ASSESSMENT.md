# Current Implementation Assessment: HTTP/1.1, HTTP/2, Upstream Proxy

## What's Actually Implemented

### scred-proxy (Forward Proxy)
- ✓ HTTP/1.1 (fully implemented, streaming, Keep-Alive)
- ✗ HTTP/2 (NOT implemented in proxy)
- ✓ Upstream proxy (via `FixedUpstream` configuration)
- ✗ MITM HTTPS (forward proxy doesn't do TLS interception)
- ✓ Connection pooling (exists but unused: `connection_pool.rs`)
- ✓ DNS caching (exists: `dns_cache.rs`)
- ✓ SIMD redaction via streaming
- ✓ Per-path redaction rules
- ✓ Pattern selection (tier-based)

### scred-mitm (MITM Proxy)
- ✓ HTTP/1.1 (via `http_handler.rs`)
- ✓ HTTP/2 (via `h2_mitm_handler.rs` + h2 crate)
- ✗ Upstream proxy (NOT implemented in MITM)
- ✓ MITM HTTPS (full TLS interception with cert generation)
- ✓ SIMD redaction
- ✓ Per-stream redaction for HTTP/2

### Shared (scred-http)
- ✓ HTTP/1.1 parsing (optimized header reading)
- ✓ HTTP/2 frame handling (via h2 crate)
- ✓ HPACK compression (via h2 crate)
- ✓ TLS/mTLS support
- ✓ DNS resolver with exponential backoff (optimized)
- ✓ Upstream proxy support (`proxy_resolver.rs`)

---

## Current Speed Profile (from Phase 5)

### Measurements
- **Direct echo server**: 3.736 MB/s (baseline)
- **Proxy (HTTP/1.1 + Keep-Alive)**: 0.90 MB/s
- **Proxy overhead**: 3.5× (0.35ms per request)

### Where the Time Goes
```
Per-request breakdown (0.56ms total):
├─ Send to upstream: 0.02ms (4%)
├─ Receive from upstream: 0.53ms (96%) ← BOTTLENECK
│  ├─ Network round-trip: 0.19ms (upstream response time)
│  └─ Proxy processing: 0.34ms
│     ├─ Header parsing: 0.04ms
│     ├─ Redaction (SIMD): 0.05ms
│     ├─ Request/response formatting: 0.07ms
│     ├─ Async scheduling overhead: 0.05ms
│     └─ Buffer management: 0.13ms
```

### Bottleneck Analysis
- **Primary (96%)**: Network I/O - waiting for upstream response
- **Secondary (4%)**: Proxy processing (parsing, redaction, formatting)

---

## Feature Gap Analysis

| Feature | scred-proxy | scred-mitm | Status |
|---------|-------------|-----------|--------|
| HTTP/1.1 | ✓ Full | ✓ Full | Complete |
| HTTP/2 | ✗ None | ✓ Full | **Partially Complete** |
| Upstream Proxy | ✓ Impl | ✗ None | **Asymmetric** |
| MITM HTTPS | ✗ N/A | ✓ Full | **Asymmetric** |
| Keep-Alive | ✓ Yes | ✓ Yes | Complete |
| Connection Pool | ✓ Code exists | ? Untested | **Unused** |
| SIMD Redaction | ✓ Yes | ✓ Yes | Complete |
| Per-stream redaction | ✗ N/A | ✓ H2 only | **H2 only** |

---

## Speed Overhead Reassessment

### Current Proxy (HTTP/1.1)
```
Bottleneck: Network I/O (0.19ms upstream) + processing (0.34ms)
├─ Unavoidable: 0.19ms (upstream response time)
├─ Redaction: 0.05ms (unavoidable - it's the feature)
├─ Network hops (extra latency): 0.10ms (unavoidable - proxy architecture)
├─ Parsing/formatting: 0.04ms (PARTIALLY AVOIDABLE)
├─ Async overhead: 0.05ms (PARTIALLY AVOIDABLE via sync runtime)
└─ Unaccounted: 0.11ms (scheduling jitter, buffer overhead)

Total: 0.56ms per request = 0.90 MB/s
```

### If Using HTTP/2 (theoretical MITM)
```
Potential improvements:
- Multiplexed streams (5-10 concurrent): 5-10× RPS
- But MITM doesn't use upstream proxy, so no forwarding benefit
- Actual gain: UNKNOWN (not measured)

Remaining bottlenecks:
- Still need to parse binary frames (HPACK decompression)
- Still need to wait for upstream response
- Per-stream redaction not cheaper than per-request
- Network I/O still dominates
```

### If Using Upstream Proxy (current proxy only)
```
No speed impact on loopback (extra hops are negligible)
On real network (datacenter):
- Proxy to upstream: +0.5-1ms (within datacenter)
- DOES NOT reduce redaction overhead
- Actually INCREASES latency (extra hop)

Tradeoff: Functionality vs throughput
```

---

## How to Achieve Maximum Throughput

### Path A: Optimize Current (HTTP/1.1 Proxy) → 3-5 MB/s

**Step 1**: Enable connection pooling (code exists, currently unused)
```rust
// crates/scred-http/src/connection_pool.rs exists but:
// - Not wired into scred-proxy
// - Not tested with upstream
// - Could provide 3-5× with 10 concurrent connections
```

**Implementation**: Wire connection pool into `DnsResolver::connect_with_retry()`
- Time: 2-3 weeks
- Code changes: 300-500 LOC
- Expected gain: 3-5 MB/s (3-5 concurrent streams)

**Current issue**: Sequential connections, one at a time
```
Request 1: Connect (5ms) → Send (1ms) → Recv (200ms) → Total: 206ms
Request 2: Connect (5ms) → Send (1ms) → Recv (200ms) → Total: 206ms
Sequential: 412ms for 2 requests = 0.97 MB/s

With pool (2 connections):
Request 1: Connect (5ms) [connection A] → Wait (200ms) → Recv (200ms)
Request 2: Connect (5ms) [connection B] → Wait (200ms) → Recv (200ms)
Parallel: 405ms total = 1.9 MB/s (2× speedup)
```

**With 10 connections**: 5-10× speedup possible (if upstream can handle)

---

### Path B: HTTP/2 Support for Proxy → 5-15 MB/s (MAJOR EFFORT)

**Current state**: HTTP/2 only works in MITM, not proxy

**Why not proxy**: 
- Proxy is for HTTP/1.1 upstream (not HTTP/2 capable)
- Would need to support upgrading HTTP/2 client → HTTP/1.1 upstream conversion
- Or require upstream to support HTTP/2
- Complex: h2 crate on client side, need custom multiplexing

**Implementation**:
1. Add h2 server support to proxy (like MITM has)
2. Multiplex client HTTP/2 streams to upstream
3. Each stream gets independent connection (OR pool them)
4. Per-stream redaction (already implemented in H2MitmHandler pattern)

**Time**: 4-8 weeks
**Expected gain**: 5-15 MB/s (multiplexing 5-15 concurrent streams)
**Code**: 1000+ LOC

---

### Path C: Hybrid Approach (Recommended) → 5-20 MB/s

**Combination**:
1. Enable connection pooling in HTTP/1.1 proxy (3 weeks) → 3-5 MB/s
2. Add HTTP/2 support to proxy (6 weeks) → 5-15 MB/s
3. Support both HTTP/1.1 and HTTP/2 clients → handles any client

**Total time**: 8-9 weeks
**Total gain**: 5-15 MB/s (depending on upstream capacity)

---

### Path D: Reassess Fundamentals → 20-50 MB/s (Raw TCP)

**Only if HTTP features not needed**:
- Drop upstream proxy support
- Drop HTTP parsing entirely
- Use raw TCP + SIMD redaction
- Potential: 20-50 MB/s

**But**: Loses all HTTP-aware features (routing, HTTPS, etc.)

---

## Specific Optimization Opportunities

### 1. **Connection Pooling** (Easy, 3-5× gain)
```
Code location: crates/scred-http/src/connection_pool.rs
Status: EXISTS but UNUSED
Action: Wire into proxy's DnsResolver
Time: 2-3 weeks
Gain: 3-5 MB/s
Risk: Low (proven pattern)
```

### 2. **DNS Caching** (Easy, 5-10% gain)
```
Code location: crates/scred-http/src/dns_cache.rs
Status: EXISTS but UNUSED
Action: Enable in proxy
Time: 1 week
Gain: 0.05-0.1 MB/s (saves 1-5ms DNS per request)
Risk: Low (cached DNS might stale)
```

### 3. **Remove Logging Overhead** (Easy, 5% gain)
```
Current: info!() on every request
Cost: ~0.01ms per request per log statement
Action: Reduce to debug!/warn! only
Time: 1 week
Gain: 0.05 MB/s
Risk: None (development tradeoff)
```

### 4. **Buffer Reuse** (Medium, 2-3% gain)
```
Current: Allocate new buffers per request
Could: Use ArrayPool or buffer cache
Code: Not yet implemented
Time: 2-3 weeks
Gain: 0.02-0.03 MB/s
Risk: Medium (complex lifecycle)
```

### 5. **Async Runtime Tuning** (Medium, 5-10% gain)
```
Current: Default tokio runtime
Could: Use glommio or custom executor
Time: 2-4 weeks
Gain: 0.05-0.1 MB/s
Risk: High (different runtime semantics)
```

### 6. **HTTP/2 Multiplexing** (Hard, 5-15× gain)
```
Current: HTTP/1.1 only in proxy
Exists in MITM: h2_mitm_handler.rs
Action: Port pattern to proxy
Time: 4-8 weeks
Gain: 5-15 MB/s (with 5-15 concurrent streams)
Risk: High (complex state management)
```

---

## Realistic Speed Targets

| Optimization | Time | Code | Gain | Cumulative MB/s |
|---|---|---|---|---|
| Baseline | - | - | - | 0.90 |
| + Logging reduction | 1w | 50 | +5% | 0.95 |
| + DNS caching | 1w | 100 | +5% | 1.0 |
| + Connection pool (2 conn) | 2w | 200 | +100% | 2.0 |
| + Connection pool (5 conn) | 2w | 200 | +300% | 3.5 |
| + Connection pool (10 conn) | 2w | 200 | +500% | 5.4 |
| + HTTP/2 (5 concurrent streams) | 6w | 1000 | +200% | 16.2 |
| + HTTP/2 (10 concurrent streams) | 6w | 1000 | +400% | 26.0 |

**Realistic near-term target**: 3-5 MB/s (via connection pooling, 2-3 weeks)
**Medium-term target**: 5-15 MB/s (add HTTP/2, 4-8 additional weeks)

---

## Implementation Priority

### Phase 1 (IMMEDIATE): Connection Pooling
- **Time**: 2-3 weeks
- **Gain**: 3-5 MB/s
- **Code**: Exist, just wire it up
- **Risk**: LOW
- **Recommendation**: DO THIS FIRST

### Phase 2 (MEDIUM): DNS + Logging
- **Time**: 1-2 weeks
- **Gain**: +10% (0.9-1.0 MB/s additional)
- **Risk**: LOW
- **Recommendation**: Can do while Phase 1 is in progress

### Phase 3 (LONG-TERM): HTTP/2
- **Time**: 4-8 weeks
- **Gain**: 5-15 MB/s additional
- **Risk**: MEDIUM
- **Recommendation**: Only if throughput becomes critical

---

## Current Blocker

The **connection pooling code exists but is not wired up**!

```rust
// crates/scred-http/src/connection_pool.rs
pub struct ConnectionPool<T> { ... }

// But scred-proxy calls:
DnsResolver::connect_with_retry(&upstream_addr)
  // which creates NEW connection every request
  // instead of reusing from pool
```

**Quick win**: Enable the pool in proxy → 3-5 MB/s immediately.

