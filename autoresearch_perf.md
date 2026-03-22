# Autoresearch: SCRED HTTP/2 Performance Profiling & Optimization

## Objective

Establish performance baseline and identify optimization opportunities in HTTP/2 upstream handling (Phase 2 completion).

**Goal**: Measure latency per scenario without breaking 458 tests, identify hot paths, create foundation for targeted optimizations.

**Workload**: Synthetic HTTP/2 requests through SCRED MITM proxy, measuring:
- Frame forwarding latency (Scenario 4)
- Header redaction overhead
- HPACK decode/encode latency
- End-to-end request latency

## Metrics

- **Primary**: `tests_passing` (higher is better)
  - Must maintain 458/458 tests
  - Zero regressions
  
- **Secondary**: 
  - `h2_frame_forwarding_us` — H2↔H2 frame forwarding latency (microseconds)
  - `hpack_decode_us` — HPACK header decompression (microseconds)
  - `redaction_overhead_pct` — Redaction overhead as % of total time

## How to Run

```bash
cd /Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred

# Baseline: Run tests and measure current latency
cargo test --lib --all 2>&1 | grep "test result:" | head -1 | sed 's/.* \([0-9]*\) passed.*/tests_passing=\1/'

# Future: Add benchmarking harness
# cargo bench --bench http2_performance 2>&1 | grep -E "h2_frame_forwarding|hpack_decode|redaction"
```

## Files in Scope

| File | Purpose | Changes Allowed |
|------|---------|-----------------|
| `crates/scred-http/src/h2/frame_forwarder.rs` | H2↔H2 frame forwarding | Add metrics/telemetry only |
| `crates/scred-http/src/h2/hpack.rs` | HPACK encode/decode | Profile, optimize if safe |
| `crates/scred-http/src/h2/per_stream_redactor.rs` | Per-stream redaction | Profile, optimize if safe |
| `crates/scred-mitm/src/mitm/tls_mitm.rs` | MITM routing | Add scenario metrics only |
| `crates/scred-http/src/h2/` | All H2 modules | Add benchmarks |
| Benchmarks directory | New benchmarks | Create `benches/http2_performance.rs` |

## Off Limits

- ❌ Do NOT modify redaction patterns (47 patterns must be exact)
- ❌ Do NOT skip per-stream redaction (security requirement)
- ❌ Do NOT break RFC 7540 compliance
- ❌ Do NOT add new dependencies for perf (use stable Rust only)
- ❌ Do NOT remove features (only optimize existing)

## Constraints

1. **Must maintain 458/458 tests passing** (zero regressions)
2. **All changes must be safe Rust** (no `unsafe` blocks)
3. **Benchmarks must be reliable** (not flaky)
4. **No external dependencies** (only std + already used crates)
5. **Production code quality** (clean, documented, maintainable)

## What's Been Tried

**Phase 2 Foundation Work**:
- ✅ Fixed http2_downgrade flag (removed dead code, -79 LOC)
- ✅ Implemented Scenario 3 proxy detection (added 30 LOC)
- ✅ All 4 scenarios now working correctly
- ✅ 458/458 tests passing

**Performance Observations** (from code review):
- Frame forwarding appears straightforward (minimal overhead)
- HPACK decoding likely more complex (dynamic table management)
- Redaction runs for every header (47 patterns per header)
- No obvious bottlenecks yet (need profiling)

**Dead Ends** (to avoid):
- ❌ Removing per-stream redaction (doesn't scale, need isolation)
- ❌ Caching headers (unsafe with MITM)
- ❌ Breaking RFC 7540 (interop issues)

## Optimization Strategy

**Phase 1: Baseline & Metrics** (This session)
1. ✅ Verify 458/458 tests still pass
2. Create benchmarking harness for H2 operations
3. Measure current latency:
   - Frame forwarding time
   - HPACK decode time
   - Redaction time
4. Identify top 3 bottlenecks

**Phase 2: Targeted Optimization** (Next session)
1. Profile identified bottlenecks
2. Implement safe optimizations:
   - Zero-copy where possible
   - Buffer pooling
   - Table caching
3. Measure improvement
4. Keep if >5% improvement, no regression

**Phase 3: Real-World Tuning** (Future)
1. Test with real workloads
2. Per-scenario tuning
3. Configuration options

## Key Components

### Frame Forwarder (Scenario 4)
- **File**: `frame_forwarder.rs` (624 LOC)
- **Job**: Bidirectional H2 frame forwarding
- **Potential**: Frame parsing, frame encoding, redaction calls
- **Target latency**: <5ms per frame (100+ fps)

### HPACK Decoder (All Scenarios)
- **File**: `hpack.rs` (400+ LOC)
- **Job**: Decompress H2 headers
- **Potential**: Dynamic table lookups, Huffman decode
- **Target latency**: <1ms for typical headers

### Per-Stream Redactor (All Scenarios)
- **File**: `per_stream_redactor.rs` (300+ LOC)
- **Job**: Redact sensitive headers per stream
- **Potential**: Pattern matching, regex evaluation
- **Target overhead**: <10% of total latency

## Success Criteria

✅ 458/458 tests passing (zero regressions)
✅ Benchmarking harness created
✅ Baseline metrics established
✅ Top 3 bottlenecks identified
✅ Code compiles without warnings
✅ Documentation updated

## Notes for Resuming Agents

- Phase 2 is **complete** - do not modify fix commits
- Current baseline: **458/458 tests**
- Goal: **Performance measurement only** (no optimization yet)
- When ready: Move to performance optimization phase
- Ideas file: `autoresearch.ideas.md` has list of optimizations to try

