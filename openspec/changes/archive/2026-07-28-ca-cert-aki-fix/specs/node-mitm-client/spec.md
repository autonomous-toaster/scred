# Node.js MITM Client

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add `use_authority_key_identifier_extension = true` to `generate_cert_signed_by_ca` |
| T2.2 | Add Node.js example to README using `NODE_EXTRA_CA_CERTS` |
| T3.2 | Verify Node.js `fetch` works through MITM proxy |

## ADDED Requirements

### Requirement: Node.js clients can connect through MITM proxy

T1.1 SHALL complete BEFORE T3.2. T2.2 SHALL complete BEFORE T3.2.

#### Scenario: Node.js fetch through MITM

- **WHEN** T3.2 runs `node -e "await fetch('https://httpbin.org/anything', ...)"` with `HTTPS_PROXY` and `NODE_EXTRA_CA_CERTS` set
- **THEN** the request SHALL succeed with status 200
