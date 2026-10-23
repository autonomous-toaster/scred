## ADDED Requirements

| Task | Description |
|------|-------------|
| T1.1 | Change `try_forward_h2` to return `(StatusCode, Vec<u8>)` |
| T1.2 | Change `handle_upstream_h2_connection` to return `(StatusCode, Vec<u8>)` |
| T1.3 | Change `forward_via_http1_1` to return `(StatusCode, Vec<u8>)` |
| T1.4 | Change `read_http1_response_direct` to return `(StatusCode, Vec<u8>)` |
| T1.5 | Change `read_http1_response_redacted` to return `(StatusCode, Vec<u8>)` |
| T2.1 | Update `handle_h2_upstream_request` to use actual status |
| T2.2 | Update `send_h2_response` to use actual status |
| T3.1 | Unit tests for `try_forward_h2` / `handle_upstream_h2_connection` status propagation |
| T3.2 | Unit test for `read_http1_response_direct` status parsing |
| T3.3 | Unit test for `read_http1_response_redacted` status parsing |
| T3.4 | Integration test: H2 upstream returns 401, client receives 401 |
| T3.5 | Run full test suite — verify no regressions |

### Requirement: Upstream status code MUST be forwarded faithfully

ALWAYS T1.1, T1.2, T2.1, T2.2 SHALL forward the upstream HTTP response status code to the client during H2→H1.1 transcoding without modification.

T3.1, T3.2, T3.3, T3.4 SHALL complete AFTER T1.1, T1.2, T1.3, T1.4, T1.5 SHALL run.

#### Scenario: H2 upstream returns 401 with body
- **WHEN** T1.1, T1.2, T2.1 run and the H2 upstream returns status 401 with error body
- **THEN** the client MUST receive `HTTP/1.1 401 Unauthorized` with the original body

#### Scenario: H2 upstream returns 200 with payload
- **WHEN** T1.1, T1.2, T2.1 run and the H2 upstream returns status 200 with payload
- **THEN** the client MUST receive `HTTP/1.1 200 OK` with the original (or redacted) body

#### Scenario: H2 fallback to HTTP/1.1 preserves status
- **WHEN** T1.3, T1.4, T1.5 run and H2 fails, falling back to HTTP/1.1
- **THEN** the status code from the HTTP/1.1 response line MUST be forwarded to the client

#### Scenario: Redacted response preserves status
- **WHEN** T1.5, T1.2 run with `RedactionMode::Redact`
- **THEN** the body SHALL be redacted but the status code MUST match the upstream response

#### Scenario: Native H2 client preserves status
- **WHEN** T1.2, T2.2 run and the native H2 client (`H2MitmHandler`) receives a non-2xx status
- **THEN** the client MUST receive the actual status code, not `200`

### Requirement: Error response on connection failure MUST return 502

ALWAYS T1.2, T2.2, T3.5 SHALL return 502 Bad Gateway on connection failure.

#### Scenario: Upstream connection failure
- **WHEN** upstream connection fails
- **THEN** the client MUST receive `HTTP/1.1 502 Bad Gateway`
