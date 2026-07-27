# pre-existing-fixes Specification

## Purpose
TBD - created by archiving change ci-green. Update Purpose after archive.
## Requirements
### Requirement: Compilation errors fixed before clippy

T1.1 SHALL complete BEFORE T1.2 SHALL run.
T1.2 SHALL complete BEFORE T1.3 SHALL run.
T1.3 SHALL complete BEFORE T2.1 SHALL run.

#### Scenario: cached_dns_resolver compiles
- **WHEN** T1.1 runs
- **THEN** `CachedDnsResolver` SHALL have a single `new()` method

#### Scenario: connection_pool compiles
- **WHEN** T1.3 runs
- **THEN** `ConnectionPool::new()` SHALL assign `_addr: addr`

### Requirement: Clippy warnings fixed before machete

T2.1 SHALL complete BEFORE T2.2 SHALL run.
T2.2 SHALL complete BEFORE T2.3 SHALL run.
T2.3 SHALL complete BEFORE T3.1 SHALL run.

#### Scenario: engine/mod.rs has no clippy warnings
- **WHEN** T2.1 runs
- **THEN** `cargo clippy -p scred-policy -- -Dwarnings` SHALL pass

### Requirement: Machete clean before verification

T3.1 SHALL complete BEFORE T3.2 SHALL run.
T3.2 SHALL complete BEFORE T5.1 SHALL run.

#### Scenario: no unused dependencies
- **WHEN** T3.1 runs
- **THEN** `cargo machete` SHALL NOT report unused dependencies for removed crates

#### Scenario: false positives ignored
- **WHEN** T3.2 runs
- **THEN** each Cargo.toml with false positives SHALL have a `[package.metadata.cargo-machete]` section

