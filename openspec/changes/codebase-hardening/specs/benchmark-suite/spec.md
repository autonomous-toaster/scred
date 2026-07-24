## Task Reference

| Task ID | Description |
|---------|-------------|
| T2.1 | Rewrite scaling benchmark — data outside b.iter(), cover all 5 detection tiers |
| T2.2 | Rewrite realistic benchmark — data outside b.iter(), cover all pattern types |
| T2.3 | Add RedactionStream benchmark |
| T2.4 | Remove broken pattern_frequency and empty simd_benchmark |
| T2.5 | Add CI benchmark gate to Justfile |

## ADDED Requirements

### Requirement: Scaling benchmark covers all tiers

T2.1 SHALL complete BEFORE T2.2 SHALL run.

#### Scenario: Data built outside b.iter()
- **WHEN** T2.1 runs
- **THEN** the benchmark data SHALL be constructed once before the `b.iter()` closure

#### Scenario: All 5 detection tiers tested
- **WHEN** T2.1 runs
- **THEN** the benchmark SHALL include patterns from simple_prefix, prefix_validation, JWT, multiline_markers, and URI patterns

### Requirement: Realistic benchmark covers all pattern types

T2.2 SHALL complete BEFORE T2.3 SHALL run.

#### Scenario: Realistic data includes all pattern categories
- **WHEN** T2.2 runs
- **THEN** the realistic benchmark SHALL include AWS keys, GitHub tokens, OpenAI keys, JWTs, SSH keys, database URIs, and webhook URLs

### Requirement: RedactionStream benchmark exists

T2.3 SHALL complete BEFORE T2.4 SHALL run.

#### Scenario: Streaming redaction measured
- **WHEN** T2.3 runs
- **THEN** the benchmark SHALL measure `RedactionStream::feed()` + `finalize()` throughput

### Requirement: Dead benchmarks removed

T2.4 SHALL complete AFTER T2.3 SHALL complete.

#### Scenario: pattern_frequency.rs removed
- **WHEN** T2.4 runs
- **THEN** `benches/pattern_frequency.rs` SHALL be deleted

#### Scenario: simd_benchmark.rs removed
- **WHEN** T2.4 runs
- **THEN** `benches/simd_benchmark.rs` SHALL be deleted

### Requirement: CI benchmark gate exists

T2.5 SHALL complete AFTER T2.3 SHALL complete AND T2.4 SHALL complete.

#### Scenario: just bench-ci runs and compares
- **WHEN** T2.5 runs
- **THEN** `just bench-ci` SHALL run benchmarks and compare against a stored baseline

### Requirement: Benchmark regression threshold

T2.6 SHALL complete AFTER T2.5 SHALL complete.

#### Scenario: Regression threshold defined
- **WHEN** T2.6 runs
- **THEN** the CI benchmark gate SHALL fail if throughput drops by more than 5% compared to the stored baseline
