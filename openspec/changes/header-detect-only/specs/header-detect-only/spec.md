# Header Detect-Only

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Change `apply_header_policy` no-policy branch from redact to detect-only in H2 path |
| T1.2 | Change `stream_request_to_upstream` header handling from `redact_buffer` to per-header `detect_all` |
| T1.3 | Update tests for detect-only header behavior |

## ADDED Requirements

### Requirement: All path headers are detect-only (T1.1, T1.2, T1.3)

T1.1 SHALL complete BEFORE T1.3 ALWAYS. T1.2 SHALL complete BEFORE T1.3.

#### Scenario: HTTP/1.1 header with secret is detected but not modified

- **WHEN** T1.2 processes a header with a secret value
- **THEN** the header value SHALL be forwarded unchanged

#### Scenario: HTTP/1.1 header detection is logged

- **WHEN** T1.2 detects a secret in a header
- **THEN** a log message SHALL contain the pattern type and header name but NOT the full value
