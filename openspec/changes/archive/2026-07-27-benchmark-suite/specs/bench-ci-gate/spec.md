# Benchmark CI Gate

## Task Reference

| Task ID | Description |
|---------|-------------|
| T6.1 | Run all benchmarks and save baseline to `.benchmark-baseline.json` |
| T6.2 | Update `just bench-ci` to compare against baseline with 5% threshold |
| T6.3 | Add `--quick` mode support for faster CI runs |
| T6.4 | Verify `just bench-ci` passes with no regressions |
| T6.5 | Verify `just bench-ci` fails when a regression is introduced |

## ADDED Requirements

### Requirement: Store benchmark baseline

T6.1 SHALL complete BEFORE T6.2.

#### Scenario: First run creates baseline

- **WHEN** T6.1 runs with no existing baseline
- **THEN** it SHALL save results to `.benchmark-baseline.json`

### Requirement: Detect regressions

T6.2 SHALL complete BEFORE T6.4.

#### Scenario: Regression detected

- **WHEN** T6.5 introduces a 5% regression
- **THEN** `just bench-ci` SHALL exit with code 1

#### Scenario: No regression

- **WHEN** T6.4 runs with no regressions
- **THEN** `just bench-ci` SHALL exit with code 0

### Requirement: Support quick CI mode

T6.3 SHALL complete BEFORE T6.4.

#### Scenario: Quick mode

- **WHEN** T6.3 runs with `--quick` flag
- **THEN** it SHALL use reduced warmup (1s instead of 3s) and fewer samples
