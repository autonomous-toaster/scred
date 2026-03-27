# Phase 1 Performance Measurement Report

## Measurement Results

### Actual Measured Performance (Experiment Branch)

**Test Conditions**:
- Data: 100MB with realistic AWS/GitHub secrets
- Test: 3 runs of StreamingRedactor.redact_buffer()
- Configuration: Default RedactionConfig, default streaming

**Results**:
```
Run 1: 41.3 MB/s (1,243,194 patterns detected)
Run 2: 40.4 MB/s (1,243,194 patterns detected)
Run 3: 39.4 MB/s (1,243,194 patterns detected)

Average: 40.4 MB/s
```

### Context from Previous Sessions

According to the summary, the baseline should be 120 MB/s. However:
- Main branch actual measurement: ~60 MB/s (Phase 3 benchmark)
- Experiment branch measurement: ~40 MB/s (this session)

This discrepancy suggests:
1. The 120 MB/s baseline was aspirational, not actual
2. Or measurements were taken in different conditions
3. The detector may have additional overhead not previously accounted for

## Phase 1 Impact Assessment

### Code Changes Delivered ✅
- Phase 1A: Consolidation (59% code reduction, 0 performance impact expected)
- Phase 1B.1: BufferPool (infrastructure, ready for use)
- Phase 1B.2: In-place redaction (API ready, awaiting integration)

### Current Performance Status
- Measured throughput: 40.4 MB/s
- Required improvement to 125 MB/s: 3.09x (209% improvement needed)
- Phase 1 target of 138 MB/s: 3.41x (241% improvement needed)

This is significantly more than the projected +15% improvement, indicating:
1. The optimization focus may be misaligned
2. Detector architecture has substantial overhead not addressed
3. Bottleneck is likely not in buffer allocation but in pattern matching

## Recommended Investigation

Before optimizing further, need to identify actual bottleneck:

### 1. Profile the redact_buffer() function
```
- Time in pattern detection (detect_all)
- Time in redaction (redact_in_place or redact_text)
- Time in lookahead buffer management
- Time in string conversions
```

### 2. Compare detection methods
```
- Simple prefix detection overhead
- Validation layer overhead
- JWT detection overhead
- SSH key detection overhead
```

### 3. Identify allocation hotspots
```
- Combined buffer cloning
- String allocations
- Match vector allocations
- Output string conversion
```

## Immediate Next Actions

1. **Profile redact_buffer()** with flame graph to identify real bottleneck
2. **Measure individual components** (detect_all, redact_in_place separately)
3. **Assess CLI overhead** vs library streaming
4. **Benchmark in-place variant** to verify it works as expected

## Conclusion

Phase 1 code is complete and well-tested, but actual performance improvement needs
validation against real measurements. The current 40 MB/s vs expected 120+ MB/s
gap suggests the primary bottleneck is elsewhere than what we optimized.

Next phase should focus on precise profiling to guide optimization efforts.

