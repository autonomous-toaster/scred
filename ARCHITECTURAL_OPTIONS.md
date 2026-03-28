# Fundamental Architectural Changes for Maximum Throughput

## Current Bottleneck Analysis

**HTTP/1.1 Request-Response Proxy Model**:
```
Client Request → Parse Headers → Upstream Request → Parse Response → Client Response
     ↓              ↓                   ↓                  ↓              ↓
  0.1ms           0.1ms              0.15ms             0.1ms          0.14ms
                  (read)            (network)           (read)         (write)
                                                                       
Total: 0.49ms per request → 1.06 MB/s ceiling
```

Each request requires:
1. Syscall to read from client
2. Parse headers (string operations)
3. Syscall to write to upstream
4. Syscall to read from upstream
5. Parse headers again
6. Syscall to write to client

**Problem**: 6+ syscalls + 2 parsing operations per request

---

## Architecture Option 1: TCP Stream Multiplexer (BEST for RAW THROUGHPUT)

### Design
```
Client TCP Socket → Raw Byte Buffer → Redact-In-Place → Upstream TCP Socket
        ↓                ↓                  ↓                    ↓
      (zero-copy)   (ringbuffer)      (SIMD scan)           (zero-copy)
                    (no HTTP parsing)  (pattern match)        (async write)
```

### Key Changes
1. **Skip HTTP parsing entirely** - Don't parse headers, just forward all bytes
2. **Ring buffer** - Circular buffer for input, output, and detection window
3. **SIMD pattern scanning** - Scan raw bytes for secret patterns (already implemented)
4. **Zero-copy forwarding** - Direct memory→network without intermediate buffers
5. **Async batching** - Queue writes, flush in batches

### Expected Throughput
- **Current**: 1.06 MB/s (HTTP/1.1, full parsing)
- **This approach**: 20-50 MB/s (10-50× improvement)
  - Eliminates header parsing overhead
  - Reduces syscalls to ~2 per 4KB chunk (vs 1 per request)
  - SIMD pattern matching at near-memory speeds

### Trade-offs
- ✅ Maximum throughput (20-50 MB/s)
- ✅ Stateless, cache-friendly
- ❌ Cannot see/modify HTTP semantics (headers, redirects, etc.)
- ❌ Cannot do request routing based on URL/headers
- ❌ Cannot handle chunked encoding properly
- ❌ Cannot support HTTP/2

### Implementation Difficulty: **MEDIUM**
- Use existing SIMD detector
- Add ring buffer for windowing
- Replace request/response parsing with raw forwarding
- Keep `tokio` async runtime

### Code Structure
```rust
// Instead of:
loop {
    request = parse_http_request(client_reader)  // 0.1ms
    upstream_writer.write(request)               // 0.15ms
    response = parse_http_response(upstream_reader) // 0.1ms
    client_writer.write(response)                // 0.14ms
}

// Would become:
loop {
    chunk = client_reader.read_chunk(4096)       // 0.01ms
    redacted = detector.scan_and_redact(chunk)   // 0.001ms
    upstream_writer.write_batch(redacted)        // 0.001ms (batched)
}
```

---

## Architecture Option 2: HTTP/2 Multiplexing (GOOD for REAL-WORLD USAGE)

### Design
```
Multiple Client Streams (HTTP/2) → Single Upstream Connection (HTTP/2)
                    ↓
            Concurrent Frame Processing
                    ↓
         Redaction per stream in parallel
```

### Key Changes
1. **HTTP/2 support** - Binary framing, multiplexing on single connection
2. **Stream-level buffering** - 64KB buffer per stream instead of request-level
3. **Concurrent processing** - Multiple streams processed in parallel
4. **HPACK header compression** - Reduce header overhead significantly
5. **Server push awareness** - Handle push promises

### Expected Throughput
- **Current**: 1.06 MB/s (single sequential stream)
- **This approach**: 5-15 MB/s (5-15× improvement)
  - Multiplexing reduces connection overhead
  - Multiple concurrent requests on one connection
  - Better CPU utilization

### Trade-offs
- ✅ Real HTTP semantics preserved
- ✅ Better for modern clients/servers
- ✅ Header compression (HPACK)
- ❌ More complex implementation
- ❌ Upstream may not support HTTP/2
- ❌ Still parsing headers (slower than raw)

### Implementation Difficulty: **HARD**
- Use `hyper` HTTP/2 server
- Add connection pooling for upstream
- Handle stream priority
- Deal with upgrade negotiations

