## Task Reference

| Task ID | Description |
|---------|-------------|
| T2.1 | Implement DetectionStream with internal lookahead management |
| T2.2 | Implement finalize(self) that consumes self and returns (Vec<Match>, Stats) |

## ADDED Requirements

### Requirement: Feed chunks, get match events

T2.1 SHALL complete BEFORE T2.2 SHALL run.

#### Scenario: Single chunk with secret
- **WHEN** T2.1 runs
- **THEN** feeding a chunk containing `AKIAIOSFODNN7EXAMPLE` SHALL return a match with start=0, end=20

#### Scenario: Secret spans two chunks
- **WHEN** T2.1 runs
- **THEN** feeding `"data AKIA"` then `"IOSFODNN7EXAMPLE more"` SHALL return no matches from the first feed, then a match from the second feed covering the full secret

#### Scenario: No secrets
- **WHEN** T2.1 runs
- **THEN** feeding plain text SHALL return an empty slice

### Requirement: Finalize flushes lookahead matches

T2.2 SHALL complete AFTER T2.1 SHALL complete.

#### Scenario: Finalize with match in lookahead
- **WHEN** T2.2 runs
- **THEN** feeding 100 bytes containing a secret, then finalizing, SHALL return the match
