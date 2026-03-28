# Protocol Support Alignment: Executive Summary

## Problem Statement

**scred-proxy and scred-mitm have asymmetric protocol support**, creating code duplication and feature gaps:

| Feature | Proxy | MITM |
|---------|-------|------|
| HTTP/1.1 | ✓ | ✓ |
| HTTP/2 | ✗ | ✓ |
| Upstream proxy | ✓ | ✗ |
| Connection pooling | Code exists (UNUSED) | ? |
| DNS caching | Code exists (UNUSED) | ? |
| Throughput | 0.90 MB/s | Unknown |

**Impact**: Users must choose between features (MITM HTTPS XOR upstream proxy), code is duplicated across proxies, and performance is left on the table.

---

## Solution

Create **Protocol Support Alignment Initiative** to make both proxies architecturally identical via shared code in scred-http layer.

### Target State (ALL in scred-http, used by both proxies)
- ✓ HTTP/1.1 handler (unified)
- ✓ HTTP/2 handler (unified)
- ✓ Upstream proxy support (unified)
- ✓ Connection pooling (enabled)
- ✓ DNS caching (enabled)
- ✓ SIMD redaction (existing)

### Result
- **Same features** in both proxy deployments
- **Max throughput** for both (3-5 MB/s P1, 5-15 MB/s P2)
- **Zero duplication** (single source of truth)
- **Bug fixes apply to both** automatically

---

## Quick Wins (Phase 1)

### Available NOW - Zero New Code Needed
```
✓ Connection pool code exists (connection_pool.rs)
✓ DNS cache code exists (dns_cache.rs)
✓ HTTP/2 handler code exists (h2_mitm_handler.rs in MITM)
✓ Upstream proxy code exists (proxy_resolver.rs)
```

Just need to **wire them up and reuse**!

### P1-1: Enable Connection Pooling (2-3 weeks)
- **Current**: Creates new TCP connection per request → 0.90 MB/s
- **Solution**: Reuse 5-10 pooled connections → 3-5 MB/s
- **Code**: Create `PooledDnsResolver` wrapper (120 LOC)
- **Risk**: LOW (existing code, just needs wiring)
- **Gain**: 3-5 MB/s (~300% improvement)

### P1-2: Enable DNS Cache (1 week, parallel)
- Saves 1-5ms per cached DNS lookup
- Existing code just needs integration
- Risk: LOW

### P1-3: Reduce Logging Overhead (1 week, parallel)
- Change `info!()` to `debug!()` on per-request calls
- Risk: NONE

**Result after P1**: 3-5 MB/s achievable (vs 0.90 baseline)

---

## Medium-term Improvements (Phase 2)

### P2-1: HTTP/2 Support for Proxy (4-8 weeks)
- Port H2MitmHandler pattern from MITM to proxy
- Enable multiplexed requests (5-15 concurrent streams)
- **Expected**: 5-15 MB/s with multiplexing
- **Risk**: MEDIUM (complex state management)

### P2-2: Upstream Proxy Support for MITM (2-3 weeks, parallel)
- Use proxy_resolver.rs pattern in MITM
- Enable MITM → upstream forwarding
- **Risk**: MEDIUM (CONNECT tunneling)

**Result after P2**: Both proxies support all features, 5-15 MB/s achievable

---

## Architecture Consolidation (Phase 3)

### P3-1: Unified Protocol Handlers (3-4 weeks)
- Create `ProtocolHandler` trait in scred-http
- Move HTTP/1.1 logic to single implementation
- Move HTTP/2 logic to single implementation
- Both proxies use same handlers
- **Result**: Zero code duplication, maximum reuse

---

## Timeline & Effort

```
Week 1-2:  P1-1 Connection pooling (PRIMARY)
Week 1-2:  P1-2, P1-3 in parallel (DNS cache, logging)
Week 3-4:  Measure & validate P1 results
Week 5-8:  P2-1 HTTP/2 for proxy
Week 5-7:  P2-2 Upstream for MITM (parallel)
Week 9-12: P3 Consolidation
Week 13+:  Polish & production testing

Total: 12-13 weeks to full alignment
```

---

## Success Metrics

### Performance
- ✓ Week 2: 3-5 MB/s (connection pooling)
- ✓ Week 8: 5-15 MB/s (HTTP/2 multiplexing)
- ✓ Week 12: Parity between proxy and MITM

