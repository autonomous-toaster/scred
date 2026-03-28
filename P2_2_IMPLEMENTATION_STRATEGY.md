# P2-2 Implementation Strategy: Multi-Upstream Pooling for MITM

**Status**: Ready to implement
**Complexity**: MEDIUM
**Timeline**: 2-3 weeks
**Code Reuse**: 90%

## What's Done

Created `MultiUpstreamPool` in scred-http:
- Per-upstream `PooledDnsResolver` management
- Automatic pool creation on first use
- Pool reuse across requests to same upstream

## What's Next

### Step 1: MITM Integration (2-3 days)
Modify `scred-mitm/src/mitm/upstream_connector.rs`:
- Accept `MultiUpstreamPool` parameter
- Use pool.get_or_create(upstream_addr) instead of DnsResolver::connect_with_retry
- Minimal code change (~20 LOC)

### Step 2: Logging Reduction (1 day)
Apply same pattern as P1-3 in MITM files:
- info!() → debug!() for per-request logs
- 4 files: http_handler.rs, h2_mitm_handler.rs, tls_mitm.rs, proxy.rs

### Step 3: Benchmarking (2-3 days)
- Test MITM with single upstream (verify pooling works)
- Test MITM with multiple upstreams (verify per-upstream pool separation)
- Measure throughput improvement vs baseline

## Code Structure

```
MITM Connection Flow:
  Client → MITM proxy
       → MitmConfig.get_proxy_for(host)
       → (if upstream) connect via MultiUpstreamPool
          ├─ Pool key: upstream_addr
          └─ Reuse TcpStream if cached
       → Upstream
```

## Expected Impact

**Single upstream scenario** (like scred-proxy):
- Before: New connection per request
- After: Reuse pooled connection
- Gain: 10-11× improvement (same as Phase 1)

**Multi-upstream scenario** (MITM advantage):
- 3 different upstreams at 1000 req/s total
- Each upstream: ~330 req/s with pooling
- Result: ~3× faster than sequential

## Next Phase Plan

After P2-2: Consider P2-1 (HTTP/2 multiplexing)
- Requires h2 crate version alignment
- Lower priority: Phase 1 already exceeded targets
- More complex: async streaming with h2

## Why This Approach

1. **Incremental**: Changes isolated to connection layer
2. **Safe**: MultiUpstreamPool already tested structure
3. **Parallel**: Can proceed independently from P2-1
4. **High value**: MITM becomes efficient for multi-upstream

## Files to Modify

- `crates/scred-mitm/src/mitm/upstream_connector.rs` (+30 LOC)
- `crates/scred-mitm/src/mitm/http_handler.rs` (-10 LOC logging)
- `crates/scred-mitm/src/mitm/h2_mitm_handler.rs` (-10 LOC logging)
- `crates/scred-mitm/src/main.rs` (+10 LOC init)

Total new code: ~30 LOC (rest is logging reduction)
