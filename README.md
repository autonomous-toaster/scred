# SCRED — Secret Detection and Redaction Engine

High-performance secret redaction system using Aho-Corasick automaton + memchr for deterministic O(n) detection. No regex. Streaming-first. Length-preserving redaction.

## Performance

### Detection Throughput (scaling)

| Benchmark | Throughput |
|-----------|------------|
| `detect_all_10kb` | ~860 MB/s |
| `detect_all_100kb` | ~830 MB/s |
| `detect_all_1mb` | ~790 MB/s |
| `detect_all_10mb` | ~720 MB/s |
| `detect_all_realistic_1mb` | ~250 MB/s |

### Redactor Throughput (1MB, 64KB chunks)

| Benchmark | Time | Throughput |
|-----------|------|------------|
| `redact_reader_to_writer/1kb` | ~30 ms | ~33 MB/s |
| `redact_reader_to_writer/64kb` | ~20 ms | ~50 MB/s |
| `redact_reader_to_writer/1024kb` | ~20 ms | ~50 MB/s |
| `pattern_density/none` | ~18 ms | ~56 MB/s |
| `pattern_density/sparse` | ~20 ms | ~50 MB/s |
| `pattern_density/dense` | ~20 ms | ~50 MB/s |
| `cross_boundary/aligned` | ~20 ms | ~50 MB/s |
| `cross_boundary/cross_boundary` | ~20 ms | ~50 MB/s |

### CLI Text Mode (ConfigurableEngine::detect_and_redact)

| Benchmark | Time | Throughput |
|-----------|------|------------|
| `cli_text_mode/1mb` | ~20 ms | ~50 MB/s |
| `cli_text_mode/10mb` | ~200 ms | ~50 MB/s |
| `cli_pattern_density/none` | ~18 ms | ~56 MB/s |
| `cli_pattern_density/sparse` | ~20 ms | ~50 MB/s |
| `cli_pattern_density/dense` | ~22 ms | ~46 MB/s |

### Proxy Forwarding (mocked I/O)

| Benchmark | Time |
|-----------|------|
| `proxy_forward_simple/1kb` | ~1.06 µs |

### MITM Certificate Generation

| Benchmark | Time |
|-----------|------|
| `mitm_cert_generation/cache_miss` | ~410 µs |
| `mitm_cert_generation/cache_hit` | ~167 ns |

### Reproduce

```sh
# Full benchmark suite
cargo bench --workspace

# Quick CI mode (reduced warmup/samples)
just bench-ci --quick

# Individual benchmarks
cargo bench -p scred-detector --bench scaling
cargo bench -p scred-detector --bench realistic
cargo bench -p scred-redactor --bench throughput
cargo bench -p scred --bench streaming
cargo bench -p scred-proxy --bench throughput
cargo bench -p scred-mitm --bench latency
```

All benchmarks build data **outside** `b.iter()` to avoid measuring allocation time. Data covers all 5 detection tiers (simple prefix, prefix validation, JWT, multiline markers, URI patterns).

## Quick Start

```sh
# CLI redaction
cat secrets.txt | scred

# With output file
cat secrets.txt | scred -o redacted.txt

# Select which patterns to detect/redact
env | scred --detect aws-*,github-* --redact CRITICAL
```

## Components

| Binary | Description |
|--------|-------------|
| `scred` | CLI — read stdin, redact, write stdout/file |
| `scred-proxy` | Forward HTTP proxy with body redaction |
| `scred-mitm` | MITM TLS proxy with policy support |

## CLI

```sh
scred [OPTIONS]

Options:
  --detect <TYPES>    Patterns to detect (default: ALL)
                      Glob: aws-*, github-*, sk-*, mysql*
                      Tiers: CRITICAL, API_KEYS
                      Combine: CRITICAL,mysql*,!test-*
  --redact <TYPES>    Patterns to redact (default: ALL)
  -o, --output <FILE> Write to file instead of stdout
  -v, --verbose       Show statistics
```

## Rust API

```rust
use std::sync::Arc;
use scred_redactor::{RedactionEngine, RedactionConfig, RedactionStream};

let engine = Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }));

// Streaming redaction
let mut stream = RedactionStream::new(engine.clone());
let out1 = stream.feed(b"some data AKIA");
let out2 = stream.feed(b"IOSFODNN7EXAMPLE more");
let (out3, stats) = stream.finalize();
```

## Proxy Body Redaction

Both `scred-proxy` and `scred-mitm` redact request and response bodies through `RedactionStream`. Bodies are processed in 64KB chunks with bounded memory.

```sh
# Forward proxy with body redaction
SCRED_PROXY_UPSTREAM_URL=https://api.example.com scred-proxy
```

## Pattern Count

**408 patterns** across 5 detection tiers:

| Tier | Type | Count | Example |
|------|------|-------|---------|
| 1 | Simple prefix | 26 | `AKIA` (AWS), `ghp_` (GitHub) |
| 2 | Prefix validation | 359 | `postgresql://`, `sk-proj-` |
| 3 | JWT | 1 | `eyJ...` base64url-encoded |
| 4 | Multiline markers | 7 | `-----BEGIN OPENSSH PRIVATE KEY-----` |
| 5 | URI patterns | 15 | `https://hooks.slack.com/`, `https://api.github.com/` |

```sh
scred --list-patterns
```

## Configuration

```yaml
# scred.yaml
policy:
  enabled: true
  seed: "${SCRED_POLICY_SEED}"
  providers:
    - type: env
      keys: ["*_API_KEY", "*_SECRET"]
  defaults:
    headers:
      Authorization: replace
      "*": redact
    body:
      request: redact
      response: redact
```

## Build

```sh
cargo build --release
cargo test --workspace
```

## License

MIT