### Quality
- ✓ All existing tests pass
- ✓ New tests for pooling, HTTP/2, upstream
- ✓ Zero code duplication
- ✓ Zero regressions

### Features
- ✓ Both support HTTP/1.1 + HTTP/2
- ✓ Both support upstream proxy
- ✓ Both support pooling + caching
- ✓ Both can do MITM (configuration)

---

## Next Steps

### IMMEDIATE (Start Today)
1. ✅ Create todos (6 todos created)
2. ✅ Write detailed specs (P1-1 spec done)
3. **→ Start P1-1 implementation** (next session)

### This Week
- Implement PooledDnsResolver
- Wire pool into proxy
- Write and run tests
- Benchmark pooling effectiveness

### Next 2 Weeks
- Complete P1-2 (DNS cache)
- Complete P1-3 (logging reduction)
- Measure cumulative improvement (target: 3-5 MB/s)

### Following 4 Weeks
- Start P2-1 (HTTP/2 for proxy)
- Parallel: P2-2 (upstream for MITM)

---

## Todos Created

| ID | Task | Duration | Effort |
|---|---|---|---|
| TODO-d83bf818 | P1-1: Wire pool | 2-3 weeks | Low |
| TODO-f8242652 | P1-2: Enable DNS cache | 1 week | Low |
| TODO-355fcf4d | P1-3: Reduce logging | 1 week | None |
| TODO-3340a94c | P2: Consolidate protocol handlers | 2-3 weeks | Medium |
| TODO-f0056574 | P2-1: Add HTTP/2 to proxy | 4-8 weeks | Medium |
| TODO-7d104c9f | P2-2: Add upstream to MITM | 2-3 weeks | Medium |
| TODO-4146eafa | P3: Consolidate architecture | 3-4 weeks | High |

---

## Files Created

### Planning Documents
- **PROTOCOL_ALIGNMENT_ROADMAP.md** (466 lines)
  - Complete 12-week roadmap
  - All phases detailed
  - Risk mitigation
  - Success criteria

- **CURRENT_IMPLEMENTATION_ASSESSMENT.md** (316 lines)
  - What's currently implemented
  - Feature gaps
  - Speed analysis
  - Optimization opportunities

- **TCP_UPSTREAM_HTTP2_MITM_ANALYSIS.md** (238 lines)
  - Architecture trade-offs
  - Feature compatibility matrix
  - Why raw TCP breaks features

- **P1_1_CONNECTION_POOL_SPEC.md** (540 lines)
  - Detailed implementation spec
  - Code examples
  - Testing strategy
  - Rollout plan

---

## Key Insights

### 1. Code Already Exists
Connection pooling, DNS caching, HTTP/2 handling - all implemented but unused. Just need to wire them up.

### 2. Quick Wins Available
P1-1 alone gives 3-5 MB/s improvement in 2-3 weeks with minimal risk.

### 3. No Need for Raw TCP
Both proxies can reach 5-15 MB/s via HTTP/2 multiplexing while keeping all features. Raw TCP (20-50 MB/s) not needed.

### 4. Isomorphism Reduces Complexity
Having both proxies use identical code from scred-http eliminates maintenance burden and feature gaps.

### 5. Network I/O is Bottleneck
Throughput limited by network latency (0.19ms unavoidable), not protocol parsing. Smart architecture (pooling, multiplexing) needed, not CPU optimization.

---

## Recommendation

### Start P1-1 Immediately
- 2-3 week timeline
- LOW risk (existing code)
- HIGH impact (3-5 MB/s)
- Clear success metrics
- Detailed spec provided

### Track Progress with Todos
- 6 todos created for all phases
- Update status as work progresses
- Benchmark at each phase gate

### Achieve Feature Parity in 12 Weeks
- Week 4: 3-5 MB/s (pooling)
- Week 8: 5-15 MB/s (HTTP/2)
- Week 12: Unified architecture, zero duplication

---

## Bottom Line

**Scred can achieve 5-15 MB/s throughput with complete feature alignment (HTTP/1.1, HTTP/2, upstream, MITM, pooling) in 12 weeks by leveraging existing code and smart architecture.**

Start with P1-1 (connection pooling) this week for 3-5 MB/s improvement.
