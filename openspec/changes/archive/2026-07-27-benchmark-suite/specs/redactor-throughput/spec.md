# Redactor Throughput Benchmark

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Evaluate 9 orphaned bench files — compile each, remove broken ones |
| T1.2 | Add `[[bench]]` entries to Cargo.toml for viable bench files |
| T1.3 | Verify `cargo bench -p scred-redactor` runs and produces results |
| T2.1 | Create `crates/scred-redactor/benches/throughput.rs` with Criterion harness |
| T2.2 | Implement `build_realistic_data()` with varying pattern densities |
| T2.3 | Implement benchmark for `redact_reader_to_writer` with 1KB, 64KB, 1MB chunks |
| T2.4 | Implement benchmark for `process_chunk` with cross-boundary secrets |
| T2.5 | Verify benchmarks compile and produce meaningful results |

## ADDED Requirements

### Requirement: Wire up orphaned bench files

T1.1 SHALL complete BEFORE T1.2. T1.2 SHALL complete BEFORE T1.3.

#### Scenario: Orphaned bench evaluation

- **WHEN** T1.1 evaluates each orphaned bench file
- **THEN** each file SHALL either be wired up (if it compiles) or removed (if it doesn't)

#### Scenario: Bench entries added

- **WHEN** T1.2 adds `[[bench]]` entries
- **THEN** `cargo bench -p scred-redactor` SHALL list the wired-up benchmarks

### Requirement: Measure streaming redaction throughput

T2.1 SHALL complete BEFORE T2.2. T2.2 SHALL complete BEFORE T2.3. T2.3 SHALL complete BEFORE T2.5.

#### Scenario: Small chunk throughput

- **WHEN** T2.3 runs with 1KB chunks and realistic mixed data
- **THEN** it SHALL report throughput in MB/s with statistical significance

#### Scenario: Medium chunk throughput

- **WHEN** T2.3 runs with 64KB chunks and realistic mixed data
- **THEN** it SHALL report throughput in MB/s

#### Scenario: Large chunk throughput

- **WHEN** T2.3 runs with 1MB chunks and realistic mixed data
- **THEN** it SHALL report throughput in MB/s

### Requirement: Measure pattern density impact

T2.2 SHALL complete BEFORE T2.3. T2.3 SHALL complete BEFORE T2.5.

#### Scenario: No secrets throughput

- **WHEN** T2.3 runs with data containing zero secrets
- **THEN** it SHALL report throughput as a baseline

#### Scenario: Sparse secrets throughput

- **WHEN** T2.3 runs with data containing 1 secret per KB
- **THEN** it SHALL report throughput

#### Scenario: Dense secrets throughput

- **WHEN** T2.3 runs with data containing 10 secrets per KB
- **THEN** it SHALL report throughput

### Requirement: Measure lookahead efficiency

T2.4 SHALL complete BEFORE T2.5.

#### Scenario: Cross-boundary secret

- **WHEN** T2.4 runs with secrets that span chunk boundaries
- **THEN** it SHALL report throughput compared to aligned secrets
