# SCRED - Secret Detection and Redaction Engine

Fast, safe secret detection and redaction for logs, configs, and network traffic.

## Components

| Binary | Description |
|--------|-------------|
| `scred` | CLI tool for redacting files and streams |
| `scred-proxy` | Reverse proxy with secret detection/redaction |
| `scred-mitm` | MITM proxy for TLS interception and filtering |

## Quick Start

```bash
# Build everything
cargo build --release

# CLI redaction
cat secrets.txt | scred

# Generate TLS certificates for MITM
scred-mitm --generate-certs

# Run MITM proxy
scred-mitm

# Run reverse proxy
SCRED_PROXY_UPSTREAM_URL=https://api.openai.com scred-proxy
```

## Compilation Options

### scred-proxy

```bash
# Default: redaction enabled
cargo build -p scred-proxy --release

# Forward-only (no detection, minimal overhead)
cargo build -p scred-proxy --release --no-default-features

# With policy (placeholder-based secret injection)
cargo build -p scred-proxy --release --features policy
```

### scred-mitm

```bash
# Default: redaction + traffic filtering
cargo build -p scred-mitm --release

# No redaction (forward-only MITM)
cargo build -p scred-mitm --release --no-default-features

# With policy
cargo build -p scred-mitm --release --features policy

# Without traffic filtering
cargo build -p scred-mitm --release --no-default-features --features redaction
```

### Feature Summary

| Feature | scred-proxy | scred-mitm | Description |
|---------|-------------|------------|-------------|
| `redaction` | default | default | Secret detection and redaction |
| `policy` | optional | optional | Placeholder-based secret injection |
| `traffic-filter` | - | default | Domain whitelist/blacklist |

## Configuration

### Unified Configuration Structure

```yaml
# scred.yaml

# Global Policy Configuration (Placeholder Replacement)
policy:
  enabled: true
  seed: "${SCRED_POLICY_SEED}"  # Env var expansion

  # Discovery API for containers
  discovery:
    enabled: true
    port: 9998
    path: /placeholders

  # Default providers
  providers:
    - type: env
      keys: ["*_API_KEY", "*_SECRET", "*_TOKEN"]

  # Which patterns get placeholders
  patterns: ["*"]

  # WHERE to replace placeholders with real secrets
  location:
    request_body: true
    request_headers: true
    response_body: true
    response_headers: true
    keep_headers: []      # Headers to keep as placeholders
    replace_headers: []   # Headers to force replace

# Global Redaction Configuration
redaction:
  mode: redact  # detect | redact | passthrough

  patterns:
    redact: ["*"]         # Redact all
    keep: ["public-*"]    # Keep public patterns visible

# Per-Host Policies
policies:
  # OpenAI: Replace in body, keep headers as placeholders
  - hosts: ["*.openai.com"]
    policy:
      providers:
        - type: env
          keys: ["OPENAI_API_KEY"]
      patterns: ["openai-*"]
      location:
        request_body: true
        request_headers: false  # Keep Authorization header as placeholder
        keep_headers: ["Authorization"]

  # AWS: Replace everywhere
  - hosts: ["*.amazonaws.com"]
    policy:
      providers:
        - type: env
          keys: ["AWS_*"]
      patterns: ["aws-*"]
    redaction:
      patterns:
        redact: ["*"]
        keep: []

  # Development: No redaction
  - hosts: ["localhost", "*.local"]
    redaction:
      mode: passthrough

scred-mitm:
  listen:
    port: 8080
  traffic:
    enabled: false
    allowed-domains: ["*"]
  ca-cert:
    generate: true
    cache-dir: /tmp/scred-certs

scred-proxy:
  listen:
    port: 9999
  upstream:
    url: "https://api.openai.com"
```

### Pattern Filter Semantics

