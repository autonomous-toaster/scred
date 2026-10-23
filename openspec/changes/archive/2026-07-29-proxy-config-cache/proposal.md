## Why

`get_proxy_url()` in `upstream_connection.rs` calls `MitmConfig::from_env()` on every upstream connection. `from_env()` logs the full proxy configuration (`http_proxy`, `https_proxy`, `no_proxy`) each time. For a proxy handling many requests, this produces dozens of identical log lines per second — noise that drowns out useful signals.

## What Changes

- Cache the `MitmConfig` in `get_proxy_url()` using `std::sync::OnceLock`
- `from_env()` is called only once; subsequent calls return the cached config
- Proxy env var logging fires once on first request, then silence

## Capabilities

### New Capabilities
- `proxy-config-cache`: Cache proxy config to avoid repeated env var logging

### Modified Capabilities
- (none)

## Impact

- `crates/scred-http/src/upstream_connection.rs`: Add `OnceLock` cache in `get_proxy_url()`
