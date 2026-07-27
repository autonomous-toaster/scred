## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Remove duplicated dead code Block 2 (lines 1411-1629) from handle_h2_client_transcoding |
| T1.2 | Verify brace balance — cargo check -p scred-mitm passes for both lib and test |
| T2.1 | Extract response body forwarding into forward_response_body() |
| T2.2 | Verify no behavioral change — existing tests pass |
| T3.1 | Create tls_mitm/ directory with mod.rs, handler.rs, helpers.rs, tests.rs |
| T3.2 | Move handle_h2_client_transcoding to handler.rs |
| T3.3 | Move helper functions to helpers.rs |
| T3.4 | Move test functions to tests.rs |
| T3.5 | Update mod.rs declarations and re-exports |
| T3.6 | Verify file sizes — all under 550 lines |
| T3.7 | Full test suite passes |

## ADDED Requirements

### Requirement: Dead code removed before verification

T1.1 SHALL complete BEFORE T1.2 SHALL run.

#### Scenario: Block 2 removed
- **WHEN** T1.1 runs
- **THEN** `tls_mitm.rs` SHALL NOT contain the duplicated Block 2 code

### Requirement: Brace balance verified

T1.2 SHALL complete BEFORE T2.1 SHALL run.

#### Scenario: cargo check passes
- **WHEN** T1.2 runs
- **THEN** `cargo check -p scred-mitm --tests` SHALL pass without brace errors

### Requirement: Helper extraction preserves behavior

T2.1 SHALL complete BEFORE T2.2 SHALL run.

#### Scenario: forward_response_body extracted
- **WHEN** T2.1 runs
- **THEN** `handle_h2_client_transcoding` SHALL call `forward_response_body()` instead of inline body forwarding

### Requirement: Tests pass after extraction

T2.2 SHALL complete BEFORE T3.1 SHALL run.

#### Scenario: existing tests pass
- **WHEN** T2.2 runs
- **THEN** `cargo test -p scred-mitm` SHALL pass

### Requirement: Module structure created before moving code

T3.1 SHALL complete BEFORE T3.2 SHALL run.

#### Scenario: directory exists
- **WHEN** T3.1 runs
- **THEN** `crates/scred-mitm/src/mitm/tls_mitm/` SHALL exist with mod.rs, handler.rs, helpers.rs, tests.rs

### Requirement: Code moved to sub-modules

T3.2 SHALL complete BEFORE T3.3 SHALL run.

#### Scenario: handler.rs contains transcoding
- **WHEN** T3.2 runs
- **THEN** `handler.rs` SHALL contain `handle_h2_client_transcoding`

T3.3 SHALL complete BEFORE T3.4 SHALL run.

#### Scenario: helpers.rs contains helpers
- **WHEN** T3.3 runs
- **THEN** `helpers.rs` SHALL contain `send_h2_error_response`, `encode_h2_headers_frame`, `encode_h2_data_frame`, `parse_status_code`

T3.4 SHALL complete BEFORE T3.5 SHALL run.

#### Scenario: tests.rs contains tests
- **WHEN** T3.4 runs
- **THEN** `tests.rs` SHALL contain `test_tls_mitm_compiles`, `test_streaming_mode_always_active`, `test_single_request_handler_signature`

### Requirement: Module declarations updated

T3.5 SHALL complete BEFORE T3.6 SHALL run.

#### Scenario: mod.rs updated
- **WHEN** T3.5 runs
- **THEN** `mod.rs` SHALL declare `pub mod tls_mitm;` instead of `pub mod tls_mitm;` pointing to the file

### Requirement: File sizes within limit

T3.6 SHALL complete BEFORE T3.7 SHALL run.

#### Scenario: all files under 550 lines
- **WHEN** T3.6 runs
- **THEN** each file in `tls_mitm/` SHALL be under 550 lines

### Requirement: Full test suite passes

T3.7 SHALL complete AFTER T3.6 SHALL complete.

#### Scenario: all tests pass
- **WHEN** T3.7 runs
- **THEN** `cargo test -p scred-mitm` SHALL pass
