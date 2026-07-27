## Task Reference

| Task ID | Description |
|---------|-------------|
| T4.1 | Create tests/integration/proxy_body_redaction.sh (curl + assertions only) |
| T4.2 | Rewrite just test-integration-proxy (podman setup in Justfile, call script) |
| T5.1 | Run just ci and confirm zero violations |

## ADDED Requirements

### Requirement: Integration test script exists

T4.1 SHALL complete BEFORE T4.2 SHALL run.

#### Scenario: script tests AWS key redaction
- **WHEN** T4.1 runs
- **THEN** the script SHALL send a POST with `AKIAIOSFODNN7EXAMPLE` through the proxy
- **THEN** the script SHALL verify the secret is NOT present in the response body

#### Scenario: script tests JWT redaction
- **WHEN** T4.1 runs
- **THEN** the script SHALL send a POST with a JWT through the proxy
- **THEN** the script SHALL verify the JWT is NOT present in the response body

### Requirement: Justfile orchestrates integration test

T4.2 SHALL complete BEFORE T5.1 SHALL run.

#### Scenario: Justfile starts httpbin
- **WHEN** T4.2 runs
- **THEN** `just test-integration-proxy` SHALL start httpbin via podman

#### Scenario: Justfile starts scred-proxy
- **WHEN** T4.2 runs
- **THEN** `just test-integration-proxy` SHALL build and start scred-proxy

#### Scenario: Justfile calls test script
- **WHEN** T4.2 runs
- **THEN** `just test-integration-proxy` SHALL call `tests/integration/proxy_body_redaction.sh`

### Requirement: CI passes cleanly

T5.1 SHALL complete AFTER T4.2 SHALL complete AND T3.2 SHALL complete.

#### Scenario: just ci passes
- **WHEN** T5.1 runs
- **THEN** `just ci` SHALL exit with code 0
