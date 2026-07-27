## Context

`tls_mitm.rs` has a brace imbalance that blocks `cargo check -p scred-mitm --tests`. The root cause is in `handle_h2_client_transcoding`:

1. The `else if is_chunked` block (line 1274) is missing its closing `}`
2. The `}` at line 1367 closes both the `else if is_chunked` block and the `FrameType::Headers` arm
3. A duplicated dead code block (Block 2, lines 1411-1629) has extra `}` that compensate, allowing the lib to compile but not the test build

## Goals / Non-Goals

**Goals:**
- Fix brace imbalance so `cargo check -p scred-mitm --tests` passes
- Remove duplicated dead code Block 2
- Minimal changes — no refactoring, no behavioral changes

**Non-Goals:**
- Extracting helper functions
- Splitting into sub-modules
- Any behavioral changes

## Decisions

### Fix strategy

The fix is surgical:
1. Add `}` at indent 32 after line 1359 to close the `else if is_chunked` block
2. Remove Block 2 (lines 1411-1629) — this is dead code duplicated from Block 1

This is the minimal fix. The function remains 662 lines but the braces are balanced.

### Why not refactor

Refactoring `handle_h2_client_transcoding` into smaller functions is desirable but out of scope. The goal is to unblock CI with minimal risk. A refactoring change can follow.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Removing Block 2 might remove reachable code | Block 2 is after a `continue;` statement — unreachable |
| Adding `}` might create new brace issues | Verify with `cargo check -p scred-mitm --tests` |
