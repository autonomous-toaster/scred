# MITM Latency Benchmark

## Task Reference

| Task ID | Description |
|---------|-------------|
| T5.1 | Add `criterion` dev-dependency to `crates/scred-mitm/Cargo.toml` |
| T5.2 | Create `crates/scred-mitm/benches/latency.rs` with Criterion harness |
| T5.3 | Implement benchmark for `handle_single_request` with mocked TLS stream |
| T5.4 | Implement benchmark for `handle_h2_downgrade` with H2 preface |
| T5.5 | Implement benchmark for `CertificateGenerator::get_or_generate_cert` |
| T5.6 | Verify benchmarks compile and produce meaningful results |

## ADDED Requirements

### Requirement: Measure TLS handshake overhead

T5.1 SHALL complete BEFORE T5.2. T5.2 SHALL complete BEFORE T5.3. T5.3 SHALL complete BEFORE T5.6.

#### Scenario: TLS handshake with cache hit

- **WHEN** T5.3 runs with a cached certificate for the domain
- **THEN** it SHALL report handshake time in microseconds

#### Scenario: TLS handshake with cache miss

- **WHEN** T5.3 runs with a new domain (certificate must be generated)
- **THEN** it SHALL report handshake time in microseconds

### Requirement: Measure request forwarding latency

T5.3 SHALL complete BEFORE T5.6. T5.4 SHALL complete BEFORE T5.6.

#### Scenario: HTTP/1.1 request latency

- **WHEN** T5.3 runs `handle_single_request` with an HTTP/1.1 request and mocked upstream
- **THEN** it SHALL report end-to-end latency in microseconds

#### Scenario: H2 downgrade latency

- **WHEN** T5.4 runs with an H2 preface (triggering `handle_h2_downgrade`)
- **THEN** it SHALL report downgrade overhead in microseconds

### Requirement: Measure certificate generation cost

T5.5 SHALL complete BEFORE T5.6.

#### Scenario: Certificate generation time

- **WHEN** T5.5 generates a new certificate for a domain
- **THEN** it SHALL report generation time in milliseconds
