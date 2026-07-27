## 1. CI Quality Gates (P0)

- [x] 1.1 Fix Justfile check-file-sizes path (sift/src → crates/)
- [x] 1.2 Add bench recipe to Justfile
- [x] 1.3 Add --all-features to check recipe
- [x] 1.4 Add workspace lints to Cargo.toml
- [x] 1.5 Fix .expect()/.unwrap() in production code
- [x] 1.6 Remove REGEX_PATTERN_COUNT dead code, compute counts from arrays
- [x] 1.7 Consolidate pattern selector parsing — CLI uses library function
- [x] 1.8 Add fmt check to Justfile

## 2. Benchmark Suite

- [x] 2.1 Rewrite scaling benchmark — data outside b.iter(), cover all 5 tiers
- [x] 2.2 Rewrite realistic benchmark — data outside b.iter(), cover all pattern types
- [x] 2.3 Add RedactionStream benchmark
- [x] 2.4 Remove broken pattern_frequency and empty simd_benchmark
- [x] 2.5 Add CI benchmark gate to Justfile
- [x] 2.6 Add benchmark regression threshold (5%)

## 3. Proxy Body Redaction (CRITICAL)

- [x] 3.1 Wire RedactionStream into scred-proxy body forwarding path
- [x] 3.2 Verify proxy body redaction with integration test

## 4. CLI Improvements

- [x] 4.1 Add --output / -o flag to scred-cli
- [x] 4.2 Remove tier concept from CLI help (replace with glob examples)

## 5. README Rewrite

- [x] 5.1 Rewrite README with proof of performance, reproduction steps, accurate examples
