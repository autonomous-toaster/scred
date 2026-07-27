## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Fix Justfile check-file-sizes path (sift/src → crates/) |
| T1.2 | Add bench recipe to Justfile |
| T1.3 | Add --all-features to check recipe |
| T1.4 | Add workspace lints to Cargo.toml |
| T1.5 | Fix .expect()/.unwrap() in production code |
| T1.6 | Remove REGEX_PATTERN_COUNT dead code |
| T1.7 | Consolidate pattern selector parsing in CLI |

## ADDED Requirements

### Requirement: Justfile paths are correct

T1.1 SHALL complete BEFORE T1.2 SHALL run.

#### Scenario: check-file-sizes finds source files
- **WHEN** T1.1 runs
- **THEN** `just check-file-sizes` SHALL search `crates/` for .rs files, not `sift/src` or `sift-core/src`

### Requirement: Bench recipe exists

T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: just bench runs benchmarks
- **WHEN** T1.2 runs
- **THEN** `just bench` SHALL run `cargo bench --workspace`

### Requirement: Check uses all-features

T1.3 SHALL complete BEFORE T1.4 SHALL run.

#### Scenario: just check compiles all feature gates
- **WHEN** T1.3 runs
- **THEN** `just check` SHALL pass `--all-features` to cargo check

### Requirement: Workspace lints enforce safety

T1.4 SHALL complete BEFORE T1.5 SHALL run.

#### Scenario: unwrap_used is denied
- **WHEN** T1.4 runs
- **THEN** `Cargo.toml` SHALL contain `unwrap_used = "deny"` in `[workspace.lints.clippy]`

#### Scenario: expect_used is denied
- **WHEN** T1.4 runs
- **THEN** `Cargo.toml` SHALL contain `expect_used = "deny"` in `[workspace.lints.clippy]`

#### Scenario: unsafe_code is forbidden
- **WHEN** T1.4 runs
- **THEN** `Cargo.toml` SHALL contain `unsafe_code = "forbid"` in `[workspace.lints.rust]`

### Requirement: No .expect() or .unwrap() in production code

T1.5 SHALL complete AFTER T1.4 SHALL complete.

#### Scenario: AhoCorasick errors propagate
- **WHEN** T1.5 runs
- **THEN** `detector.rs` SHALL NOT contain `.expect()` or `.unwrap()` calls

#### Scenario: URI pattern errors propagate
- **WHEN** T1.5 runs
- **THEN** `uri_patterns.rs` SHALL NOT contain `.unwrap()` calls

### Requirement: Dead code removed

T1.6 SHALL complete AFTER T1.5 SHALL complete.

#### Scenario: REGEX_PATTERN_COUNT removed
- **WHEN** T1.6 runs
- **THEN** `patterns.rs` SHALL NOT contain `REGEX_PATTERN_COUNT`

#### Scenario: Pattern counts computed from arrays
- **WHEN** T1.6 runs
- **THEN** `TOTAL_PATTERNS` SHALL be computed as `.len()` on each pattern array

### Requirement: CLI uses library PatternSelector

T1.7 SHALL complete AFTER T1.5 SHALL complete AND T1.6 SHALL complete.

#### Scenario: CLI calls PatternSelector::from_str
- **WHEN** T1.7 runs
- **THEN** `scred-cli/src/main.rs` SHALL use `PatternSelector::from_str()` instead of custom parsing

### Requirement: Justfile has fmt check

T1.8 SHALL complete AFTER T1.7 SHALL complete.

#### Scenario: just fmt checks formatting
- **WHEN** T1.8 runs
- **THEN** `just fmt` SHALL run `cargo fmt --check`
