## Why

`tls_mitm.rs` (1726 lines) has a pre-existing brace imbalance in `handle_h2_client_transcoding` (662 lines). The `else if is_chunked` block (line 1274) is missing its closing `}`, causing the `}` at line 1367 to close both the `else if is_chunked` block and the `FrameType::Headers` arm. The lib compiles because extra `}` in a duplicated dead code block (Block 2, lines 1411-1629) balance the missing `{`, but the test build fails. This blocks `just ci` for the entire workspace.

## What Changes

- Add missing `}` to close the `else if is_chunked` block in `handle_h2_client_transcoding`
- Remove the duplicated dead code Block 2 (lines 1411-1629) that was compensating for the missing brace
- Verify `cargo check -p scred-mitm --tests` passes

## Capabilities

### New Capabilities
- `brace-fix`: Fix brace imbalance in tls_mitm.rs, remove compensating dead code

### Modified Capabilities
<!-- No existing specs to modify -->

## Impact

- `crates/scred-mitm/src/mitm/tls_mitm.rs` — add 1 `}`, remove ~220 lines of dead code
