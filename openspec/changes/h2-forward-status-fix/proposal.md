## Why

The MITM proxy swallows upstream HTTP error responses (4xx, 5xx) when the upstream server negotiates HTTP/2. The response status code is discarded during H2→H1.1 transcoding, and the client receives a hardcoded `200 OK` instead of the actual status (e.g. `401 Unauthorized`). This breaks authentication flows, rate limiting, and any error handling that depends on the HTTP status code.

## What Changes

- `scred-mitm` H2 upstream forwarder: change return type from `Vec<u8>` (body only) to `(StatusCode, Vec<u8>)` through the entire H2 response pipeline
- `handle_h2_upstream_request` in `tls_mitm.rs`: use actual upstream status instead of hardcoded `200 OK`
- `send_h2_response` in `h2_mitm_handler.rs`: use actual upstream status instead of hardcoded `200`
- All intermediate functions that currently discard the status code

## Capabilities

### New Capabilities
- `h2-status-preservation`: The MITM proxy MUST forward upstream HTTP response status codes faithfully during H2→H1.1 transcoding. Error responses (4xx, 5xx) MUST reach the client with the correct status code.

### Modified Capabilities
<!-- No existing specs are modified - this is a bug fix for a capability that was never specified -->

## Impact

**Affected crate**: `scred-mitm`
- `crates/scred-mitm/src/mitm/tls_mitm.rs` — `handle_h2_upstream_request` hardcodes `200 OK`
- `crates/scred-mitm/src/mitm/h2_mitm_handler.rs` — `send_h2_response` hardcodes `200`
- `crates/scred-mitm/src/mitm/h2_upstream_forwarder/mod.rs` — `try_forward_h2`, `handle_upstream_h2_connection` return `Vec<u8>` (no status)
- `crates/scred-mitm/src/mitm/h2_upstream_forwarder/forward.rs` — `forward_via_http1_1`, `read_http1_response_direct`, `read_http1_response_redacted` all return `Vec<u8>` (no status)

**Not affected**: `scred-proxy` — operates on raw byte streams, no HTTP status parsing
