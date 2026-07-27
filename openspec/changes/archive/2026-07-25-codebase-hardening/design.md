## Context

The codebase has accumulated technical debt: a Justfile copied from another project with wrong paths, no workspace lints, `.expect()`/`.unwrap()` calls in production code, dead code (unimplemented regex patterns), duplicated pattern selector parsing, a critical missing body redaction path in the forward proxy, and benchmarks that don't cover all patterns.

## Goals / Non-Goals

**Goals:**
- Fix Justfile to work correctly for this project (P0)
- Add workspace lints to enforce code quality automatically
- Remove all `.expect()`/`.unwrap()` from production code
- Remove dead code (REGEX_PATTERN_COUNT, broken benchmarks)
- Wire body redaction into scred-proxy
- Consolidate pattern selector parsing into the library
- Rewrite benchmarks to cover all 408 patterns with correct methodology
- Rewrite README with proof of performance and reproduction steps

**Non-Goals:**
- Performance optimization (benchmarks measure current state)
- New features beyond what's listed
- Architecture changes beyond what's listed

## Decisions

### Justfile fixes

The `check-file-sizes` recipe currently searches `sift/src` and `sift-core/src` — paths from the original project this Justfile was copied from. These must be changed to `crates/`. The `check` recipe needs `--all-features` to catch feature-gated compilation errors. A `bench` recipe must be added.

### Workspace lints

Following the baish project's pattern: `unsafe_code = "forbid"`, `unwrap_used = "deny"`, `expect_used = "deny"`, and clippy groups (`all`, `pedantic`, `nursery`) all set to `"deny"`. These are set at the workspace level so all crates inherit them.

### Pattern counts

Replace individual `*_COUNT` constants with `.len()` calls on the actual arrays. This eliminates the risk of stale counts.

### Proxy body redaction

`scred-proxy` currently only does header placeholder replacement. The body is forwarded without redaction. The fix: create a `RedactionStream` from the engine and feed the body through it before writing to upstream, and feed the response body through it before writing to client.

### Benchmark methodology

All benchmarks must construct data outside `b.iter()` to avoid measuring allocation time. Each benchmark must cover all 5 detection tiers. Dead/empty benchmarks are removed.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Adding workspace lints may break CI | Fix violations first, then add lints |
| Proxy body redaction may add latency | RedactionStream is zero-copy; overhead is minimal |
| Benchmark rewrite may change numbers | Keep old benchmarks until new ones are validated |
