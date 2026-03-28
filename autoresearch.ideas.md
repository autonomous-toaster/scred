# scred-proxy Optimization Ideas

## Baseline Performance
- Current: 0.017 MB/s (34.3 RPS, 200 sequential requests)
- Response size: ~500 bytes
- Bottleneck: Logging overhead, sequential processing, DNS resolution per request

## Potential Optimizations

### 1. Reduce Logging Overhead ⭐ HIGH PRIORITY
- **Current**: Every connection logs multiple lines (connection, request, selector, location rewriting)
- **Problem**: Logging I/O expensive in loop
- **Solution**: 
  - Add RUST_LOG filtering (use error/warn only)
  - Batch logging statements
  - Use ring buffer for connection logs
  - Lazy formatting of log messages
- **Expected gain**: 2-3x throughput

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

### 6. Concurrent Request Handling
- **Current**: Likely sequential per-connection, but can have multiple simultaneous connections
- **Problem**: Test uses sequential curl, not representing real workload
- **Solution**:
  - Benchmark with concurrent connections (wrk, Apache Bench)
  - Verify tokio handles concurrent connections efficiently
- **Expected gain**: Reveals true bottleneck

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
