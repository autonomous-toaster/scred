## Why

The codebase has accumulated dead code, missing lint rules, a broken Justfile (copied from another project), duplicated pattern selector parsing, `.expect()`/`.unwrap()` calls in production code, a critical missing body redaction path in the forward proxy, and benchmarks that don't cover all patterns. These issues make the code harder to maintain, less safe, and less trustworthy. Fixing them is P0 because they directly impact code quality, developer experience, and correctness.

## What Changes

- **P0**: Fix Justfile — correct paths (`sift/src` → `crates/`), add `bench` recipe, add `--all-features` to `check`, add benchmark CI gate
- Add workspace lints — `unsafe_code = "forbid"`, `unwrap_used`/`expect_used` = `"deny"`, clippy groups
- Remove dead code — `REGEX_PATTERN_COUNT`, `regex_patterns.rs` reference, compute pattern counts from array lengths
- Fix `.expect()`/`.unwrap()` in production code — propagate errors instead
- **CRITICAL**: Fix proxy body redaction — wire `RedactionStream` into `scred-proxy`
- Consolidate pattern selector parsing — CLI uses library `PatternSelector::from_str()`
- Add `--output` / `-o` flag to CLI
- Remove tier concept — replace with pattern name/prefix glob filtering (already supported)
- Rewrite benchmarks — cover all 408 patterns, fix data-in-b.iter() anti-pattern, remove dead benches
- Rewrite README — with proof of performance, reproduction steps, accurate examples

## Capabilities

### New Capabilities
- `ci-quality-gates`: Justfile fixes, workspace lints, benchmark CI gate — ensures code quality is enforced automatically
- `benchmark-suite`: Comprehensive benchmarks covering all 408 patterns, detection + redaction + streaming, with CI baseline comparison
- `proxy-body-redaction`: Wire RedactionStream into scred-proxy so request/response bodies are redacted

### Modified Capabilities
<!-- No existing specs to modify -->

## Impact

- `Justfile` — full rewrite of `check-file-sizes`, add `bench`, fix paths
- `Cargo.toml` — add `[workspace.lints]` section
- `crates/scred-detector/src/patterns.rs` — remove `REGEX_PATTERN_COUNT`, compute counts from arrays
- `crates/scred-detector/src/detector.rs` — fix `.expect()` calls
- `crates/scred-detector/src/uri_patterns.rs` — fix `.unwrap()` calls
- `crates/scred-proxy/src/main.rs` — add body redaction via RedactionStream
- `crates/scred-cli/src/main.rs` — use library PatternSelector, add --output flag
- `crates/scred-detector/benches/` — rewrite scaling, realistic, remove broken benches
- `README.md` — full rewrite with proof, reproduction steps
