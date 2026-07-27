## Context

`tls_mitm.rs` was identified during codebase hardening as exceeding the 550-line file size limit (1726 lines). The file has a pre-existing brace imbalance in `handle_h2_client_transcoding` (662 lines) caused by a duplicated dead code block. The lib compiles but the test build fails, blocking CI.

## Goals / Non-Goals

**Goals:**
- Fix brace imbalance so `cargo test -p scred-mitm` passes
- Remove duplicated dead code (Block 2, lines 1411-1629)
- Extract response body forwarding into a helper function
- Split `tls_mitm.rs` into sub-modules under 550 lines each
- No behavioral changes

**Non-Goals:**
- Refactoring the H2 transcoding logic itself
- Adding new features
- Performance optimization

## Decisions

### Brace fix strategy

The root cause is a duplicated Block 2 (lines 1411-1629) that mirrors Block 1 (lines 1047-1366) but has an extra `} else {` branch. Block 2 is dead code — it's never reached because the function returns before it. The fix: remove Block 2 entirely. This balances the braces without needing to understand the full function logic.

### Helper extraction

The response body forwarding logic (lines 1244-1366 in Block 1) is a self-contained unit that handles content-length, chunked, and until-EOF body forwarding. Extract it into `forward_response_body()` to reduce the main function size and isolate the brace issue.

### Module structure

```
tls_mitm/
├── mod.rs          # Re-exports, module declarations
├── handler.rs      # handle_h2_client_transcoding (after extraction)
├── helpers.rs      # forward_response_body, send_h2_error_response, encode helpers
└── tests.rs        # Test functions
```

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Removing Block 2 might remove reachable code | Verify with code review — Block 2 is inside the same function after a `return` statement |
| Extracting helpers might introduce bugs | Keep extraction minimal, test after each step |
| Splitting might break module structure | Update `mod.rs` declarations and imports |
