## Why

Two issues:

1. **Blocked URL noise**: When the corporate proxy blocks a CONNECT request, `proxy_resolver::connect_through_proxy` logs the entire HTML error page (up to 1024 bytes) as a single log line. This is noisy and unhelpful.

2. **No access log**: The MITM proxy doesn't log which URLs are being requested. Users want a lightweight access log like nginx showing `METHOD https://host/path` for every request.

## What Changes

- `proxy_resolver::connect_through_proxy`: log only the status line + target host instead of the full response body
- `upstream_connection::connect_through_proxy`: add target host to the log message for consistency
- `H2MitmHandler::handle_stream`: change `debug!` to `info!` with method + full URI
- `handle_single_request`: add `info!` log with `METHOD https://host/path` before upstream forwarding
- `handle_h2_upstream_request`: add `info!` log with `METHOD https://host/path`

## Capabilities

### New Capabilities
- `proxy-blocked-log`: Concise blocked-URL logging when corporate proxy rejects CONNECT
- `access-log`: Lightweight access log showing every proxied request

### Modified Capabilities
- (none)

## Impact

- `crates/scred-http/src/proxy_resolver.rs`: Change error log to show status line + target host
- `crates/scred-http/src/upstream_connection.rs`: Add target host to warn log for consistency
- `crates/scred-mitm/src/mitm/h2_mitm_handler.rs`: Change `debug!` to `info!` for request logging
- `crates/scred-mitm/src/mitm/tls_mitm.rs`: Add `info!` access logs in `handle_single_request` and `handle_h2_upstream_request`
