# Python MITM Client

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add `use_authority_key_identifier_extension = true` to `generate_cert_signed_by_ca` |
| T2.1 | Add Python example to README using `REQUESTS_CA_BUNDLE` |
| T3.1 | Verify Python `requests` works through MITM proxy |

## ADDED Requirements

### Requirement: Python clients can connect through MITM proxy

T1.1 SHALL complete BEFORE T3.1. T2.1 SHALL complete BEFORE T3.1.

#### Scenario: Python requests through MITM

- **WHEN** T3.1 runs `python3 -c "import requests; requests.post('https://httpbin.org/anything', ...)"` with `HTTPS_PROXY` and `REQUESTS_CA_BUNDLE` set
- **THEN** the request SHALL succeed with status 200

#### Scenario: Secret redaction visible

- **WHEN** T3.1 sends a secret in the request body
- **THEN** the response SHALL show the secret redacted
