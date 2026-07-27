## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Remove imports for non-existent modules |
| T1.2 | Delete backup files |
| T1.3 | Comment out broken functions temporarily |
| T1.4 | Verify what compiles and what doesn't |
| T2.1 | Verify h2 crate version in Cargo.toml |
| T2.2 | Add h2 and http imports to tls_mitm.rs |
| T2.3 | Implement h2::server handshake for client connections |
| T2.4 | Implement h2::client handshake for upstream connections |
| T2.5 | Test basic H2 connectivity (no redaction yet) |
| T3.1 | Implement request interception from h2::server |
| T3.2 | Implement response sending via h2::server |
| T3.3 | Implement request forwarding via h2::client |
| T3.4 | Implement response receiving via h2::client |
| T3.5 | Test end-to-end request/response flow |
| T4.1 | Integrate redaction engine at request level |
| T4.2 | Integrate redaction engine at response level |
| T4.3 | Test redaction with various patterns |
| T4.4 | Verify redaction doesn't break H2 protocol |
| T5.1 | Integrate policy engine for requests |
| T5.2 | Integrate policy engine for responses |
| T5.3 | Test policy enforcement |
| T5.4 | Handle policy errors gracefully |
| T6.1 | Handle H2 protocol errors (GOAWAY, RST_STREAM) |
| T6.2 | Handle connection errors |
| T6.3 | Handle chunked encoding |
| T6.4 | Handle trailers |
| T6.5 | Handle large bodies |
| T6.6 | Handle concurrent streams |
| T7.1 | Remove commented-out old code |
| T7.2 | Remove unused imports |
| T7.3 | Run cargo fmt |
| T7.4 | Run cargo clippy |
| T7.5 | Address clippy warnings |
| T8.1 | Write unit tests for redaction logic |
| T8.2 | Write integration tests for H2 MITM |
| T8.3 | Test with curl --http2 |
| T8.4 | Test with real browsers (Chrome, Firefox) |
| T8.5 | Test with various upstream servers |
| T9.1 | Run cargo check --workspace --all-targets |
| T9.2 | Run cargo build --workspace |
| T9.3 | Run cargo test --workspace |
| T9.4 | Run just ci |
| T9.5 | Document any remaining issues |

## ADDED Requirements

### Requirement: Phase ordering

T1.1 SHALL complete BEFORE T1.2.
T1.2 SHALL complete BEFORE T1.3.
T1.3 SHALL complete BEFORE T1.4.
T1.4 SHALL complete BEFORE T2.1.
T2.1 SHALL complete BEFORE T2.2.
T2.2 SHALL complete BEFORE T2.3.
T2.3 SHALL complete BEFORE T2.4.
T2.4 SHALL complete BEFORE T2.5.
T2.5 SHALL complete BEFORE T3.1.
T3.1 SHALL complete BEFORE T3.2.
T3.2 SHALL complete BEFORE T3.3.
T3.3 SHALL complete BEFORE T3.4.
T3.4 SHALL complete BEFORE T3.5.
T3.5 SHALL complete BEFORE T4.1.
T4.1 SHALL complete BEFORE T4.2.
T4.2 SHALL complete BEFORE T4.3.
T4.3 SHALL complete BEFORE T4.4.
T4.4 SHALL complete BEFORE T5.1.
T5.1 SHALL complete BEFORE T5.2.
T5.2 SHALL complete BEFORE T5.3.
T5.3 SHALL complete BEFORE T5.4.
T5.4 SHALL complete BEFORE T6.1.
T6.1 SHALL complete BEFORE T6.2.
T6.2 SHALL complete BEFORE T6.3.
T6.3 SHALL complete BEFORE T6.4.
T6.4 SHALL complete BEFORE T6.5.
T6.5 SHALL complete BEFORE T6.6.
T6.6 SHALL complete BEFORE T7.1.
T7.1 SHALL complete BEFORE T7.2.
T7.2 SHALL complete BEFORE T7.3.
T7.3 SHALL complete BEFORE T7.4.
T7.4 SHALL complete BEFORE T7.5.
T7.5 SHALL complete BEFORE T8.1.
T8.1 SHALL complete BEFORE T8.2.
T8.2 SHALL complete BEFORE T8.3.
T8.3 SHALL complete BEFORE T8.4.
T8.4 SHALL complete BEFORE T8.5.
T8.5 SHALL complete BEFORE T9.1.
T9.1 SHALL complete BEFORE T9.2.
T9.2 SHALL complete BEFORE T9.3.
T9.3 SHALL complete BEFORE T9.4.
T9.4 SHALL complete BEFORE T9.5.

#### Scenario: Phase 1 completes before Phase 2

- **WHEN** T1.4 completes
- **THEN** T2.1 SHALL be ready to start

#### Scenario: Phase 2 completes before Phase 3

- **WHEN** T2.5 completes
- **THEN** T3.1 SHALL be ready to start

#### Scenario: All phases complete

- **WHEN** T9.4 completes
- **THEN** T9.5 SHALL be ready to start
