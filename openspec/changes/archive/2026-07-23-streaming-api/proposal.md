## Why

SCRED's streaming redaction API requires callers to manually manage a lookahead buffer, track byte positions, and signal EOF — exposing the internal windowing protocol. This makes the library hard to use correctly and creates a maintenance burden for every consumer (HTTP proxy, CLI, future integrations). A clean streaming API should hide the windowing entirely: feed chunks in, get redacted output out, with no lookahead management, no EOF tracking, and no risk of silent data loss.

## What Changes

- Add `RedactionStream` — `feed(&mut self, &[u8]) -> Vec<u8>`, `finalize(self) -> (Vec<u8>, Stats)`. Consumes self to prevent use-after-finalize.
- Add `DetectionStream` — same pattern but returns `Vec<Match>` instead of redacted bytes.
- Add `AsyncRedactionReader<R: AsyncRead>` — wraps any AsyncRead, redacts transparently.
- Add `RedactionStream::pipe()` — convenience for read→redact→write.
- **BREAKING**: Remove `StreamingRedactor::process_chunk()` and `StreamingRedactor::process_chunk_bytes()` from the public API. These expose the lookahead buffer and EOF flag.
- **BREAKING**: Remove `StreamingConfig` chunk_size and lookahead_size from public API. These are internal implementation details.
- Update HTTP proxy code (`streaming_request.rs`, `streaming_response.rs`) to use the new API.

## Capabilities

### New Capabilities
- `redaction-stream`: Sync streaming redaction with internal windowing — feed chunks, get redacted bytes, finalize to flush lookahead
- `detection-stream`: Sync streaming detection — feed chunks, get match events, finalize to flush lookahead
- `async-redaction-reader`: AsyncRead wrapper that redacts transparently — wrap any AsyncRead source
- `streaming-pipe`: Convenience function for read→redact→write in one async call

### Modified Capabilities
<!-- No existing specs to modify -->

## Impact

- `scred-redactor/src/streaming.rs` — new types, removal of old `process_chunk` API
- `scred-http/src/streaming_request.rs` — use new `RedactionStream` instead of manual lookahead
- `scred-http/src/streaming_response.rs` — use new `RedactionStream` instead of manual lookahead
- `scred-http/src/chunked_parser.rs` — use new `RedactionStream` instead of manual lookahead
- No new external dependencies
