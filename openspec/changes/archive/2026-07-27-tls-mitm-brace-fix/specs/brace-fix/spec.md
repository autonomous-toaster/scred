## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add missing `}` to close `else if is_chunked` block in handle_h2_client_transcoding |
| T1.2 | Remove duplicated dead code Block 2 (lines 1411-1629) |
| T1.3 | Verify cargo check -p scred-mitm --tests passes |

## ADDED Requirements

### Requirement: Dead code removed before verification

T1.1 SHALL complete BEFORE T1.2 SHALL run.
T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: else if is_chunked block closed
- **WHEN** T1.1 runs
- **THEN** the `else if is_chunked` block SHALL have a matching closing `}`

#### Scenario: Block 2 removed
- **WHEN** T1.2 runs
- **THEN** `tls_mitm.rs` SHALL NOT contain the duplicated Block 2 code

#### Scenario: cargo check passes
- **WHEN** T1.3 runs
- **THEN** `cargo check -p scred-mitm --tests` SHALL pass without brace errors
