## 1. Redactor Bench Infrastructure

- [x] 1.1 Evaluate 9 orphaned bench files in `crates/scred-redactor/benches/` — compile each, remove broken ones
- [x] 1.2 Add `[[bench]]` entries to `crates/scred-redactor/Cargo.toml` for viable bench files
- [x] 1.3 Verify `cargo bench -p scred-redactor` runs and produces results

## 2. Redactor Throughput Benchmark

- [x] 2.1 Create `crates/scred-redactor/benches/throughput.rs` with Criterion harness
- [x] 2.2 Implement `build_realistic_data()` with varying pattern densities (none, sparse, dense)
- [x] 2.3 Implement benchmark for `redact_reader_to_writer` with 1KB, 64KB, 1MB chunks
- [x] 2.4 Implement benchmark for `process_chunk` with cross-boundary secrets
- [x] 2.5 Verify benchmarks compile and produce meaningful results

## 3. CLI Streaming Benchmark

- [x] 3.1 Add `criterion` dev-dependency to `crates/scred-cli/Cargo.toml`
- [x] 3.2 Create `crates/scred-cli/benches/streaming.rs` with Criterion harness
- [x] 3.3 Implement benchmark for `stream_and_redact()` with piped stdin data
- [x] 3.4 Implement benchmark with varying pattern densities
- [x] 3.5 Verify benchmarks compile and produce meaningful results

## 4. Proxy Throughput Benchmark

- [x] 4.1 Add `criterion` dev-dependency to `crates/scred-proxy/Cargo.toml`
- [x] 4.2 Create `crates/scred-proxy/benches/throughput.rs` with Criterion harness
- [x] 4.3 Implement benchmark for `forward_simple` with mocked I/O (`tokio::io::duplex()`)
- [x] 4.4 Implement benchmark for `forward_with_policy` with disabled policy engine
- [x] 4.5 Implement benchmark with varying body sizes (1KB, 100KB, 1MB)
- [x] 4.6 Verify benchmarks compile and produce meaningful results

## 5. MITM Latency Benchmark

- [x] 5.1 Add `criterion` dev-dependency to `crates/scred-mitm/Cargo.toml`
- [x] 5.2 Create `crates/scred-mitm/benches/latency.rs` with Criterion harness
- [x] 5.3 Implement benchmark for `handle_single_request` with mocked TLS stream
- [x] 5.4 Implement benchmark for `handle_h2_downgrade` with H2 preface
- [x] 5.5 Implement benchmark for `CertificateGenerator::get_or_generate_cert` (cache hit vs miss)
- [x] 5.6 Verify benchmarks compile and produce meaningful results

## 6. CI Integration

- [x] 6.1 Run all benchmarks and save baseline to `.benchmark-baseline.json`
- [x] 6.2 Update `just bench-ci` to compare against baseline with 5% threshold
- [x] 6.3 Add `--quick` mode support for faster CI runs
- [x] 6.4 Verify `just bench-ci` passes with no regressions
- [x] 6.5 Verify `just bench-ci` fails when a regression is introduced
