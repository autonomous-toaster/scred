# SCRED - Secret Detection and Redaction Engine

Best effort secret redaction system without regex and streaming in mind.

## Features

- **raw string redaction**
- **Unified Policy System**: Per-header action control (replace, redact, detect, passthrough)
- **Placeholder Replacement**: Never expose real secrets - use deterministic placeholders
- **Secret Redaction**: Detect and redact 52+ secret patterns (AWS, GitHub, OpenAI, etc.)
- **Host-Specific Rules**: Different policies for different domains
- **Streaming Performance**: O(n) processing with bounded memory
- **Discovery API**: Containers fetch placeholders via HTTP

## Components

| Binary | Description |
|--------|-------------|
| `scred` | CLI tool for redacting files and streams |
| `scred-proxy` | Reverse proxy with secret detection/redaction |
| `scred-mitm` | MITM proxy for TLS interception and filtering |


The goal of `scred-mitm` is to avoid accidental secret leaks by an AI agent, not to fight against data exfiltraion.

## Quick Start

```bash
# Build everything
cargo build --release

# CLI redaction
cat secrets.txt | scred

# Generate TLS certificates for MITM
scred-mitm --generate-certs

# Run MITM proxy with unified policy
export OPENAI_API_KEY=sk-proj-xxx
export SCRED_POLICY_SEED=my-seed
scred-mitm
```

## Unified Policy System

SCRED uses a unified policy system that combines:

1. **Placeholder Replacement**: Replace `{prefix}-scrd-{hex}` with real secrets
2. **Secret Redaction**: Detect and redact secrets in HTTP traffic

### Per-Header Action Control

Different actions for different headers:

```yaml
policy:
  defaults:
    headers:
      Authorization: replace      # Placeholder → secret
      "X-Api-Key": replace        # Placeholder → secret
      "X-Public-*": passthrough   # Don't touch public headers
      "*": redact                 # Redact all other headers
    body:
      request: redact
      response: redact
```

### Actions

| Action | Headers | Body | Description |
|--------|---------|------|-------------|
| `replace` | ✅ | ❌ | Replace placeholders with real secrets |
| `redact` | ✅ | ✅ | Replace detected secrets with `[REDACTED]` |
| `detect` | ✅ | ✅ | Log detections without modifying |
| `passthrough` | ✅ | ✅ | No processing |

## Configuration

### Minimal Configuration

```yaml
# scred-mitm.yaml
policy:
  enabled: true
  seed: "${SCRED_POLICY_SEED}"
  
  providers:
    - type: env
      keys: ["*_API_KEY", "*_SECRET", "*_TOKEN"]
  
  discovery:
    enabled: true
    port: 9998
```

### Host-Specific Rules

```yaml
policy:
  defaults:
    headers:
      Authorization: replace
      "*": redact
    body:
      request: redact
      response: redact

hosts:
  "*.openai.com":
    headers:
      Authorization: replace  # sk-scred-xxx → sk-proj-xxx
      "*": redact
  
  "*.internal.company.com":
    merge: replace
    headers:
      "*": passthrough  # Trust internal services
```

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for full reference.

## Compilation Options

### scred-proxy

```bash
cargo build -p scred-proxy --release

### scred-mitm

```bash
cargo build -p scred-mitm --release
```

## Security

### Host Validation

- CONNECT destination is authoritative for domain restrictions
- Host header is NOT used for policy matching
- This prevents spoofing attacks

### Audit Trail

All actions are logged:

```
[unified] Replaced 1 placeholder(s) in header: Authorization
[unified] Redacted 2 secret(s) in header: X-Custom
[unified] Detected AWS_SECRET in header: X-Debug
```

## Development

### Run Tests

```bash
# All tests
cargo test --all

# Unified policy tests
cargo test -p scred-config unified_policy
cargo test -p scred-policy unified_engine
cargo test -p scred-mitm --features policy unified_integration
```

### Build Docker Images

```bash
# Build minimal scratch images
podman build -f Dockerfile.scred-mitm -t scred-mitm:latest .
podman build -f Dockerfile.scred-proxy -t scred-proxy:latest .
```

## License

MIT

# Global Redaction Configuration
```yaml
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
