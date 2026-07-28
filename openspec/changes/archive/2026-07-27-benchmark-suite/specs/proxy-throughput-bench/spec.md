# Proxy Throughput Benchmark

## Task Reference

| Task ID | Description |
|---------|-------------|
| T4.1 | Add `criterion` dev-dependency to `crates/scred-proxy/Cargo.toml` |
| T4.2 | Create `crates/scred-proxy/benches/throughput.rs` with Criterion harness |
| T4.3 | Implement benchmark for `forward_simple` with mocked I/O |
| T4.4 | Implement benchmark for `forward_with_policy` with disabled policy engine |
| T4.5 | Implement benchmark with varying body sizes (1KB, 100KB, 1MB) |
| T4.6 | Verify benchmarks compile and produce meaningful results |

## ADDED Requirements

### Requirement: Measure proxy request forwarding throughput

T4.1 SHALL complete BEFORE T4.2. T4.2 SHALL complete BEFORE T4.3. T4.3 SHALL complete BEFORE T4.6.

#### Scenario: Simple forwarding throughput

- **WHEN** T4.3 runs `forward_simple` with 1000 HTTP requests and mocked upstream
- **THEN** it SHALL report throughput in requests per second

#### Scenario: Policy forwarding throughput

- **WHEN** T4.4 runs `forward_with_policy` with a disabled policy engine and 1000 HTTP requests
- **THEN** it SHALL report throughput in requests per second

### Requirement: Measure latency percentiles

T4.3 SHALL complete BEFORE T4.6. T4.4 SHALL complete BEFORE T4.6.

#### Scenario: Latency distribution

- **WHEN** T4.3 runs 1000 requests
- **THEN** it SHALL report p50, p95, and p99 latency in microseconds

### Requirement: Measure body size impact

T4.5 SHALL complete BEFORE T4.6.

#### Scenario: Small body throughput

- **WHEN** T4.5 runs with 1KB response bodies
- **THEN** it SHALL report throughput in requests per second

#### Scenario: Large body throughput

- **WHEN** T4.5 runs with 1MB response bodies
- **THEN** it SHALL report throughput in requests per second
