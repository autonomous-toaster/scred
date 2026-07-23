## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Implement RedactionStream with internal lookahead management |
| T1.2 | Implement finalize(self) that consumes self and returns (Vec<u8>, Stats) |
| T1.3 | Implement Drop warning for unfinalized streams |
| T1.4 | Remove old process_chunk / process_chunk_bytes from public API |

## ADDED Requirements

### Requirement: Feed chunks, get redacted output

T1.1 SHALL complete BEFORE T1.2 SHALL run.

#### Scenario: Single chunk with secret
- **WHEN** T1.1 runs
- **THEN** feeding a chunk containing `AKIAIOSFODNN7EXAMPLE` SHALL return `AKIAxxxxxxxxxxxxxxxx`

#### Scenario: Secret spans two chunks
- **WHEN** T1.1 runs
- **THEN** feeding `"data AKIA"` then `"IOSFODNN7EXAMPLE more"` SHALL return `"data "` then `"AKIAxxxxxxxxxxxxxxxx more"`

#### Scenario: Empty chunk
- **WHEN** T1.1 runs
- **THEN** feeding an empty slice SHALL return empty output

### Requirement: Finalize flushes lookahead

T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: Finalize with data in lookahead
- **WHEN** T1.2 runs
- **THEN** feeding 100 bytes then finalizing SHALL return the 100 bytes

#### Scenario: Finalize returns stats
- **WHEN** T1.2 runs
- **THEN** the returned Stats SHALL include bytes_read, bytes_written, patterns_found, chunks_processed

### Requirement: Drop warns on unfinalized stream

T1.3 SHALL complete AFTER T1.2 SHALL complete.

#### Scenario: Drop with data in lookahead
- **WHEN** T1.3 runs
- **THEN** dropping a stream with 100 bytes in the lookahead SHALL log a warning at WARN level

#### Scenario: Drop with empty lookahead
- **WHEN** T1.3 runs
- **THEN** dropping a finalized stream SHALL NOT log a warning

### Requirement: Old API removed

T1.4 SHALL complete AFTER T1.1 SHALL complete.

#### Scenario: Old methods not accessible
- **WHEN** T1.4 runs
- **THEN** external code SHALL NOT be able to call process_chunk or process_chunk_bytes
