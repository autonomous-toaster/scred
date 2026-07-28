# H2 Header Redaction

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add header value redaction to `H2MitmHandler::on_request_received` |
| T1.2 | Add non-regression tests for H2 header redaction |
| T1.3 | Verify curl headers are redacted through H2 path |
| T1.4 | Verify Node.js headers are redacted through H2 path |

## ADDED Requirements

### Requirement: H2 request header values are redacted (T1.1, T1.3, T1.4)

T1.1 SHALL complete BEFORE T1.3. T1.1 SHALL complete BEFORE T1.4.

#### Scenario: curl H2 request with secret header

- **WHEN** T1.3 sends a request through the MITM proxy with `Authorization: Bearer sk-proj-test123`
- **THEN** the upstream SHALL receive the header value redacted

#### Scenario: Node.js H2 request with secret header

- **WHEN** T1.4 sends a request through the MITM proxy with `some-thing: AKIAIOSFODNN7EXAMPLE`
- **THEN** the upstream SHALL receive the header value redacted

### Requirement: Hop-by-hop headers are preserved (T1.1, T1.3)

T1.1 SHALL complete BEFORE T1.3 ALWAYS.

#### Scenario: Host header preserved

- **WHEN** T1.3 sends a request with `Host: api.example.com`
- **THEN** the upstream SHALL receive the header value unchanged

### Requirement: Non-regression tests exist (T1.2, T1.3)

T1.2 SHALL complete BEFORE T1.3 ALWAYS.

#### Scenario: Unit test for header redaction

- **WHEN** T1.2 runs
- **THEN** it SHALL verify that secret header values are redacted and non-secret values are unchanged