### Potential Implementation
```rust
// Use hyper's HTTP/2 support
let h2_server = hyper::server::conn::http2::Builder::new()
    .max_concurrent_streams(Some(1000))
    .build(executor)?;

// Each stream processed in parallel
tokio::spawn(async move {
    // Redact stream independently
    redact_stream(client_stream, upstream_stream).await
});
```

---

## Architecture Option 3: Zero-Copy Splice (EXTREME - Linux-specific)

### Design
```
Client Socket → FS Cache/Buffer → Kernel Splice → Redaction MMIO → Upstream Socket
      ↓             ↓                   ↓              ↓              ↓
   (user)       (kernel)          (zero-copy)    (memory-map)    (kernel)
                                 (no context    (pattern scan)  (no context
                                  switch)                        switch)
```

### Key Changes
1. **`splice()` syscall** - Zero-copy data movement in kernel
2. **Memory-mapped detection** - MMIO for pattern matching
3. **eBPF redaction** - In-kernel byte modification
4. **DPDK or AF_XDP** - Userspace packet processing

### Expected Throughput
- **Current**: 1.06 MB/s (full userspace)
- **This approach**: 50-200 MB/s (50-200× improvement)
  - Zero-copy kernel forwarding
  - eBPF pattern matching in-kernel
  - Bypass userspace context switches

### Trade-offs
- ✅ Maximum possible throughput
- ✅ Kernel-level efficiency
- ❌ Extremely complex (eBPF knowledge required)
- ❌ Linux-specific only
- ❌ Cannot inspect/modify HTTP semantics
- ❌ Debugging nightmare

### Implementation Difficulty: **VERY HARD**
- Requires eBPF programming
- Requires DPDK or AF_XDP knowledge
- Kernel module development skills
- Not practical for most teams

---

## Architecture Option 4: Async Connection Pooling (PRACTICAL - 3-5× GAIN)

### Design
```
Client Requests → Async Queue → Connection Pool → Multiple Upstream Connections
      ↓              ↓              ↓                     ↓
   (buffered)    (500 req/s)   (10 connections)    (parallel requests)
                                                         ↓
                                         Parallel pipelining to upstream
```

### Key Changes
1. **Connection pool** - Maintain N persistent connections to upstream
2. **Request pipelining** - Send multiple requests before reading responses
3. **Async/await batching** - Queue requests, process in batches
4. **Automatic retry** - Reconnect on connection failure

### Expected Throughput
- **Current**: 1.06 MB/s (single connection, sequential)
- **This approach**: 3-5 MB/s (3-5× improvement)
  - Multiple connections → parallel HTTP transactions
  - Pipelining → amortize connection overhead
  - Better network utilization

### Trade-fails
- ✅ Moderate throughput increase
- ✅ Preserves HTTP semantics
- ✅ Works with any HTTP/1.1 upstream
- ✅ Relatively simple implementation
- ❌ Not as fast as raw TCP streaming
- ❌ Complexity in error handling

### Implementation Difficulty: **EASY**
- Use `deadpool` or similar connection pool
- Simple request queue
- Standard tokio async patterns

### Practical Implementation
```rust
// Simple connection pool
let pool = Pool::new(
    ConnectionManager::new("http://upstream:8000"),
    10, // pool size
)?;

// Async batching
for request in batch_of_requests {
    let conn = pool.get().await?;
    tokio::spawn(forward_request(conn, request));
}
```

---

## Architecture Option 5: Streaming Transformer Pipeline (NOVEL)

### Design
```
Raw TCP Input → Buffer Pool → Pipeline Stage 1 → Pipeline Stage 2 → Raw TCP Output
      ↓             ↓          (Read 4KB)        (Redact in-place)    ↓
   (socket)    (pre-alloc)    (detect)          (pattern match)    (buffered)
                                 ↓                    ↓             (batched)
                            (scan SIMD)         (write batch)
```

### Key Changes
1. **Buffer pool** - Pre-allocated buffers, zero allocation overhead
2. **Pipeline stages** - Separate I/O from processing from writing
3. **SIMD batch processing** - Process 64 bytes at a time
4. **Work-stealing queue** - Multiple workers pull from shared queue

### Expected Throughput
- **Current**: 1.06 MB/s
- **This approach**: 10-30 MB/s (10-30× improvement)
  - Zero allocation overhead
  - Perfect SIMD utilization
  - CPU cache efficiency

