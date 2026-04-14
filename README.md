# SCRED - Secret Detection and Redaction Engine

Best effort secret redaction system without regex, lenght preservation, and streaming in mind.

Totally vibe coded, don't use it if your life depend on it.

## Features

- **raw string redaction**
- **Unified Policy System**: Per-header action control (replace, redact, detect, passthrough)
- **Placeholder Replacement**: Never expose real secrets - use deterministic placeholders
- **Secret Redaction**: Detect and redact 52+ secret patterns (AWS, GitHub, OpenAI, etc.)
- **Host-Specific Rules**: Different policies for different domains
- **Streaming Performance**: O(n) processing with bounded memory
- **Discovery API**: Containers fetch placeholders via HTTP


## scred-cli

Read stdin, redact, write stdout.

```sh
env | grep API | scred

AZURE_OPENAI_API_KEY=ex0wxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
CONTEXT7_API_KEY=ctx7xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
EXA_API_KEY=71b3xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
LANGSMITH_API_KEY=lsv2xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
LITELLM_API_KEY=sk-Exxxxxxxxxxxxxxxxxxxxx
MCP_OPEN_API_KEY=sk-Cxxxxxxxxxxxxxxxxxxxxx
METABASE_ADMIN_API_KEY=mb_Dxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
METABASE_API_KEY=mb_Dxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
MISTRAL_API_KEY=YV3Sxxxxxxxxxxxxxxxxxxxxxxxxxxxx
NVIDIA_NIM_API_KEY=nvapxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
OPENAI_API_KEY=sk-fxxxxxxxxxxxxxxxxxxx
SOME_OTHER_OPENAI_API_KEY=sk-pxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
SERPER_API_KEY=b087xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
TAVILY_API_KEY=tvlyxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

## mitm lab

```sh
podman compose build
podman compose up
```


Use `127.0.0.1:9999` as proxy for curl.

Example :

export OPENAI_API_KEY="sk-fake-key-for-testing"

curl --cacert ./data/scred-mitm/ca-cert.pem -x 127.0.0.1:9999 https://httpbin.org/anything -H "x-something: $OPENAI_API_KEY" -H "Authorization: $OPENAI_API_KEY"


```json
{
  "args": {},
  "data": "",
  "files": {},
  "form": {
    "some": "sk-fxxxxxxxxxxxxxxxxxxx" # Secret is redacted from body form data
  },
  "headers": {
    "Accept": "*/*",
    "Authorization": "sk-fxxxxxxxxxxxxxxxxxxx", # Authorization header is REDACTED (per-host override)
    "Host": "httpbin.org",
    "User-Agent": "curl/8.7.1",
    "X-Amzn-Trace-Id": "Root=1-69d903ea-7ef9e2cd1528443b3fb34073",
    "X-Something": "sk-fxxxxxxxxxxxxxxxxxxx" # All headers are redacted
  },
  "json": null,
  "method": "GET",
  "origin": "x.y.z.a",
  "url": "https://httpbin.org/anything"
}
```

Open `http://localhost:8081` to inspect the proper redaction in `mitmweb` (pasword: `password`)

## Policy-Based Secret Injection

scred-mitm expose the placeholders for known secrets that will be visible bu the agent as placeholder, and replaced on the fly while streaming the request to the upstream

```sh
# expose real key to scred-mitm
export OPENAI_API_KEY="sk-fake-key-for-testing"
podman compose up
```

```sh
curl -s http://127.0.0.1:9998/placeholders
OPENAI_API_KEY=sk-fake-scrd-7566da4420
```

```sh
export $(curl -s http://127.0.0.1:9998/placeholders)

echo $OPENAI_API_KEY
# same length placeholder
echo $OPENAI_API_KEY
sk-fake-scrd-7566da4420

curl --cacert ./data/scred-mitm/ca-cert.pem -x 127.0.0.1:9999 https://httpbin.org/anything -H "x-something: $OPENAI_API_KEY" -H "Authorization: $OPENAI_API_KEY"
```

```json
{
  "args": {},
  "data": "",
  "files": {},
  "form": {},
  "headers": {
    "Accept": "*/*",
    "Authorization": "sk-fake-key-for-testing",
    "Host": "httpbin.org",
    "User-Agent": "curl/8.7.1",
    "X-Amzn-Trace-Id": "Root=1-69d93a99-2966910a60dcef0066bc84e0",
    "X-Something": "sk-fxxxxxxxxxxxxxxxxxxx"
  },
  "json": null,
  "method": "GET",
  "origin": "x.y.z.a",
  "url": "https://httpbin.org/anything"
}
```

Open `http://localhost:8081` to inspect the proper redaction / placeholder replacements in `mitmweb` (pasword: `password`)


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

# Global Redaction Configuration
```yaml
policy:
  enabled: true
  seed: "${SCRED_POLICY_SEED}"
  
  # Secret providers - glob patterns match environment variables
  providers:
    - type: env
      keys: ["*_API_KEY", "*_SECRET", "*_TOKEN"]
  
  # Discovery API - containers fetch their placeholders
  # Endpoint: http://proxy:9998/placeholders
  discovery:
    enabled: true
    port: 9998
  
  # Default processing rules
  defaults:
    headers:
      Authorization: replace    # Replace placeholders with real secrets
      "X-Api-Key": replace
      "*": redact               # Redact detected secrets in other headers
    body:
      request: redact
      response: redact
    patterns:
      redact: ["*"]
      keep: []

# MITM Proxy Configuration
scred-mitm:
  listen:
    port: 8080
    address: "0.0.0.0"
  
  # CA certificate for TLS interception
  ca-cert:
    cert-path: ./data/ca-cert.pem
    key-path: ./data/ca-key.pem
  
  # Traffic filtering (default-deny)
  traffic:
    mode: allow-list
    allowed-domains:
      - "*.openai.com"
      - "*.anthropic.com"
      - "*.api.nvidia.com"
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
