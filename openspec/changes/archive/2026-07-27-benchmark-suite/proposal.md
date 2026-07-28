## Why

The codebase has 5 working Criterion benchmarks for the detector crate, 9 orphaned bench files for the redactor, and zero benchmarks for CLI streaming, proxy throughput, or MITM latency. Without benchmarks, performance regressions go undetected — a 15% slowdown in redaction throughput or a 20ms increase in TLS handshake latency could ship without anyone noticing. The `just bench-ci` command exists but has nothing meaningful to compare against.

## What Changes

- Wire up 9 orphaned redactor bench files into `Cargo.toml` and fix any compilation issues
- Add Criterion benchmarks for `StreamingRedactor::redact_reader_to_writer` and `process_chunk` throughput
- Add Criterion benchmark for CLI `stream_and_redact()` end-to-end with realistic stdin data
- Add Criterion benchmark for proxy `forward_simple` / `forward_with_policy` with mocked I/O
- Add Criterion benchmark for MITM `handle_single_request` with mocked TLS stream
- Update `just bench-ci` to use a stored baseline and detect regressions
- Remove orphaned bench files that are not worth keeping

## Capabilities

### New Capabilities
- `redactor-throughput`: Measure streaming redaction throughput with varying chunk sizes, pattern densities, and lookahead scenarios
- `cli-streaming-bench`: Measure end-to-end CLI redaction performance with realistic stdin data (no secrets, sparse, dense)
- `proxy-throughput-bench`: Measure proxy request forwarding throughput with mocked I/O, with and without policy engine
- `mitm-latency-bench`: Measure MITM TLS handshake overhead and request forwarding latency with mocked TLS streams
- `bench-ci-gate`: CI gate that compares benchmark results against a stored baseline and fails on regressions > 5%

### Modified Capabilities
- (none — no existing spec-level behavior changes)

## Impact

- `crates/scred-redactor/Cargo.toml`: Add `[[bench]]` entries for existing bench files
- `crates/scred-cli/Cargo.toml`: Add `criterion` dev-dependency and `[[bench]]` entries
- `crates/scred-proxy/Cargo.toml`: Add `criterion` dev-dependency and `[[bench]]` entries
- `crates/scred-mitm/Cargo.toml`: Add `criterion` dev-dependency and `[[bench]]` entries
- `Justfile`: Update `bench-ci` to use stored baseline
- `.benchmark-baseline.json`: New baseline file (git-tracked)
- May remove some orphaned redactor bench files that don't compile or are redundant
