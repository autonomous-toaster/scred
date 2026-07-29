## 1. Fix proxy blocked CONNECT logging

- [x] 1.1 Fix `proxy_resolver::connect_through_proxy` in `crates/scred-http/src/proxy_resolver.rs`:
  - Change `error!` to `warn!`
  - Log only the status line + target host:port
  - Do not log the full response body
- [x] 1.2 Fix `upstream_connection::connect_through_proxy` in `crates/scred-http/src/upstream_connection.rs`:
  - Add target host:port to the `warn!` message

## 2. Add access log for proxied requests

- [x] 2.1 Add access log in `H2MitmHandler::handle_stream` (`crates/scred-mitm/src/mitm/h2_mitm_handler.rs`):
  - Change `tracing::debug!` to `tracing::info!` for the request log
  - Format: `{method} {uri}`
- [x] 2.2 Add access log in `handle_h2_upstream_request` (`crates/scred-mitm/src/mitm/tls_mitm.rs`):
  - Add `tracing::info!` after parsing method and path
  - Format: `{method} https://{target_host}{path}`
- [x] 2.3 Add access log in `handle_single_request` (`crates/scred-mitm/src/mitm/tls_mitm.rs`):
  - Add `tracing::info!` before the HTTP/1.1 upstream branch (after the H2 upstream check)
  - Format: `{method} https://{target_host}{path}`
