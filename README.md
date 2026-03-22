# SCRED

SCRED is a streaming secret redactor for three common workflows:

- **CLI**: clean logs, config dumps, and exported text before sharing
- **Proxy**: put SCRED in front of an internal or external HTTP service
- **MITM**: inspect and redact HTTPS traffic in controlled debugging environments

SCRED is designed to be as transparent as possible: keep traffic flowing, preserve structure, and only redact actual secrets.

## What it does

SCRED detects many common secret formats such as:
- cloud credentials
- API tokens
- bearer tokens
- private keys
- passwords embedded in URLs or config strings

Redaction is **character-preserving** so payload shape stays stable for downstream tools.

## Real-life style examples

### 1. Clean a support log before sending it

Input:
```text
Authorization: Bearer sk-proj-abcdefghijklmnopqrstuvwxyz123456
DATABASE_URL=postgres://app:supersecretpassword@db.internal:5432/prod
```

Output:
```text
Authorization: Bearer sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
DATABASE_URL=postgres://app:xxxxxxxxxxxxxxxxxxx@db.internal:5432/prod
```

### 2. Put SCRED in front of an internal API

You have a staging API that sometimes returns secrets in debug payloads. Run `scred-proxy` in front of it so clients keep using normal HTTP, but accidental secrets are removed.

### 3. Inspect HTTPS traffic during debugging

You need to reproduce an SDK issue against a third-party API, but you do not want tokens, cookies, or credentials appearing in captured traffic. Run `scred-mitm` locally and point your client to it.

## Binaries

- `scred` — CLI redactor
- `scred-proxy` — fixed-upstream reverse proxy with redaction
- `scred-mitm` — HTTPS MITM proxy for controlled debugging

## Quick start

### Build
```bash
cargo build --release
```

### CLI
```bash
echo 'Authorization: Bearer sk-proj-abcdefghijklmnopqrstuvwxyz123456' | \
  ./target/release/scred
```

### Reverse proxy
```bash
SCRED_PROXY_UPSTREAM_URL=https://httpbin.org \
  ./target/release/scred-proxy

curl http://127.0.0.1:9999/anything
```

### MITM proxy
```bash
./target/release/scred-mitm

curl -x http://127.0.0.1:8080 https://httpbin.org/anything -k
```

## Notes

- SCRED streams request and response bodies instead of buffering everything in memory.
- MITM use should stay limited to controlled development or debugging environments.
- Pattern quality matters. SCRED includes regression tests to catch false positives on normal web content.

## HTTP/2 Support

SCRED supports HTTP/2 clients and servers with native multiplexing support:

### Architecture
- Per-stream redaction state for proper isolation
- No downgrade: HTTP/2 clients stay on HTTP/2
- Native h2 multiplexing preserved
- See [HTTP2_DESIGN.md](HTTP2_DESIGN.md) for architecture details

### Current Status (Phase 1)
- ✅ Stream redaction state management
- ⏳ Client-side HTTP/2 handler (in progress)
- ⏳ Upstream HTTP/2 support (Phase 2)
- ⏳ Production testing and optimization (Phase 3)

### Testing HTTP/2 (Coming Soon)
```bash
./target/release/scred-mitm --listen 0.0.0.0:8443
curl --http2 --insecure https://localhost:8443/anything
```