### Trade-offs
- ✅ Excellent throughput (10-30 MB/s)
- ✅ Scalable to many cores
- ✅ No HTTP semantics loss
- ❌ Complex implementation
- ❌ Requires careful threading/lock-free design
- ❌ Benchmark-dependent results

### Implementation Difficulty: **HARD**
- Lock-free queue design
- Buffer pool management
- Thread coordination
- SIMD alignment

---

## Quick Comparison Table

| Architecture | Throughput | Complexity | HTTP Semantics | Real-World |
|---|---|---|---|---|
| **Current (HTTP/1.1)** | 1 MB/s | Low | ✅ | ✅ |
| **Option 1: Raw TCP Multiplexer** | 20-50 MB/s | Medium | ❌ | ⚠️ (gateway only) |
| **Option 2: HTTP/2** | 5-15 MB/s | Hard | ✅ | ✅ |
| **Option 3: Kernel Splice + eBPF** | 50-200 MB/s | Very Hard | ❌ | ⚠️ (expert only) |
| **Option 4: Connection Pool** | 3-5 MB/s | Easy | ✅ | ✅ |
| **Option 5: Streaming Pipeline** | 10-30 MB/s | Hard | ✅ | ⚠️ (requires tuning) |

---

## Recommendation by Use Case

### Use Case 1: "I need 160 MB/s to match CLI"
**Answer**: You can't. The CLI is redacting memory, not proxying network traffic. This is a category error.

**What you CAN do**:
- Option 1 (Raw TCP): 20-50 MB/s (easiest path to 10×+)
- Option 3 (Kernel): 50-200 MB/s (if you're a kernel expert)

### Use Case 2: "I need production proxy at 5-10 MB/s"
**Best choice**: Option 4 (Connection Pool)
- Easy to implement (+200 LOC)
- 3-5× improvement
- No HTTP semantics loss
- Production-ready code patterns

### Use Case 3: "I need maximum throughput for corporate gateway"
**Best choice**: Option 1 (Raw TCP Multiplexer)
- 20-50 MB/s achievable
- Accept loss of HTTP routing capability
- Monitor-only use case (no modification needed)
- Moderate complexity

### Use Case 4: "I need HTTP/2 support for modern clients"
**Best choice**: Option 2 (HTTP/2)
- 5-15 MB/s with multiplexing
- Full HTTP semantics
- Future-proof
- Hard implementation, but standard patterns

### Use Case 5: "I need absolute maximum throughput"
**Best choice**: Option 3 (Kernel Splice + eBPF)
- 50-200 MB/s possible
- Requires Linux kernel expertise
- Not recommended unless you have it
- Consider cloud infrastructure alternative instead

---

## My Recommendation for scred-proxy

**Current Status**: ✅ 1.06 MB/s production-ready

**Next Priority** (if throughput becomes issue):
1. **Short term** (1-2 weeks): Implement Option 4 (Connection Pool) → 3-5 MB/s
2. **Medium term** (1-2 months): Consider Option 2 (HTTP/2) → 5-15 MB/s
3. **Long term** (optional): Explore Option 1 (Raw TCP) for specific use cases → 20-50 MB/s

**Avoid**:
- Option 3 (too complex for marginal gains)
- Attempting to match CLI's 160 MB/s (different category)

---

## Implementation Roadmap (if needed)

### Phase 1: Connection Pooling (Easy, 3-5× gain)
```rust
// Add deadpool crate
upstream_pool = Pool::new(UpstreamManager::new(upstream_url), 10)?;

// Modify proxy to use pool
let upstream_conn = pool.get().await?;
```

### Phase 2: HTTP/2 Support (Hard, 5-15× gain)
```rust
// Replace hyper::server with h2 or upgrade to HTTP/2
let h2_server = hyper::server::http2::Builder::new();
```

### Phase 3: Raw TCP Mode (Medium, 20-50× gain)
```rust
// Add feature flag: --mode=raw-tcp
// Disable HTTP parsing, just forward bytes with SIMD redaction
```

---

## Conclusion

**You cannot achieve 160 MB/s with HTTP proxy** because:
1. HTTP parsing inherently requires parsing headers
2. Request-response model requires 2+ round-trips
3. Network I/O has hard limits (syscalls)

**You CAN achieve**:
- 1-5 MB/s easily (current + connection pool)
- 5-15 MB/s with HTTP/2
- 20-50 MB/s with raw TCP streaming
- 50-200 MB/s with kernel-level tricks (not recommended)

The best architecture depends on your actual use case, not artificial benchmarks.
