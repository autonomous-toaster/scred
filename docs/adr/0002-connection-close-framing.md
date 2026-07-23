---
status: accepted
date: 2026-07-12
---

# Connection-Close Framing for Streamed Responses

## Context and Problem Statement

After redaction, the response body byte count may differ from the original Content-Length header. SCRED needs a framing mechanism that works with streaming redaction without buffering the entire body to recalculate Content-Length.

## Considered Options

* **Recalculate Content-Length** — requires buffering the entire body to count bytes after redaction. Violates streaming-first (STD-002).
* **Use chunked transfer-encoding** — wrap the redacted body in chunked encoding. Adds complexity: chunk boundaries must be managed, trailers must be handled, and some clients don't support chunked responses to HTTP/1.1 requests.
* **Connection: close** — strip Content-Length and Transfer-Encoding, let the client detect end-of-body by connection close.

## Decision Outcome

Chosen option: **Connection: close** — strip Content-Length and Transfer-Encoding headers, add `Connection: close`, and let the client detect end-of-body by connection termination.

### Consequences

* Good, because it's the simplest correct framing — no buffering, no chunk management, no recalculation
* Good, because it works with all HTTP versions and all clients
* Bad, because it disables connection reuse (each request requires a new connection)
* Bad, because it doesn't work for long-lived streaming protocols (SSE, WebSockets) — those need separate handling

## Compliance

- `stream_response_to_client()` in `scred-http/src/streaming_response.rs` must strip Content-Length and Transfer-Encoding from forwarded headers
- `Connection: close` must be added to all streamed responses
- The `X-SCRED-Redacted` header is added to signal that redaction occurred
