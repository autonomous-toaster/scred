## 1. Core H2 response chain — propagate status code

- [x] 1.1 `try_forward_h2`: return `(StatusCode, Vec<u8>)` — pass status from `send_h2_request` response
- [x] 1.2 `handle_upstream_h2_connection`: return `(StatusCode, Vec<u8>)` — propagate from `try_forward_h2` or `forward_via_http1_1`
- [x] 1.3 `forward_via_http1_1`: return `(StatusCode, Vec<u8>)` — parse status from response
- [x] 1.4 `read_http1_response_direct`: return `(StatusCode, Vec<u8>)` — parse status before extracting body
- [x] 1.5 `read_http1_response_redacted`: return `(StatusCode, Vec<u8>)` — read first line for status before streaming redaction

## 2. Callers — use actual status

- [x] 2.1 `handle_h2_upstream_request` (tls_mitm.rs): use actual status instead of hardcoded `200 OK`
- [x] 2.2 `send_h2_response` (h2_mitm_handler.rs): use actual status instead of hardcoded `200`

## 3. Tests

- [x] 3.1 Add unit tests for `try_forward_h2` / `handle_upstream_h2_connection` status propagation (mocked H2 response with 401)
- [x] 3.2 Add unit test for `read_http1_response_direct` status parsing
- [x] 3.3 Add unit test for `read_http1_response_redacted` status parsing
- [x] 3.4 Add integration test: H2 upstream returns 401, client receives 401
- [x] 3.5 Run full test suite — verify no regressions (T1.9)
