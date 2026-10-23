## Context

When the MITM proxy connects to an upstream server that negotiates HTTP/2, the response path goes through `handle_upstream_h2_connection()`, which returns `Result<Vec<u8>>` — the response **body only**. The actual HTTP status code (e.g. `401 Unauthorized`) is extracted from the H2 response but then **discarded** at every layer:

1. `send_h2_request()` extracts `response_parts.status` but stores it in `_response` (discarded)
2. `try_forward_h2()` returns `Vec<u8>` (no status)
3. `handle_upstream_h2_connection()` returns `Vec<u8>` (no status)
4. Both callers hardcode `200 OK` when constructing the H1.1 response

The HTTP/1.1 path (`handle_single_request` → `read_response_line` + `forward_response_no_redaction`) does NOT have this bug — it reads the actual status line from upstream and forwards it verbatim.

`scred-proxy` is also not affected — it operates on raw byte streams and never parses or reconstructs HTTP status lines.

## Goals

- HTTP response status codes from upstream MUST reach the client faithfully during H2→H1.1 transcoding
- All 1xx, 2xx, 3xx, 4xx, 5xx responses MUST preserve their original status code
- The HTTP/1.1 fallback path (when H2 fails) MUST also preserve the status code
- Existing redaction behavior MUST be preserved (status code is never redacted)

## Non-Goals

- Forwarding upstream response **headers** (Content-Type, Set-Cookie, etc.) — these are already lost in the H2 path; fixing that is out of scope
- Changing the `scred-proxy` crate — not affected
- Changing the H1.1 MITM path — not affected

## Decisions

### Decision 1: Return `(StatusCode, Vec<u8>)` through the entire H2 chain

**Option A (chosen)**: Change return type from `Result<Vec<u8>>` to `Result<(StatusCode, Vec<u8>)>` at every level.

**Option B (rejected)**: Pass the status as an out-parameter (`&mut StatusCode`). More invasive, less idiomatic.

**Rationale**: Straightforward type change. The status is already available at the lowest level (`send_h2_request` receives `response_parts.status`). The H1.1 fallback functions already read the raw response line and can parse the status from it.

### Decision 2: Parse status from H1.1 response line in fallback path

For `forward_via_http1_1`, `read_http1_response_direct`, and `read_http1_response_redacted`, the status must be parsed from the response line (e.g. `HTTP/1.1 401 Unauthorized`).

`read_http1_response_direct` currently reads the full response via `read_response_direct()` then strips the body via `extract_http_response_body()`. The status line is available in the raw response before extraction.

`read_http1_response_redacted` uses streaming redaction. The first chunk contains the response line — we can buffer the first read and parse the status before passing to the redactor.

**Rationale**: Minimal changes to each function. The status line is guaranteed to be in the first read chunk (it's at the start of the response).

### Decision 3: Use `StatusCode` from the `http` crate

Both `StatusCode` and `http::Response` are already imported in the affected files. Rather than parsing to an integer, use `StatusCode::from_u16()` for validation.

**Rationale**: Zero new dependencies, validates the parsed value.

## Risks / Trade-offs

- **Risk**: `read_http1_response_redacted` uses chunked streaming. The first chunk may not be large enough to contain the full response line. **Mitigation**: Use `BufReader` to read the first line before starting the streaming redactor, then redirect remaining data through the redactor. The existing H1.1 path already does this pattern (`read_response_line` before `stream_response_with_redaction`).
- **Risk**: Backward compatibility — external code may depend on the `handle_upstream_h2_connection` signature. **Mitigation**: This is a private/internal function (`pub(crate)`), not a public API.
- **Trade-off**: Response headers (Content-Type, etc.) are still lost in the H2 path. This is a separate issue tracked as a non-goal.

## Open Questions

None. The approach is straightforward and the territory has been fully read.
