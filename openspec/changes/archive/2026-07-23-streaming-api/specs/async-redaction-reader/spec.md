## Task Reference

| Task ID | Description |
|---------|-------------|
| T3.1 | Implement AsyncRedactionReader<R: AsyncRead> |
| T3.2 | Implement poll_read with iteration cap and wake_by_ref pattern |
| T3.3 | Implement Drop warning for cancelled futures |

## ADDED Requirements

### Requirement: Wrap any AsyncRead, redact transparently

T3.1 SHALL complete BEFORE T3.2 SHALL run.

#### Scenario: Read all from wrapped reader
- **WHEN** T3.1 runs
- **THEN** wrapping a byte source containing `AKIAIOSFODNN7EXAMPLE` and reading to completion SHALL yield `AKIAxxxxxxxxxxxxxxxx`

#### Scenario: Partial reads
- **WHEN** T3.1 runs
- **THEN** reading 1 byte at a time from the wrapper SHALL eventually yield all redacted bytes

### Requirement: poll_read handles backpressure

T3.2 SHALL complete BEFORE T3.3 SHALL run.

#### Scenario: Inner reader always has data
- **WHEN** T3.2 runs
- **THEN** if the inner reader provides data faster than the lookahead fills, poll_read SHALL yield to the executor after 8 iterations

#### Scenario: Inner reader returns Pending
- **WHEN** T3.2 runs
- **THEN** if the inner reader returns Poll::Pending, poll_read SHALL propagate Pending

### Requirement: Drop warns on cancelled future

T3.3 SHALL complete AFTER T3.2 SHALL complete.

#### Scenario: Future cancelled mid-stream
- **WHEN** T3.3 runs
- **THEN** dropping the reader with unread data in the output buffer or lookahead SHALL log a warning
