## Task Reference

| Task ID | Description |
|---------|-------------|
| T3.1 | Wire RedactionStream into scred-proxy body forwarding path |
| T3.2 | Verify proxy body redaction with integration test |

## ADDED Requirements

### Requirement: Proxy redacts request and response bodies

T3.1 SHALL complete BEFORE T3.2 SHALL run.

The proxy SHALL redact all detected secret patterns from HTTP request and response bodies before forwarding. Body redaction SHALL use `RedactionStream` for streaming, zero-copy processing.

#### Scenario: Request body redacted
- **WHEN** T3.1 runs
- **THEN** a POST request with a detected secret (e.g., `AKIAIOSFODNN7EXAMPLE`) in the body SHALL have the secret redacted before reaching the upstream

#### Scenario: Response body redacted
- **WHEN** T3.1 runs
- **THEN** a response with a detected secret (e.g., `sk-proj-abc123`) in the body SHALL have the secret redacted before reaching the client
