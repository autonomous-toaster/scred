## Context

SCRED's current streaming API (`StreamingRedactor::process_chunk`) requires callers to manage a lookahead buffer, track byte positions, and signal EOF. This leaks the internal windowing protocol and makes the library error-prone. The HTTP proxy code in `scred-http` has to duplicate this logic in every streaming handler.

This change introduces a clean streaming API that hides the windowing entirely, using Rust's ownership model to enforce correct usage.

## Goals / Non-Goals

**Goals:**
- `RedactionStream` — feed chunks, get redacted bytes, finalize to flush lookahead
- `DetectionStream` — feed chunks, get match events, finalize to flush lookahead
- `AsyncRedactionReader<R: AsyncRead>` — transparent redaction for any async source
- `RedactionStream::pipe()` — convenience for read→redact→write
- Remove old `process_chunk` / `process_chunk_bytes` from public API

**Non-Goals:**
- Custom redaction strategies (placeholder, etc.) — streaming requires same-length replacement
- Lua/plugin pattern extensibility
- Performance optimization beyond the current baseline

## Decisions

### finalize(self) consumes the stream

`finalize()` takes `self` by value, consuming the stream. After finalize, the compiler prevents any further calls. Stats are returned in the tuple alongside the flushed lookahead.

This is preferred over `finalize(&mut self)` with a separate `stats()` method because:
- No panic path — the compiler enforces the state transition
- No runtime checks — zero cost
- Stats are always available at finalize time

### Chunk size and lookahead size are internal

The 64KB chunk size and 512B lookahead size are implementation details tied to the pattern engine. The lookahead size (512B) is verified to be >= the longest pattern prefix (22 bytes for `pypi-AgEIcHlwaS5vcmc`). The pattern's internal lookahead (up to 20KB for PGP keys) is separate and handled by the pattern scanner, not the streaming window.

Exposing these values would let clients silently break pattern boundary detection. They remain internal.

### AsyncRedactionReader uses iteration cap

`poll_read` loops reading from the inner source, feeding the stream, and buffering output. An iteration cap of 8 prevents starvation of other tasks. After the cap, `cx.waker().wake_by_ref()` + `Poll::Pending` yields to the executor. The stream's lookahead preserves state across calls.

### DetectionStream is a separate type

`DetectionStream` returns `Vec<Match>` instead of `Vec<u8>`. These are different return types with different semantics. A single type with a mode flag would require runtime checks and make the API harder to reason about.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| `finalize(self)` prevents calling `stats()` after finalize | Stats are returned in the finalize tuple |
| Drop warning may be missed in logs | Warning is at WARN level; can be promoted to panic in debug builds |
| Iteration cap may cause unnecessary yielding | Cap of 8 is generous; rarely hit in practice (inner either has data or doesn't) |
| 512B lookahead insufficient for future patterns | Compute lookahead from pattern definitions at startup; update if patterns change |
