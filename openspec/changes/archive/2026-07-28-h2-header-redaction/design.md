## Context

The `H2MitmHandler` handles H2 streams from clients that negotiate HTTP/2 via ALPN. It receives H2 HEADERS frames and DATA frames, forwards them to the upstream, and streams responses back. Currently it redacts response bodies via `redact_h2_body()` but does not redact request header values.

The HTTP/1.1 → H2 upstream path (`handle_h2_upstream_request` in `tls_mitm.rs`) already redacts headers correctly using `redact_for_h2_upstream()`. The same `StreamingRedactor::redact_buffer()` approach should work for H2 header values.

## Goals / Non-Goals

**Goals:**
- H2 request header values are redacted before forwarding to upstream
- Header names are preserved (only values are redacted)
- Hop-by-hop headers (`host`, `connection`, `transfer-encoding`) are preserved as-is
- Non-regression tests verify header redaction

**Non-Goals:**
- Response header redaction (separate concern, lower priority)
- Refactoring the H2MitmHandler architecture

## Decisions

### Decision 1: Redact header values in `on_request_received`
The `H2MitmHandler` receives H2 requests via `on_request_received` callback. Header values should be redacted here, before the request is forwarded upstream.

### Decision 2: Use `StreamingRedactor::redact_buffer()` per header value
Same approach as `redact_for_h2_upstream()` — redact each header value individually, preserving the header name. This is consistent with the HTTP/1.1 path.

### Decision 3: Skip hop-by-hop headers
`host`, `connection`, `transfer-encoding` headers should not be redacted (same as HTTP/1.1 path).

## Risks / Trade-offs

- **[Risk]** H2 HEADERS frames may contain sensitive data in pseudo-headers (`:path`, `:authority`) → **[Mitigation]** Only redact regular headers, not pseudo-headers
- **[Risk]** Performance impact of redacting each header value individually → **[Mitigation]** Header values are typically small; `redact_buffer()` is O(n)
