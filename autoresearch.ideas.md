# scred-proxy Optimization Ideas

## BOTTLENECK IDENTIFIED (Session 1)

**Root Cause**: DNS resolution + no connection pooling to upstream
- Location: `crates/scred-http/src/dns_resolver.rs`
- Issue 1: Fresh DNS lookup for every request (no caching)
- Issue 2: Exponential backoff delays (100ms, 200ms, 400ms) on failures
- Issue 3: No TCP connection pooling/Keep-Alive
- Evidence: Concurrent requests 53% SLOWER than sequential

## Potential Optimizations (Ranked by Impact)

### 1. Reduce Logging Overhead ❌ TRIED - MADE WORSE
- **Status**: DISCARD (Run #12)
- **Test**: RUST_LOG=off - disabled all logging
- **Result**: SLOWER (0.013 vs 0.017 MB/s, -23%)
- **Conclusion**: Logging is NOT the bottleneck
- **Reason discard**: Optimization made performance worse, suggests logging has minimal overhead

### 2. Connection Pooling to Upstream ⭐ HIGH PRIORITY
- **Current**: New TCP connection per request + DNS resolution
- **Problem**: DNS resolver has exponential backoff on failure (100ms, 200ms, 400ms...)
- **Solution**:
  - Reuse TCP connections with Keep-Alive
  - Pool connections with tokio
  - Cache DNS results per upstream
  - Concurrent DNS lookup
- **Expected gain**: 3-5x throughput

### 3. Tokio Runtime Optimization
- **Current**: tokio runtime likely using default settings
- **Solution**:
  - Tune worker threads
  - Enable async I/O properly
  - Check for blocking operations
  - Use tokio::spawn_blocking for I/O
- **Expected gain**: 1-2x

### 4. Buffer Management
- **Current**: Read/write buffers may not be sized optimally
- **Solution**:
  - Increase BufReader capacity (default 8KB)
  - Use custom pooled buffers
  - Avoid multiple allocations per request
- **Expected gain**: 1-1.5x

### 5. Pattern Matching Optimization
- **Current**: May be doing unnecessary pattern checks per request
- **Solution**:
  - Cache compiled patterns
  - Skip pattern matching for "Passthrough" mode
  - Pre-filter by content-type
- **Expected gain**: 0.5-1x

### 6. Concurrent Request Handling ⚠️ REVEALS BOTTLENECK
- **Status**: TESTED (Run #13)
- **Test**: 20 parallel concurrent requests
- **Result**: MUCH SLOWER (0.008 vs 0.017 MB/s, -53%)
- **Key Finding**: Proxy gets SLOWER with concurrent requests, not faster
- **Root Cause Identified**: 
  - No connection pooling/Keep-Alive to upstream
  - No DNS caching (fresh DNS lookup per request)
  - DNS resolver has 100ms+ backoff delays
  - Likely resource contention or blocking I/O
- **Next Step**: Implement DNS caching + connection pooling

### 7. Remove Unnecessary Allocations
- **Current**: String parsing, header cloning, pattern compilation
- **Solution**:
  - Use string views/slices instead of clones
  - Avoid pattern recompilation
  - Use cow::Cow for conditional allocation
- **Expected gain**: 0.5-1x

## Testing Strategy (No Cheating!)
1. ✅ Baseline: Sequential requests (done)
2. Concurrent requests with multiple connections
3. Measure with different response sizes (100 bytes, 500 bytes, 1KB, 10KB)
4. Measure with/without redaction enabled
5. Measure with varying log levels
6. Profile with flame graphs to identify real bottlenecks

## Non-Solutions (Won't Try)
- Removing features/redaction (violates "no cheating")
- Skipping pattern detection (violates "no cheating")
- Hardcoding test responses (violates "no cheating")
- Single-threaded mode (defeats purpose of tokio)
