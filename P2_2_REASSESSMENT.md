# P2-2 Reassessment: MITM as Generic HTTP Proxy

**Date**: 2026-03-28
**Status**: Architecture clarified, scope corrected

## Critical Discovery

The MITM is **NOT** a single-upstream proxy like scred-proxy.

**MITM is a generic HTTP proxy** that can route to ANY upstream based on standard proxy environment variables (HTTP_PROXY, HTTPS_PROXY, NO_PROXY).

## Current MITM Architecture

### How MITM Works Today

```
Client → MITM (listens on :8080 or :443)
       → Accepts HTTP/HTTPS requests
       → Reads MitmConfig (HTTP_PROXY, HTTPS_PROXY, NO_PROXY)
       → Routes to appropriate upstream per target host
       → Or direct connection if NO_PROXY matches
```

### MitmConfig Smart Routing

Located in `scred-http/src/proxy_resolver.rs`:

```rust
pub fn get_proxy_for(&self, target_host: &str, is_https: bool) -> Option<String>
```

Logic:
1. Check if host is in NO_PROXY list (wildcards, CIDR, suffixes supported)
2. If matched: direct connection (return None)
3. Otherwise: return HTTP_PROXY or HTTPS_PROXY depending on protocol

Example:
```bash
# MITM can forward to different upstreams
export HTTP_PROXY="http://proxy1.com:3128"
export HTTPS_PROXY="http://proxy2.com:3128"
export NO_PROXY="localhost,127.0.0.1,.local"

# Requests to any host → routed through appropriate proxy
# Requests to localhost/.local → direct connection
```

## What P2-2 Actually Means

**Original Understanding** (WRONG):
- "Add upstream to MITM" = Add ability to target an upstream

**Correct Understanding** (RIGHT):
- "Add upstream to MITM" = Enable MITM to reuse Phase 1 optimizations
  - Connection pooling per upstream
  - DNS caching per upstream
  - Efficient multi-upstream proxying

## Required Changes for P2-2

### Current Gap

MITM has smart routing (MitmConfig) but lacks optimizations:
- ❌ No connection pooling (new connection per upstream request)
- ❌ No DNS caching (fresh lookup per unique hostname)
- ❌ No logging reduction (per-request info logs)

### What Phase 1 Gave scred-proxy

✅ OptimizedDnsResolver (pooling + caching unified)
✅ Per-upstream connection pool
✅ DNS cache (60s TTL)
✅ Logging optimization

### What P2-2 Needs to Implement

1. **Integrate OptimizedDnsResolver into MITM**
   - Same as scred-proxy
   - Each upstream gets its own pool

2. **Enable pooling per upstream**
   - MitmConfig returns upstream URL
   - Pool keyed by upstream address
   - Dynamic pool creation for new upstreams

3. **Reduce logging overhead in MITM**
   - Per-request info!() → debug!()
   - Keep startup logs

## Architectural Implication

### MITM is Already More Complex Than Proxy

**scred-proxy** (simple):
```
Request → FixedUpstream (single target)
        → OptimizedDnsResolver
        → 10-connection pool (to that one upstream)
        → Response
```

**scred-mitm** (complex, generic):
```
Request → MitmConfig.get_proxy_for(target_host)
        → Might get multiple different upstreams
        → OptimizedDnsResolver (each upstream)
        → Multi-key connection pool (per upstream)
        → Response
```

### Why This is Actually Good News

1. **MITM was already designed for this**: MitmConfig exists
2. **No new concept needed**: Just add pooling to existing routing
3. **Better performance**: Multi-upstream proxying becomes efficient
4. **Standard approach**: Uses HTTP_PROXY env vars (not custom)

## P2-2 Implementation Strategy

### Phase 1 Reuse (90% code reuse)

MITM can use the same code:
- ✅ OptimizedDnsResolver (identical, no changes needed)
- ✅ CachedDnsResolver (identical, no changes needed)
- ✅ PooledDnsResolver (identical, with multi-key pool)
- ✅ Logging reduction (identical pattern)

### What's Different

Instead of single pool:
```rust
// scred-proxy (single upstream)
let pool = ConnectionPool::new(upstream_url, 10);

// scred-mitm (multi-upstream)
let pools: HashMap<String, ConnectionPool> = HashMap::new();
// For each request:
let upstream = mitm_config.get_proxy_for(host)?;
let pool = pools.entry(upstream).or_insert(
    ConnectionPool::new(upstream, 10)
);
```

## Correct P2-2 Scope

### What It IS
- ✅ Enable MITM to use OptimizedDnsResolver
- ✅ Add multi-upstream connection pooling to MITM
- ✅ Apply logging reduction (same as P1-3)
- ✅ Measure performance improvement

### What It IS NOT
- ❌ Add ability to route to upstreams (already exists via MitmConfig)
- ❌ Change MITM's generic proxy nature
- ❌ Implement new routing logic

### Estimated Scope

| Item | Effort | Risk | Reuse |
|------|--------|------|-------|
| Integrate OptimizedDnsResolver | 2-4 hours | LOW | 100% from proxy |
| Multi-key connection pool | 4-8 hours | LOW | 90% from proxy |
| Logging reduction | 1-2 hours | LOW | 100% from proxy |
| Testing | 4-6 hours | MEDIUM | 80% test patterns |
| Benchmarking | 2-3 hours | LOW | 100% from proxy |
| **TOTAL** | **13-23 hours** | **MEDIUM** | **90%** |

**Timeline**: 2-3 weeks (matches original P2-2 estimate)

## Why This Matters for Architecture

### MITM's True Role in System

```
Users wanting to intercept/redact traffic:
  ├─ Small scale (single backend): Use scred-proxy
  │  └─ Simpler, hardcoded upstream
  │
  └─ Large scale (multiple backends): Use scred-mitm
     ├─ Intercept any traffic
     ├─ Route to different upstreams via NO_PROXY
     └─ Redact across entire network
```

### Feature Parity Meaning

**After P2-2**:
- ✅ Both proxy and MITM have pooling + caching
- ✅ Both support HTTP/1.1 and HTTP/2 (after P2-1)
- ✅ scred-proxy: Single upstream, optimized
- ✅ scred-mitm: Any upstream, generic, equally fast

## Conclusion

**P2-2 is correctly scoped as**:
> Enable MITM to efficiently proxy requests to ANY upstream via
> connection pooling and DNS caching (same as Phase 1 for scred-proxy)

**Status**: Ready to implement
**Risk**: Medium (multi-key pool is slightly complex)
**Timeline**: 2-3 weeks
**Code Reuse**: 90% from Phase 1

The MITM isn't getting upstream support - it already has it.
It's getting performance optimization for generic multi-upstream proxying.
