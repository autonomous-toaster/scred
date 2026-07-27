## 1. Fix scred-http Compilation Errors

- [x] 1.1 Fix cached_dns_resolver.rs duplicate new() — rename no-arg to default(), impl Default
- [x] 1.2 Fix cached_dns_resolver.rs missing method on resolver
- [x] 1.3 Fix connection_pool.rs addr → _addr field mismatch

## 2. Fix scred-policy Clippy Warnings

- [x] 2.1 Fix engine/mod.rs: needless borrows, or_insert_with, for_kv_map, field_reassign
- [x] 2.2 Fix placeholder.rs: needless borrows, manual char comparison
- [x] 2.3 Fix streaming/mod.rs: needless borrows, explicit lifetimes

## 3. Fix Machete Unused Dependencies

- [x] 3.1 Remove truly unused dependencies from Cargo.toml files
- [x] 3.2 Add machete ignore sections for false positives

## 4. Integration Test

- [x] 4.1 Create tests/integration/proxy_body_redaction.sh (curl + assertions only)
- [x] 4.2 Rewrite just test-integration-proxy (podman setup in Justfile, call script)

## 5. Verify

- [ ] 5.1 Run just ci and confirm zero violations

## Notes

- `tls_mitm.rs` has a pre-existing brace imbalance in `handle_h2_client_transcoding`
  (662 lines) that blocks `cargo check -p scred-mitm`. This is a known issue
  that needs a dedicated refactoring effort.
- `just ci` will fail on `scred-mitm` until this is fixed.
- All other crates pass: check, lint, machete, file-sizes, veriplan.