```yaml
# Redact all patterns
patterns:
  redact: ["*"]

# Redact only AWS patterns
patterns:
  redact: ["aws-*"]

# Redact all except AWS (negation)
patterns:
  redact: ["!aws-*", "*"]

# Keep public patterns visible
patterns:
  redact: ["*"]
  keep: ["public-*"]

# Redact AWS and GitHub only
patterns:
  redact: ["aws-*", "github-*"]
```

### Merge Strategies

```yaml
# Merge: combine redact/keep lists (default)
- hosts: ["*.custom.com"]
  redaction:
    patterns:
      keep: ["gitlab-*"]  # Adds to default keep list
    merge: merge
  # Result: keep: ["public-*", "gitlab-*"]

# Replace: use override completely
- hosts: ["*.openai.com"]
  redaction:
    patterns:
      redact: ["openai-*"]
      keep: []
    merge: replace
  # Result: redact: ["openai-*"], keep: []
```

## Secret Patterns

| Pattern Name | Description |
|--------------|-------------|
| `aws-access-key` | AWS Access Key ID |
| `aws-secret-key` | AWS Secret Access Key |
| `github-token` | GitHub Personal Access Token |
| `gitlab-token` | GitLab Personal Access Token |
| `openai-api-key` | OpenAI API Key |
| `stripe-api-key` | Stripe API Key |
| `jwt-secret` | JWT Secret Key |
| `private-key` | Private Key (PEM format) |
| `password` | Password in config files |
| `api-key` | Generic API Key |

```bash
# List all patterns
scred --list-patterns

# Use glob patterns
patterns:
  redact: ["aws-*"]      # All AWS patterns
  keep: ["*-token"]      # All tokens
```

## Redaction Modes

| Mode | Detection | Redaction | Use Case |
|------|-----------|-----------|----------|
| `detect` | ✓ | ✗ | Auditing, compliance |
| `redact` | ✓ | ✓ | Production, security |
| `passthrough` | ✗ | ✗ | Debugging, trusted networks |

## Policy-Based Secret Injection

Containers receive placeholders instead of real secrets:

```bash
# 1. Set secrets in proxy environment
export OPENAI_API_KEY=sk-proj-xxx

# 2. Container fetches placeholders
curl http://scred-policy:9998/placeholders
# Returns: OPENAI_API_KEY=<placeholder-OPENAI_API_KEY>

# 3. Proxy replaces placeholders with real values
```

## Generate Configuration

```bash
# Generate example config
scred-proxy --generate-config > scred.yaml
scred-mitm --generate-config > scred.yaml

# Generate TLS certificates
scred-mitm --generate-certs
```

## Architecture

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────┐
│    scred-proxy / scred-mitm          │
│  ┌─────────────────────────────┐    │
│  │   Traffic Filter (MITM)     │    │
│  │   - Block non-whitelisted   │    │
│  │   - Glob pattern matching   │    │
│  └──────────┬──────────────────┘    │
│             │                        │
│  ┌──────────▼──────────────────┐    │
│  │      Policy (optional)      │    │
│  │   - Replace placeholders    │    │
│  │   - O(n) Aho-Corasick       │    │
│  └──────────┬──────────────────┘    │
│             │                        │
│  ┌──────────▼──────────────────┐    │
│  │    Redaction (optional)     │    │
│  │   - Detect 372+ patterns    │    │
│  │   - Character-preserving    │    │
│  │   - Streaming, zero-copy    │    │
│  └──────────┬──────────────────┘    │
└─────────────┼───────────────────────┘
              │
              ▼
       ┌─────────────────┐
       │    Upstream      │
       │ (API / Service) │
       └─────────────────┘
```

## Performance

- **Detection**: <100μs per 10KB
- **Redaction**: <100ms per 1MB
- **Memory**: <64KB streaming
- **Throughput**: 102+ MB/s

## Building

```bash
# Development
cargo build

# Release (recommended)
cargo build --release

# Run tests
cargo test --all

# Specific features
cargo test -p scred-policy
cargo test -p scred-proxy --features policy
```

## License

MIT
