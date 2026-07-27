## Why

`tls_mitm.rs` (1726 lines) has a pre-existing brace imbalance in `handle_h2_client_transcoding` (662 lines). The function contains a duplicated dead code block (Block 2, lines 1411-1629) that mirrors Block 1 (lines 1047-1366) but with an extra `else` branch that breaks brace matching. The lib compiles because the extra `}` are balanced by missing `{` in the same function, but the test build fails. This blocks `just ci` and prevents validation of other changes.

Additionally, the file exceeds the 550-line limit (target 500 + 10% tolerance) and needs to be split into sub-modules.

## What Changes

- Fix brace imbalance in `handle_h2_client_transcoding` by removing duplicated dead code Block 2
- Extract response body forwarding logic into a helper function
- Split `tls_mitm.rs` into a directory with sub-modules to pass 550-line limit
- No behavioral changes — the refactoring is purely structural

## Capabilities

### New Capabilities
- `brace-fix`: Fix brace imbalance, remove dead code, extract helpers, split into sub-modules

### Modified Capabilities
<!-- No existing specs to modify -->

## Impact

- `crates/scred-mitm/src/mitm/tls_mitm.rs` — remove Block 2 dead code, extract helpers
- `crates/scred-mitm/src/mitm/tls_mitm/` — new directory with sub-modules
- `crates/scred-mitm/src/mitm/mod.rs` — update module declaration
