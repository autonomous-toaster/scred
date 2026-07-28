## Why

When clients negotiate HTTP/2 with the MITM proxy (curl, Node.js with `NODE_USE_ENV_PROXY=1`), the `H2MitmHandler` path redacts response bodies but does **not** redact request headers. Secrets in headers like `Authorization`, `X-Api-Key`, or custom headers pass through unredacted to the upstream.

The HTTP/1.1 → H2 upstream path (`handle_h2_upstream_request`) already redacts headers correctly — only the native H2 path is missing this.

## What Changes

- Add header value redaction to `H2MitmHandler` before forwarding H2 request frames upstream
- Add non-regression tests for H2 header redaction

## Capabilities

### New Capabilities
- `h2-header-redaction`: H2 MITM handler redacts request header values before forwarding to upstream

### Modified Capabilities
- (none)

## Impact

- `crates/scred-mitm/src/mitm/h2_mitm_handler.rs`: Add header redaction logic
- `crates/scred-mitm/src/mitm/h2_mitm_handler.rs` or `tests/`: Add tests
