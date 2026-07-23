# STD-002 · Streaming-First

## Rule

All HTTP body redaction MUST process data in streaming fashion — no full-body buffering before redaction.

The streaming contract:
1. Bodies are processed in 64KB chunks via `StreamingRedactor::process_chunk()`
2. A 512-byte lookahead buffer is maintained across chunks to handle pattern boundaries
3. Headers are still parsed non-streamingly (they are small and bounded)
4. After redaction, the body is framed by `Connection: close` (see ADR-002)

Full-body buffering is FORBIDDEN in proxy paths. The CLI tool (`scred`) may buffer files since they are bounded by disk.

## Rationale

Streaming enables handling arbitrary-sized payloads without memory growth. A 1GB file upload should not require 1GB of buffer memory. The lookahead buffer (512 bytes) is sufficient because all patterns have bounded token lengths (max 500 bytes for generic patterns, 300 for most, 10000 for JWT which has its own scanner).

## Compliance

- All proxy request/response handlers in `scred-http/src/streaming_request.rs` and `scred-http/src/streaming_response.rs` must use `StreamingRedactor::process_chunk()`
- No `read_to_end()` or equivalent full-body reads in proxy paths
- The `ChunkedParser` must process chunks incrementally through the redactor
