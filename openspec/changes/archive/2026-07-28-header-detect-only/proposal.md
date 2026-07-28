## Why

The MITM proxy currently redacts ALL header values by default. This breaks upstream requests — headers like `Authorization`, `X-Api-Key`, and `Cookie` are needed by the upstream to function. The body is where secrets typically live; headers should be detect-only by default (log without modifying, without logging the full value).

## What Changes

- H2 path (`apply_header_policy`): when no policy engine, use `detect_all()` instead of `redact()` — log pattern type + header name, return original value
- HTTP/1.1 path (`stream_request_to_upstream`): use `detect_all()` per header value instead of `redact_buffer()` on raw headers — log pattern type + header name, forward headers unchanged
- Log messages include header name and pattern type only (no full header value)

## Capabilities

### New Capabilities
- `header-detect-only`: MITM proxy defaults to detect-only for headers, redact for body

### Modified Capabilities
- (none)

## Impact

- `crates/scred-mitm/src/mitm/h2_mitm_handler.rs`: Change `apply_header_policy` no-policy branch from redact to detect-only
- `crates/scred-http/src/streaming_request.rs`: Change header forwarding from `redact_buffer` to per-header `detect_all`
