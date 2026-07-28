## Context

The MITM proxy has two code paths for processing client requests:

1. **H2 path** (`H2MitmHandler`): clients that negotiate HTTP/2 via ALPN. Headers arrive in HEADERS frames. `apply_header_policy()` processes each header value. When no `PolicyEngine` is configured, it currently redacts all values using `RedactionEngine::redact()`.

2. **HTTP/1.1 path** (`stream_request_to_upstream`): clients that speak HTTP/1.1. Headers arrive as raw text. The current code runs `redact_buffer()` on the entire raw headers block, which redacts everything.

Both paths need to change from "redact all header values" to "detect-only: log pattern type + header name, pass through unchanged."

## Goals / Non-Goals

**Goals:**
- Header values are never modified by default (no redaction)
- Secret detection still runs on header values — results are logged
- Log messages contain header name and pattern type only (no full value)
- Body redaction is unchanged (still redacts)

**Non-Goals:**
- Changing the `RedactionMode` enum or config schema
- Adding per-header configuration options
- Changing response handling

## Decisions

### Decision 1: Use `scred_detector::detect_all()` for header detection
`detect_all()` returns matches without modifying the input. This is exactly what detect-only needs. The `RedactionEngine::redact()` method both detects and redacts — too heavy for detect-only.

### Decision 2: Log format: `[H2] Detected {type} in header: {name}`
No full header value in logs. Just the header name and the pattern type (e.g., `aws-access-key`, `openai-api-key`). This gives enough information to diagnose issues without leaking secrets.

### Decision 3: HTTP/1.1 path uses per-header detection instead of bulk redact
Instead of `redact_buffer(raw_headers)` which redacts the entire header block, iterate over parsed headers and run `detect_all()` on each value individually. Forward the original raw headers unchanged.

## Risks / Trade-offs

- **[Risk]** Detect-only means secrets in headers reach the upstream → **[Mitigation]** This is intentional — the upstream needs those headers. Body redaction still protects the primary leak vector.
- **[Risk]** Log spam if many headers contain secrets → **[Mitigation]** Use `info` level, can be filtered. Each header+pattern pair is logged once per request.
