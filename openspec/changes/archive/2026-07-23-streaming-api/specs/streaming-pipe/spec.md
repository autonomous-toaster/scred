## Task Reference

| Task ID | Description |
|---------|-------------|
| T4.1 | Implement RedactionStream::pipe() |

## ADDED Requirements

### Requirement: Read, redact, write in one async call

T4.1 SHALL complete BEFORE any integration tests SHALL run.

#### Scenario: Pipe buffer to buffer
- **WHEN** T4.1 runs
- **THEN** piping from a buffer containing `AKIAIOSFODNN7EXAMPLE` to an output buffer SHALL write `AKIAxxxxxxxxxxxxxxxx` to the output

#### Scenario: Pipe returns stats
- **WHEN** T4.1 runs
- **THEN** the returned Stats SHALL include bytes_read, bytes_written, patterns_found, chunks_processed
