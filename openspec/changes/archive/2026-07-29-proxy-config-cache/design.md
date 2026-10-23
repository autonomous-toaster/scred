## Context

`MitmConfig::from_env()` reads `http_proxy`/`https_proxy`/`no_proxy` env vars and logs them. It's called from `get_proxy_url()` which is invoked on every upstream connection:

```
connect_h2_upstream()  ──▶ get_proxy_url() ──▶ MitmConfig::from_env()  ← logs every time
connect_http1_upstream() ──▶ get_proxy_url() ──▶ MitmConfig::from_env()  ← logs every time
```

For a proxy handling many requests, this produces identical log lines on every request.

## Current behavior

```
WARN Proxy environment variables detected - these will affect upstream routing
WARN http_proxy: http://proxy.example.com:3128
WARN https_proxy: http://proxy.example.com:3128
WARN no_proxy: 3 entries - localhost, .internal, .example.com
... (repeated for every request) ...
```

## Desired behavior

```
WARN Proxy environment variables detected - these will affect upstream routing
WARN http_proxy: http://proxy.example.com:3128
WARN https_proxy: http://proxy.example.com:3128
WARN no_proxy: 3 entries - localhost, .internal, .example.com
... (no repeat) ...
```

## Goals / Non-Goals

**Goals:**
- Cache the proxy config so `from_env()` is called only once
- Log proxy env vars only on first initialization
- Keep the same behavior for the forward proxy (`scred-proxy`)

**Non-Goals:**
- Changing `MitmConfig::from_env()` itself
- Changing the forward proxy's per-connection config creation

## Decisions

### Decision 1: Cache in `get_proxy_url()`, not in `from_env()`
Caching in `get_proxy_url()` is the least invasive change. `MitmConfig::from_env()` stays unchanged. The forward proxy (`proxy.rs:74`) still creates its own config per-connection, which is a different use case.

### Decision 2: Use `std::sync::OnceLock`
`OnceLock` is stable since Rust 1.70. It's the standard way to lazily initialize a static value. No external dependencies needed.

## Risks / Trade-offs

- **[Risk]** If env vars change at runtime, the cached config is stale → **[Mitigation]** Proxy env vars are set at process startup and never change. This is standard behavior.
- **[Risk]** `OnceLock` requires `std` (not `no_std`) → **[Mitigation]** The crate already uses `std`.
