## Context

The codebase has 5 working Criterion benchmarks in `scred-detector`, 9 orphaned bench files in `scred-redactor` (no `[[bench]]` entries in Cargo.toml), and zero benchmarks for CLI, proxy, or MITM crates. The `just bench-ci` command exists but has no baseline to compare against.

Existing benchmark pattern (from `scred-detector`):
- Criterion harness with `harness = false`
- Realistic data generation functions
- `criterion_group!` + `criterion_main!` macros
- Stored in `crates/<name>/benches/`

## Goals / Non-Goals

**Goals:**
- Wire up all viable orphaned redactor bench files
- Add Criterion benchmarks for redactor throughput, CLI streaming, proxy throughput, and MITM latency
- Use mocked I/O (`tokio::io::duplex()`) for proxy and MITM benchmarks to avoid network flakiness
- Use realistic data patterns (reuse detector's `build_realistic_data()` approach)
- Update `just bench-ci` to store and compare against a baseline with 5% regression threshold
- All benchmarks MUST pass in CI (no flaky benchmarks)

**Non-Goals:**
- Real TCP/TLS benchmarks with network I/O (too flaky for CI)
- Micro-benchmarks of individual functions (focus on end-to-end scenarios)
- Benchmarking config loading, policy resolution, or other cold paths

## Decisions

### Decision 1: Criterion over Divan
Criterion is already used by the 5 working detector benchmarks. Stick with it for consistency. Divan is newer but would add a second benchmarking framework.

### Decision 2: Mocked I/O for proxy and MITM
Use `tokio::io::duplex()` — already proven in the test suite (15+ tests use it). This gives reproducible, fast benchmarks without network variability. Real TCP benchmarks can be added later as a separate effort.

### Decision 3: Realistic data over synthetic
Reuse the detector's `build_realistic_data()` pattern: generate data with actual secret patterns (AWS keys, GitHub tokens, JWTs, SSH keys, database URIs) mixed with normal log content. This gives meaningful throughput numbers.

### Decision 4: Phase ordering
```
Phase 1: Wire up redactor benches (quick wins, 9 files → compile + run)
Phase 2: Redactor throughput benchmark (new bench, core bottleneck)
Phase 3: CLI streaming benchmark (E2E, most impactful user-facing)
Phase 4: Proxy throughput benchmark (mocked I/O)
Phase 5: MITM latency benchmark (mocked TLS)
Phase 6: Update bench-ci with baseline
```

### Decision 5: Remove, don't fix, broken orphaned benches
Some orphaned redactor bench files may not compile or may test removed APIs. Rather than fixing them, remove them. Only wire up the ones that compile and test current code.

## Risks / Trade-offs

- **[Risk]** Mocked I/O may not reflect real network performance → **[Mitigation]** Document that benchmarks measure redaction logic, not network throughput. Real TCP benchmarks are a future enhancement.
- **[Risk]** Criterion benchmarks can be slow in CI (3s warmup per benchmark) → **[Mitigation]** Use `--quick` mode in CI, full mode for local profiling.
- **[Risk]** Baseline file goes out of date as hardware changes → **[Mitigation]** Document how to regenerate baseline. Accept that CI may need occasional baseline updates.
- **[Risk]** Some orphaned redactor benches test removed APIs → **[Mitigation]** Remove them rather than fixing. Only wire up what compiles.
