# CLI Streaming Benchmark

## Task Reference

| Task ID | Description |
|---------|-------------|
| T3.1 | Add `criterion` dev-dependency to `crates/scred-cli/Cargo.toml` |
| T3.2 | Create `crates/scred-cli/benches/streaming.rs` with Criterion harness |
| T3.3 | Implement benchmark for `stream_and_redact()` with piped stdin data |
| T3.4 | Implement benchmark with varying pattern densities |
| T3.5 | Verify benchmarks compile and produce meaningful results |

## ADDED Requirements

### Requirement: Measure end-to-end CLI streaming throughput

T3.1 SHALL complete BEFORE T3.2. T3.2 SHALL complete BEFORE T3.3. T3.3 SHALL complete BEFORE T3.5.

#### Scenario: Small input throughput

- **WHEN** T3.3 runs with 1MB of realistic mixed data piped through stdin
- **THEN** it SHALL report end-to-end throughput in MB/s

#### Scenario: Large input throughput

- **WHEN** T3.3 runs with 10MB of realistic mixed data piped through stdin
- **THEN** it SHALL report end-to-end throughput in MB/s

### Requirement: Measure peak memory usage

T3.3 SHALL complete BEFORE T3.5.

#### Scenario: Memory usage with large input

- **WHEN** T3.3 processes 10MB of data
- **THEN** it SHALL report peak RSS memory in MB

### Requirement: Measure throughput with varying pattern density

T3.4 SHALL complete BEFORE T3.5.

#### Scenario: No secrets throughput

- **WHEN** T3.4 runs with data containing zero secrets
- **THEN** it SHALL report throughput as a baseline

#### Scenario: Sparse secrets throughput

- **WHEN** T3.4 runs with data containing 1 secret per KB
- **THEN** it SHALL report throughput

#### Scenario: Dense secrets throughput

- **WHEN** T3.4 runs with data containing 10 secrets per KB
- **THEN** it SHALL report throughput
