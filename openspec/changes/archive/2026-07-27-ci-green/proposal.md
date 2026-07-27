## Why

`just ci` is blocked by 5 pre-existing compilation errors in `scred-http`, 10 pre-existing clippy warnings in `scred-policy`, and 30+ unused dependencies reported by `cargo machete`. The proxy body redaction integration test exists as a Justfile recipe but is fragile (inline bash with `docker`/`nohup`/`sleep`) and not wired into CI. Fixing these makes CI green and the integration test robust.

## What Changes

- Fix 5 compilation errors in `scred-http` (cached_dns_resolver, connection_pool)
- Fix 10 clippy warnings in `scred-policy` (engine, placeholder, streaming)
- Remove truly unused dependencies, add machete ignore for false positives
- Create `tests/integration/proxy_body_redaction.sh` (curl + assertions only)
- Rewrite `just test-integration-proxy` to handle podman setup in Justfile, call script for checks
- Verify `just ci` passes cleanly

## Capabilities

### New Capabilities
- `pre-existing-fixes`: Fix compilation errors, clippy warnings, unused deps blocking CI
- `integration-test`: Robust proxy body redaction integration test with clean separation

### Modified Capabilities
<!-- No existing specs to modify -->

## Impact

- `crates/scred-http/src/cached_dns_resolver.rs` — fix duplicate new(), missing method
- `crates/scred-http/src/connection_pool.rs` — fix addr → _addr field mismatch
- `crates/scred-policy/src/engine/mod.rs` — fix 5 clippy warnings
- `crates/scred-policy/src/placeholder.rs` — fix 3 clippy warnings
- `crates/scred-policy/src/streaming/mod.rs` — fix 2 clippy warnings
- Multiple `Cargo.toml` files — remove unused deps, add machete ignore sections
- `tests/integration/proxy_body_redaction.sh` — new file (curl + assertions)
- `Justfile` — rewrite test-integration-proxy recipe
