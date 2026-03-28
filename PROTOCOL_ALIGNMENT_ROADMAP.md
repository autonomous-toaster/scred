# Protocol Support Alignment Roadmap

## Goal
Make scred-proxy and scred-mitm **architecturally identical on protocol support** with all code in scred-http for maximum reuse and performance.

## Current Asymmetry

### scred-proxy (Forward Proxy)
- ✓ HTTP/1.1 (streaming_request.rs, streaming_response.rs)
- ✗ HTTP/2 (NOT supported)
- ✓ Upstream proxy (FixedUpstream, proxy_resolver.rs)
- ✗ MITM HTTPS (forward proxy doesn't intercept)
- ✓ Connection pool code exists (UNUSED)
- ✓ DNS cache code exists (UNUSED)
- ⚠️ Speed: 0.90 MB/s (bottleneck: sequential connections, no pooling)

### scred-mitm (MITM Proxy)
- ✓ HTTP/1.1 (http_handler.rs)
- ✓ HTTP/2 (h2_mitm_handler.rs + h2 crate)
- ✗ Upstream proxy (NOT supported)
- ✓ MITM HTTPS (TLS interception, cert generation)
- ⚠️ Speed: Unknown (HTTP/2 multiplexing enabled but not benchmarked)

### Shared Layer (scred-http)
- ✓ HTTP/1.1 parser, streaming handlers
- ✓ HTTP/2 frame handling (h2 crate)
- ✓ HPACK compression (h2 crate)
- ✓ Upstream proxy support (proxy_resolver.rs)
- ✓ Connection pool (connection_pool.rs) - UNUSED
- ✓ DNS cache (dns_cache.rs) - UNUSED

## Target State

### Both Proxy and MITM Support

| Feature | scred-proxy | scred-mitm | scred-http |
|---------|---|---|---|
| HTTP/1.1 | ✓ | ✓ | ✓ Unified |
| HTTP/2 | ✓ | ✓ | ✓ Unified |
| Upstream proxy | ✓ | ✓ | ✓ Unified |
| MITM HTTPS | ✓ | ✓ | ✓ Config |
| Connection pool | ✓ Active | ✓ Active | ✓ Unified |
| DNS cache | ✓ Active | ✓ Active | ✓ Unified |
| Code duplication | ✗ None | ✗ None | ✓ Single source |

### Benefits

1. **Maximum Reuse**: Both proxies use identical protocol handlers from scred-http
2. **Consistent Features**: Same capabilities in both deployments
3. **Speed Parity**: Both benefit from connection pooling, HTTP/2 multiplexing
4. **Maintainability**: Bug fixes and optimizations apply to both automatically
5. **Testing**: Protocol tests apply to both proxies simultaneously
6. **Extensibility**: New protocols/features added once in scred-http

## Implementation Roadmap

### Phase 1: Quick Wins (4-5 weeks, Parallel work)

**Goal**: 3-5 MB/s throughput via connection pooling, enable existing unused code

#### P1-1: Wire Connection Pool into Proxy (2-3 weeks)
**What**: Enable `connection_pool.rs` in scred-proxy
**How**: 
- Test connection_pool.rs implementation
- Modify DnsResolver::connect_with_retry() to use pool
- Add configuration (max connections: 5-10, idle timeout: 30s)
- Benchmark with concurrent requests

**Files to modify**:
- crates/scred-http/src/dns_resolver.rs (use pool)
- crates/scred-http/src/lib.rs (export pool config)
- crates/scred-proxy/src/main.rs (configure pool)
- crates/scred-proxy/Cargo.toml (dependencies)

**Expected**: 0.90 MB/s → 3-5 MB/s (with 5-10 concurrent connections)
**Risk**: LOW (code exists, just needs wiring)

**Tests**:
- Concurrent requests to single upstream
- Concurrent requests to multiple upstreams
- Connection reuse verification
- Idle timeout cleanup

---

#### P1-2: Enable DNS Cache in Proxy (1 week, parallel)
**What**: Enable `dns_cache.rs` in scred-proxy
**How**:
- Review dns_cache.rs implementation
- Integrate into DnsResolver initialization
- Add TTL configuration (default: 60s)
- Benchmark DNS hits vs direct lookup

**Files to modify**:
- crates/scred-http/src/dns_resolver.rs (use cache)
- crates/scred-http/src/lib.rs (export cache config)
- crates/scred-proxy/src/main.rs (configure cache)

**Expected**: Save 1-5ms per cached request
**Risk**: LOW (cache invalidation is graceful)

**Tests**:
- Cache hit rates
- TTL expiration
- Multiple domains

---

#### P1-3: Reduce Logging Overhead (1 week, parallel)
**What**: Optimize logging to reduce per-request cost
**How**:
- Change `info!()` on every request to `debug!()`
- Keep `info!()` for startup/config/errors
- Keep `warn!()` for anomalies

**Files to modify**:
- crates/scred-proxy/src/main.rs (reduce info!)
- crates/scred-http/src/dns_resolver.rs (reduce info!)
- crates/scred-http/src/streaming_request.rs (reduce info!)
- crates/scred-http/src/streaming_response.rs (reduce info!)

**Expected**: Save ~0.05 MB/s (logging overhead)
**Risk**: NONE (just logging levels)

**Tests**:
- Release build performance
- Debug output availability

---

**Phase 1 Result**: Both proxies have connection pooling enabled, 3-5 MB/s achievable

---

### Phase 2: Feature Parity (8-11 weeks, sequential after P1)

#### P2-1: Add HTTP/2 Support to scred-proxy (4-8 weeks)
**What**: Enable HTTP/2 client support in scred-proxy (like MITM has)
**Why**: HTTP/2 multiplexing enables 5-15× throughput with concurrent streams

**Current state**:
- HTTP/2 fully working in scred-mitm (h2_mitm_handler.rs)
- Code patterns proven and tested
- Just needs to be used by proxy

**How**:
1. Review H2MitmHandler pattern in scred-mitm
2. Create H2ProxyHandler (reuse pattern)
3. Implement protocol negotiation in proxy main.rs
4. Add ALPN support for HTTP/2 upgrade
5. Implement per-stream request forwarding
6. Handle HTTP/2 → HTTP/1.1 conversion (stream → sequential)

**Files to create**:
- crates/scred-http/src/h2_proxy_handler.rs (new)

**Files to modify**:
- crates/scred-proxy/src/main.rs (add HTTP/2 server)
- crates/scred-http/src/lib.rs (export H2ProxyHandler)
- crates/scred-http/src/h2/mod.rs (add proxy components)

**Architecture**:
```rust
// Protocol detection in proxy main.rs
match client_stream_protocol {
    Protocol::Http1 => handle_http1(client, config).await,
    Protocol::Http2 => handle_http2(client, config).await,
}

// Both handlers reuse from scred-http
fn handle_http1() {
    let handler = Http1Handler::new(config);
    handler.process_request().await
}

fn handle_http2() {
    let handler = Http2Handler::new(config);
    handler.process_stream().await
}
```

**Expected**: 5-15 MB/s with multiplexed streams (5-15 concurrent)
**Risk**: MEDIUM (complex stream state management)

**Tests**:
- HTTP/2 client → proxy → HTTP/1.1 upstream
- Multiplexed streams
- Stream prioritization
- Flow control

**Benchmarks**:
- Single stream (baseline)
- 5 concurrent streams
- 10 concurrent streams
- 20 concurrent streams (max)

---

#### P2-2: Add Upstream Proxy Support to scred-mitm (2-3 weeks, parallel)
**What**: Enable MITM proxy to forward to upstream proxy
**Why**: Feature parity with scred-proxy

**Current state**:
- scred-proxy has FixedUpstream configuration
- scred-mitm has NO upstream support
- Code patterns exist in proxy_resolver.rs

**How**:
1. Add UpstreamConfig to MITM configuration
2. Modify HTTP/1.1 handler to support upstream forwarding
3. Modify H2MitmHandler to support upstream forwarding
4. Implement CONNECT tunneling with upstream
5. Add Via header support

**Files to modify**:
- crates/scred-mitm/src/config.rs (add UpstreamConfig)
- crates/scred-mitm/src/mitm/http_handler.rs (use upstream)
- crates/scred-mitm/src/mitm/h2_mitm_handler.rs (use upstream)

**Expected**: MITM can forward to upstream
**Risk**: MEDIUM (CONNECT with upstream requires careful handling)

**Tests**:
- MITM → upstream (HTTP/1.1)
- MITM → upstream (HTTPS with CONNECT)
- MITM → upstream (HTTP/2)
- Multiple hops

---

**Phase 2 Result**: Both proxies support all features (HTTP/1.1, HTTP/2, upstream, pooling)

---

### Phase 3: Code Consolidation (3-4 weeks, after P2)

**Goal**: Single source of truth for each protocol in scred-http

#### P3-1: Create Unified Protocol Handler Architecture
**What**: Abstract protocol handlers into a trait-based system
**Why**: Eliminate duplicate code, enable reuse

**Current state**:
- HTTP/1.1 handler in scred-proxy (streaming_*.rs)
- HTTP/1.1 handler in scred-mitm (http_handler.rs)
- HTTP/2 handler in scred-mitm (h2_mitm_handler.rs)
- HTTP/2 handler in scred-proxy (to be added in P2-1)
- Lots of duplication

**Design**:
```rust
// In scred-http/src/protocol_handler.rs

pub trait ProtocolHandler: Send + Sync {
    type Request: Send + Sync;
    type Response: Send + Sync;
    
    async fn handle_request(
        &self,
        req: Self::Request,
        upstream: &UpstreamConfig,
        redactor: Arc<StreamingRedactor>,
    ) -> Result<Self::Response>;
}

pub struct Http1Handler {
    config: Http1Config,
}

impl ProtocolHandler for Http1Handler {
    // Unified HTTP/1.1 logic (currently split across proxy and MITM)
}

pub struct Http2Handler {
    config: Http2Config,
}

impl ProtocolHandler for Http2Handler {
    // Unified HTTP/2 logic (currently split across proxy and MITM)
}

pub struct ProtocolFactory;
impl ProtocolFactory {
    pub fn create(protocol: Protocol, config: &ProxyConfig) -> Box<dyn ProtocolHandler> {
        match protocol {
            Protocol::Http1 => Box::new(Http1Handler::new(config)),
            Protocol::Http2 => Box::new(Http2Handler::new(config)),
        }
    }
}
```

**Files to create**:
- crates/scred-http/src/protocol_handler.rs (trait)
- crates/scred-http/src/protocol_handler/http1.rs (HTTP/1.1 impl)
- crates/scred-http/src/protocol_handler/http2.rs (HTTP/2 impl)
- crates/scred-http/src/protocol_handler/factory.rs (creation)

**Files to remove** (duplicate code):
- crates/scred-proxy/src/... (streaming handlers moved to scred-http)
- crates/scred-mitm/src/... (duplicate handlers removed)

**Refactoring steps**:
1. Audit current handlers in both proxies
2. Extract common logic to trait methods
3. Move protocol-specific logic to implementations
4. Create factory for protocol selection
5. Update proxy main.rs to use factory
6. Update MITM proxy to use factory
7. Remove duplicate files

**Expected**: Single source of truth, no duplication
**Risk**: HIGH (major refactoring, requires careful testing)

**Tests**:
- All existing tests still pass
- Protocol handler interface works for both proxies
- Factory creates correct handler for each protocol
- Feature matrix tests (all protocol + upstream combinations)

---

**Phase 3 Result**: Clean architecture, no code duplication, maximum reuse

---

## Implementation Order

### Week 1-2: P1-1 Connection Pool (Primary)
- Enable existing pool code
- Benchmark throughput improvement
- Target: 0.90 → 3-5 MB/s

### Week 1-2: P1-2, P1-3 (Parallel with P1-1)
- Enable DNS cache
- Reduce logging
- Target: Additional +5-10% performance

### Week 3-4: Review & Benchmark P1 Results
- Verify pooling effectiveness
- Profile bottlenecks
- Prepare for P2

### Week 5-8: P2-1 HTTP/2 Support for Proxy (Primary)
- Implement HTTP/2 server in proxy
- Integrate ALPN negotiation
- Test multiplexing

### Week 5-7: P2-2 Upstream for MITM (Parallel)
- Add upstream configuration
- Implement forwarding
- Test CONNECT with upstream

### Week 9-12: P3 Consolidation (Sequential)
- Unify protocol handlers
- Remove duplication
- Comprehensive testing

### Week 13+: Polish & Documentation
- Performance optimization
- Documentation updates
- Real-world testing

---

## Success Metrics

### Performance
- **Week 2**: 3-5 MB/s (via connection pooling)
- **Week 8**: 5-15 MB/s (via HTTP/2 multiplexing)
- **Week 12**: Same throughput for proxy and MITM

### Quality
- **All existing tests pass** throughout
- **New tests for** connection pooling, DNS cache, HTTP/2, upstream forwarding
- **Feature matrix tests**: all combinations work

### Code
- **scred-http**: Single source of truth for each protocol
- **scred-proxy**: Uses handlers from scred-http, no duplicate logic
- **scred-mitm**: Uses handlers from scred-http, no duplicate logic
- **Duplication**: 0 (measured in identical lines)

### Architecture
- **Isomorphism**: Both proxies have identical protocol support
- **Maintainability**: Bug fix in one place helps both
- **Extensibility**: New protocols added once in scred-http

---

## Risk Mitigation

### Connection Pooling (P1-1): LOW RISK
- Code already exists and tested
- Mitigation: Thorough benchmark before commit
- Rollback: Simple config disable

### HTTP/2 Support (P2-1): MEDIUM RISK
- Complex stream state management
- Mitigation: Extensive testing with concurrent streams
- Rollback: Run without HTTP/2 (ALPN fallback)

### Upstream for MITM (P2-2): MEDIUM RISK
- CONNECT with upstream is tricky
- Mitigation: Test proxied HTTPS scenarios thoroughly
- Rollback: Disable upstream in MITM config

### Code Consolidation (P3): HIGH RISK
- Major refactoring, large change surface
- Mitigation: Incremental refactoring, extensive testing
- Rollback: Have branch point before P3 starts

---

## Checkpoints

### After P1 (Week 4)
- [ ] Connection pooling working, throughput measured
- [ ] DNS cache enabled and tested
- [ ] Logging reduced
- [ ] Throughput: 3-5 MB/s confirmed

### After P2-1 (Week 8)
- [ ] HTTP/2 server working in proxy
- [ ] Multiplexing tested (5+ concurrent streams)
- [ ] Throughput: 5-15 MB/s with multiplexing

### After P2-2 (Week 8)
- [ ] Upstream proxy in MITM
- [ ] CONNECT tunneling working
- [ ] Feature parity achieved

### After P3 (Week 12)
- [ ] Protocol handlers unified
- [ ] No duplicate code
- [ ] All tests passing
- [ ] Documentation updated

---

## Success Criteria

### Functional
- ✓ scred-proxy supports HTTP/1.1, HTTP/2, upstream, pooling
- ✓ scred-mitm supports HTTP/1.1, HTTP/2, upstream, pooling
- ✓ Both can operate as forward or MITM (configuration)
- ✓ All protocol code in scred-http

### Performance
- ✓ Throughput: 3-5 MB/s (P1)
- ✓ Throughput: 5-15 MB/s (P2)
- ✓ No regression vs current
- ✓ Consistent across both proxies

### Quality
- ✓ Zero test failures
- ✓ New comprehensive test suite
- ✓ No code duplication
- ✓ Clean architecture

### Documentation
- ✓ ARCHITECTURE.md explains design
- ✓ README.md updated with features
- ✓ Code comments on complex sections
- ✓ Migration guide for users

