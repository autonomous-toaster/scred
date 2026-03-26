# SCRED Performance Regression Testing Guide

## Overview

The `perf_regression_test.py` script provides automated performance regression detection for SCRED. It measures:

- **Detection latency** across multiple payload sizes
- **Pattern density effects** (sparse, medium, dense)
- **Scalar vs SIMD** implementation comparison
- **Throughput** (MB/s) for streaming redaction

## Quick Start

### Basic Run
```bash
python3 perf_regression_test.py
```

### Save Baseline
```bash
python3 perf_regression_test.py --save-results
# Creates: perf_baseline.json
```

### Compare Against Baseline
```bash
python3 perf_regression_test.py --compare-baseline --save-results
```

## Test Coverage

### Payload Sizes
- 10KB (small, typical single request)
- 100KB (medium, batch processing)
- 1MB (large, bulk redaction)

### Pattern Densities
- **Sparse**: 1 secret per 100KB (realistic web traffic)
- **Medium**: 1 secret per 10KB (moderate secret density)
- **Dense**: Multiple secrets per KB (worst case)

### Implementations
- **Scalar**: Pure Rust, maximum compatibility
- **SIMD**: x86_64 SSE2 / ARM64 NEON (nightly required)

## Expected Results

### Performance Targets

| Test | Size | Scalar | SIMD | Delta |
|------|------|--------|------|-------|
| sparse_10kb | 10KB | ~0.5ms | ~0.5ms | -0% |
| sparse_100kb | 100KB | ~5ms | ~5ms | -0% |
| sparse_1mb | 1MB | ~50ms | ~50ms | -0.5% |
| medium_10kb | 10KB | ~0.6ms | ~0.6ms | -0.5% |
| medium_100kb | 100KB | ~6ms | ~6ms | -0.5% |
| medium_1mb | 1MB | ~60ms | ~59ms | -1% |
| dense_10kb | 10KB | ~1.5ms | ~1.4ms | -5% |
| dense_100kb | 100KB | ~15ms | ~14ms | -8% |

**Note**: SIMD benefit increases with density (more charset scanning)

### Regression Thresholds

🟢 **Good**: SIMD is 0-2% faster (expected)  
🟡 **Warning**: SIMD is 0-5% slower (noise, acceptable)  
🔴 **Alert**: SIMD is >5% slower (possible regression)

## Output Format

```
================================================================================
SCRED Performance Regression Test Results
================================================================================

sparse_10kb:
--------------------------------------------------------------------------------
Impl           Size    Patterns       Latency      Throughput
--------------------------------------------------------------------------------
scalar      10.0KB          2          0.48ms       20.87MB/s
simd        10.0KB          2          0.47ms       21.49MB/s

...

================================================================================
Summary Statistics
================================================================================
Scalar: avg latency 6.23ms
SIMD:   avg latency 6.18ms
SIMD Improvement: +0.8%
```

## Baseline Management

### Create Initial Baseline
```bash
# Run tests and save results
python3 perf_regression_test.py --save-results
# Results saved to: perf_baseline.json

# Commit baseline
git add perf_baseline.json
git commit -m "perf: Establish SIMD v0.2.0 baseline"
```

### Compare Current vs Baseline
```bash
# Run tests
python3 perf_regression_test.py --save-results --compare-baseline

# Compare with baseline in CI
python3 << 'EOF'
import json

with open('perf_baseline.json') as f:
    baseline = json.load(f)

# Your comparison logic here
# Flag if regression > 5%
EOF
```

## Integration with CI/CD

### GitHub Actions Example
```yaml
- name: Performance Regression Test
  run: |
    python3 perf_regression_test.py --save-results
    
- name: Compare Against Baseline
  run: |
    if [ -f perf_baseline.json ]; then
      # Add comparison logic
      echo "Comparing against baseline..."
    fi
```

## Troubleshooting

### Build Failures
```bash
# Ensure releases are built
cargo build --release
cargo +nightly build --release --features simd-accel

# Check paths
ls -la target/release/scred
ls -la target_nightly/release/scred
```

### Missing Binary
```bash
# Build missing implementation
cargo build --release --bin scred
# OR
cargo +nightly build --release --features simd-accel --bin scred
```

### Timeout Errors
Increase timeout in the script (line ~100):
```python
timeout=10  # Increase from 5 to 10 seconds
```

## Custom Test Cases

Extend the test suite by modifying test_cases:

```python
test_cases = [
    # (name, size_kb, density_generator)
    ("my_test", 50, MyPayloadGenerator.custom),
]
```

Example custom generator:
```python
class MyPayloadGenerator:
    @staticmethod
    def custom(size_kb: int) -> str:
        # Your payload generation logic
        return payload
```

## Production Monitoring

### Key Metrics to Track

1. **Latency Variance**: Should be <3% across runs
2. **SIMD Consistency**: Should outperform scalar by 0-2%
3. **Throughput**: Should be >10MB/s for all sizes

### Alert Conditions

```python
# Alert if SIMD slower than scalar
if simd_avg > scalar_avg * 1.05:
    alert("SIMD regression detected!")

# Alert if latency increases
if current_latency > baseline_latency * 1.10:
    alert("Performance regression!")
```

## Related Documentation

- [SIMD_IMPLEMENTATION.md](SIMD_IMPLEMENTATION.md) - Technical deep dive
- [SIMD_DEPLOYMENT.md](SIMD_DEPLOYMENT.md) - Production deployment
- [RELEASE_NOTES_v0.2.0.md](RELEASE_NOTES_v0.2.0.md) - Release information

## FAQ

**Q: Why is SIMD sometimes slower?**
A: Noise from system variations is normal at <2% differences. Run multiple times or increase payload size for clearer results.

**Q: What payload size should I use?**
A: For production, use 100KB-1MB (typical bulk redaction jobs). Smaller sizes are noisier.

**Q: How often should I run this?**
A: Before each release, and periodically in production (weekly/monthly) to catch regressions.

**Q: Can I use this on CI/CD?**
A: Yes, modify the script to integrate with your CI system and store baseline results.

**Q: What if scalar is slower than SIMD?**
A: That's unexpected but possible on some CPUs. Scalar should be baseline. Check compiler flags.
