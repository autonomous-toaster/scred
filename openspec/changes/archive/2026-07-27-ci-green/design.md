## Context

The codebase-hardening change completed 19/19 tasks but `just ci` still fails due to pre-existing issues in `scred-http` and `scred-policy` that were outside the original scope. These are now the only blockers to a green CI.

## Goals / Non-Goals

**Goals:**
- `just ci` passes with zero violations
- Integration test for proxy body redaction is robust and automated
- Machete reports zero unused dependencies (or properly configured exceptions)

**Non-Goals:**
- Feature work
- Performance optimization
- Architecture changes

## Decisions

### scred-http compilation errors

| File | Error | Fix |
|------|-------|-----|
| `cached_dns_resolver.rs:50,58` | Duplicate `new()` methods | Rename no-arg `new()` to `default()`, implement `Default` trait |
| `cached_dns_resolver.rs:58` | `default()` method name conflict | Remove manual `default()`, use `#[derive(Default)]` or explicit impl |
| `cached_dns_resolver.rs:263` | Missing method on resolver | Add the missing method or fix the call site |
| `connection_pool.rs:33` | Field `addr` doesn't exist | Constructor assigns `_addr: addr` instead of `addr,` |

### scred-policy clippy warnings

All 10 warnings are small fixes:
- `needless_borrows_for_generic_args` (4x): `&[""]` → `[""]`
- `manual_pattern_char_comparison` (2x): `|c| c == '_' \|\| c == '-'` → `['_', '-']`
- `for_kv_map` (1x): iterate `.values()` instead of key-value
- `unwrap_or_default` (1x): `.or_insert_with(Vec::new)` → `.or_default()`
- `field_reassign_with_default` (2x): use struct literal with `..Default::default()`
- `needless_borrows_for_generic_args` (1x): `&hash` → `hash`
- `explicit_lifetime` (1x): elide explicit lifetime

### Machete unused dependencies

Two categories:
1. **Truly unused** — remove from `Cargo.toml` (20 deps across 9 crates)
2. **False positives** — add `[package.metadata.cargo-machete] ignored = [...]` to `Cargo.toml` (12 deps across 6 crates)

### Integration test architecture

```
Justfile (orchestration)
  ├── podman run -d --rm --name httpbin-integration ... httpbin
  ├── cargo build --release -p scred-proxy
  ├── SCRED_PROXY_UPSTREAM_URL=... ./target/release/scred-proxy &
  ├── tests/integration/proxy_body_redaction.sh  (curl + assertions)
  └── podman stop httpbin-integration; kill $PROXY_PID
```

The shell script is minimal — only curl requests and response assertions. All environment setup stays in the Justfile.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Removing a dependency that's needed transitively | Test compile after each removal |
| Machete false positives change with crate updates | Review machete config periodically |
| Integration test requires podman | Guard with `command -v podman` in Justfile |
