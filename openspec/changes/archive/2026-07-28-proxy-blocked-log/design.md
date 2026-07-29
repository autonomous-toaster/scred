## Context

### Blocked CONNECT logging

Two `connect_through_proxy` functions exist:

1. **`proxy_resolver::connect_through_proxy`** — used by the MITM proxy (`tls_mitm.rs`, `upstream_connector.rs`). Logs the full response body on error.
2. **`upstream_connection::connect_through_proxy`** — used by the forward proxy. Only logs the status line.

Both read 1024 bytes to check the CONNECT response. For a 200 response, extra bytes are tunneled data (harmless). For a 403 response, the 1024 bytes contain the HTML error page.

### Access logging

The MITM proxy has three request paths, none of which log the request URL at `info!` level:

1. **H2 client** (`H2MitmHandler::handle_stream`): has `debug!` with method + URI
2. **HTTP/1.1 → H2 upstream** (`handle_h2_upstream_request`): no request log
3. **HTTP/1.1 → HTTP/1.1 upstream** (`handle_single_request`): no request log

Users want a lightweight access log like nginx showing every proxied request.

## Current behavior

```
ERROR Proxy rejected CONNECT: HTTP/1.1 403 URLBlocked Via: ... <3550 bytes of HTML>
ERROR Failed to connect to upstream <proxy_url>: Proxy rejected CONNECT (not 200 response)
```

No access log for proxied requests.

## Desired behavior

```
INFO  POST https://httpbin.h.wpfq.fr/anything
WARN  Proxy blocked CONNECT to registry.npmjs.org:443 - HTTP/1.1 403 URLBlocked
ERROR Failed to connect to upstream <proxy_url>: Proxy blocked CONNECT to registry.npmjs.org:443 - HTTP/1.1 403 URLBlocked
```

## Goals / Non-Goals

**Goals:**
- Log only the status line, not the full response body
- Include the target host:port in the log message
- Use `warn!` level instead of `error!` (blocked requests are expected, not proxy errors)
- Log every proxied request at `info!` level with `METHOD https://host/path`

**Non-Goals:**
- Changing the 1024-byte read behavior (it's fine for 200 responses)
- Adding full URL logging (CONNECT only has host:port, not the full path)
- Logging response status codes (just the request URL for now)

## Decisions

### Decision 1: Log level `warn!` instead of `error!`
A blocked CONNECT is an expected outcome when a corporate proxy filters traffic. It's not a proxy error — it's a routing failure. `warn!` is more appropriate.

### Decision 2: Include target host in the message
The caller logs the proxy address, not the target. Adding `target_host:target_port` to the message makes it immediately clear which site was blocked.

### Decision 3: Keep the caller's error log
The caller (`tls_mitm.rs:281`) logs "Failed to connect to upstream {}: {}" with the proxy address. This is still useful context. The two lines together give a complete picture.

### Decision 4: Log at the handler level, not in shared code
Each request path logs its own access log line. This avoids coupling the logging to shared utility functions and keeps the log format consistent with the available data (full URI for H2, host+path for HTTP/1.1).

### Decision 5: `info!` level for access logs
Access logs should be visible at the default log level (`info`). Users who want to suppress them can set `RUST_LOG=warn`.

## Risks / Trade-offs

- **[Risk]** Users parsing the old log format will break → **[Mitigation]** The old format was dumping raw HTML, which no one should be parsing.
- **[Risk]** Access logs increase log volume → **[Mitigation]** Each request produces one line. At `info!` level, this is standard for proxy access logging.
