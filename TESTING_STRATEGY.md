# SCRED HTTP/2 E2E Testing Strategy

## Current State Assessment

### Existing Test Coverage
- **10 Basic E2E Tests** (e2e_httpbin.rs): HTTP/1.1 + basic H2 ALPN
  - Basic requests, ALPN negotiation, secret redaction, keep-alive, POST, error handling, compression
  - All tests use curl through httpbin.org
  - Tests verify: connectivity, HTTP status, response format, secret absence

- **6 Unit Tests** (http2_integration.rs): Protocol detection
  - ALPN protocol parsing, protocol detection, frame parsing
  - No real H2 traffic testing

- **286 Unit Tests**: Redaction patterns, stream handling, flow control
  - 47 secret patterns covered
  - Frame parsing, header decompression, upstream pooling

### Critical Gaps for Production Readiness

#### 1. **Protocol Compliance** ❌
- No RFC 9113 (HTTP/2) section validation
- No RFC 7540 (frame format) strict compliance tests
- No RFC 7541 (HPACK) header encoding/decoding correctness
- **Risk**: Silent protocol violations, incompatible clients

#### 2. **HTTP/2 Native Performance** ❌
- No multiplexing stress tests (100+ concurrent streams)
- No stream priority/dependency testing
- No flow control edge cases (window exhaustion, RST_STREAM)
- No push promise handling
- **Risk**: Performance regressions, connection stalls

#### 3. **Redaction Correctness Under H2** ❌
- Existing tests only verify "secret not in response" at HTTP/1.1 level
- No per-stream redaction state isolation validation
- No large header frame redaction (headers can span multiple HEADERS frames)
- No test for redacting continuation frames
- **Risk**: Cross-stream secret leakage

#### 4. **Stability Under Load** ⚠️
- No connection leak detection
- No memory growth profiling
- No stress testing with mixed protocol clients
- No timeout handling for stalled streams
- **Risk**: Production crashes, resource exhaustion

#### 5. **Real-world Scenarios** ⚠️
- Only tests against httpbin.org
- No localhost test server (reproducibility, no network dependency)
- No mixed H1/H2 client connections
- No multiple upstream server failover
- **Risk**: Works on demo, fails in production

#### 6. **Fuzzing & Edge Cases** ❌
- No protocol fuzzer
- No malformed frame handling
- No interleaved frame type confusion
- No header compression bomb detection
- **Risk**: Security vulnerabilities, DoS attacks

---

## Proposed Test Suite Structure

### Tier 1: Protocol Compliance Tests (RFC Coverage)
```
tests/compliance/
  - rfc7540_framing.rs       # Frame boundaries, type validation
  - rfc9113_h2.rs            # Stream lifecycle, priority, flow control
  - rfc7541_hpack.rs         # Header encoding/decoding correctness
  - frame_format.rs          # Frame header validation, reserved bits
```

### Tier 2: Redaction Integrity Tests
```
tests/redaction/
  - stream_isolation.rs      # Per-stream state, no cross-leak
  - header_redaction.rs      # Large headers, continuation frames
  - body_redaction.rs        # Chunked bodies, streaming
  - pattern_validation.rs    # All 47 patterns, false positive check (Wikipedia)
```

### Tier 3: Performance & Load Tests
```
tests/load/
  - concurrent_streams.rs    # 100, 1000, 10000 streams
  - throughput.rs            # hey/h2load with metrics
  - connection_pooling.rs    # Upstream reuse validation
  - memory_profiling.rs      # Growth tracking
```

### Tier 4: Stability & Edge Cases
```
tests/stability/
  - flow_control.rs          # Window exhaustion, recovery
  - stream_cancellation.rs   # RST_STREAM, proper cleanup
  - timeout_handling.rs      # Stalled streams, timeout recovery
  - protocol_errors.rs       # Malformed frames, graceful exit
```

### Tier 5: Real-World Scenarios
```
tests/integration/
  - localhost_server.rs      # Local test server, reproducible
  - mixed_clients.rs         # H1 + H2 on same connection
  - upstream_failover.rs     # Multiple servers, retry logic
  - tls_sessions.rs          # Session resumption, cert pinning
```

### Tier 6: Fuzzing
```
fuzz/
  - frame_fuzzer.rs          # cargo-fuzz for frame mutations
  - payload_fuzzer.rs        # Header/body compression bombs
```

---

## Implementation Plan

### Phase A: Compliance + Redaction (Week 1-2)
1. Build custom H2 test client (using h2 crate) for fine-grained control
2. Implement RFC compliance test harness
3. Add per-stream isolation validation
4. Create localhost test server to remove httpbin.org dependency

### Phase B: Performance + Load (Week 2-3)
1. Integrate hey or custom load generator
2. Add metrics collection (streams/sec, latency p50/p99, memory)
3. Concurrent stream stress tests
4. Connection pooling validation

### Phase C: Stability + Real-World (Week 3-4)
1. Add flow control edge case tests
2. Mixed protocol client simulator
3. Upstream failover scenarios
4. TLS session handling

### Phase D: Fuzzing (Week 4)
1. Set up cargo-fuzz framework
2. Frame format fuzzer
3. Compression bomb detector
4. Malformed header handling

---

## Key Anti-Patterns to Avoid (No Overfitting!)

❌ **DO NOT**:
- Test only happy path → Include errors, timeouts, edge cases
- Mock upstream servers → Use real services or localhost
- Rely on response timing for success → Verify actual state
- Increase test count without coverage gaps → Each test must test something new
- Ignore false positives → Validate redaction doesn't break parsing
- Test only at peak performance → Test at edge limits

✅ **DO**:
- Test real HTTP/2 frame boundaries
- Verify per-stream state isolation with concurrent requests
- Measure actual resource usage (memory, connections)
- Include malformed input handling
- Document why each test matters (RFC section, risk, behavior)
- Run against multiple real servers (httpbin, nghttp.org, localhost)
- Validate that the product works, not just passes tests

---

## Metrics to Track

| Metric | Target | Threshold |
|--------|--------|-----------|
| RFC Compliance | 100% coverage | No untested sections |
| Redaction Accuracy | 100% (all 47 patterns) | No false negatives |
| Concurrent Streams | 10,000 without crash | <5% failure rate |
| Throughput (H2) | 10,000+ req/sec | >500 req/sec minimum |
| Memory (1h test) | <500 MB | Growth <100 MB/hour |
| Upstream Reuse | 10-100x fewer TCP | >90% connection reuse |
| False Positives | 0 on Wikipedia | Test corpus validated |

---

## Execution Strategy

1. **Build incrementally**: Start with Tier 1, validate, then Tier 2, etc.
2. **Automate everything**: CI pipeline must run all tiers
3. **Fail fast**: Any RFC violation → immediate alert
4. **Report clearly**: Separate report for compliance, performance, security
5. **Document decisions**: Why we test each thing, what could break
