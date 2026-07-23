# SCRED Architecture

SCRED is a Rust secret detection and redaction proxy. It detects 300+ secret patterns (API keys, tokens, passwords, SSH keys, JWTs, certificates, PGP keys, database URIs, webhooks) without regex, processes data in streaming fashion, and operates as both a forward HTTP proxy and MITM TLS proxy.

## Crate Dependency Graph

```
scred-detector     (pure detection: Aho-Corasick, memchr, charset LUTs, prefix index)
       ↑
scred-redactor     (RedactionEngine + StreamingRedactor wrapping detector)
       ↑
┌──────┴──────┐
scred-http      scred-policy     (placeholder replacement engine)
(HTTP lib)      (policy engine)
       ↑              ↑
┌──────┴──────────────┴──────┐
scred-mitm    scred-proxy    scred-cli
(MITM TLS)    (forward)      (CLI tool)
       ↑
scred-config   (shared configuration)
```

### Crate Responsibilities

| Crate | Role |
|-------|------|
| `scred-detector` | Pure pattern detection. No I/O. Defines all pattern types and detection algorithms. |
| `scred-redactor` | Wraps detector with RedactionEngine (full-document) and StreamingRedactor (chunk-by-chunk). Adds redaction logic. |
| `scred-http` | HTTP library: parsing, streaming request/response handlers, DNS, connection pooling, header rewriting. Feature-gated redaction/policy support. |
| `scred-mitm` | MITM TLS proxy binary. Handles CONNECT tunnels, TLS interception, ALPN negotiation, H2 multiplexing. |
| `scred-proxy` | Forward HTTP proxy binary. Fixed upstream with policy-based placeholder replacement. |
| `scred-cli` | CLI tool for ad-hoc file redaction. |
| `scred-config` | Configuration types and hot-reload support. |
| `scred-policy` | Placeholder replacement engine: replaces secrets with reversible placeholders in transit. |

## Data Flow

### Forward HTTP Proxy

```
Client ──HTTP request──→ scred-proxy ──forwarded request──→ Upstream
                              │
                        1. Parse request line (METHOD URL)
                        2. Parse headers
                        3. Redact headers via redactor.redact_buffer()
                        4. Stream body through StreamingRedactor
                           (64KB chunks + lookahead)
                        5. Forward to upstream
                        6. Read response headers
                        7. Redact response headers
                        8. Stream response body through redactor
                        9. Forward to client
```

### MITM TLS Proxy

```
Client ──CONNECT──→ scred-mitm
   │                    │
   │          1. Parse CONNECT tunnel request
   │          2. Generate dynamic TLS cert for target host
   │          3. Complete TLS handshake with client
   │          4. ALPN negotiate: h2 or http/1.1
   │          5. Connect to upstream
   │                    │
   │◄──TLS tunnel──→   │
   │                    │
   ├──HTTP/1.1 or H2──→│──forwarded──→ Upstream
   │                    │
   │     (same streaming redaction as forward proxy)
   │                    │
   │◄──redacted resp───│◄──response─── Upstream
```

## Detection Pipeline

SCRED uses 5 detection tiers, ordered by cost (fastest first). Each tier uses different algorithms — no regex anywhere.

```
detect_all(text)
├── 1. detect_simple_prefix()   [26 patterns]
│     Algorithm: Aho-Corasick automaton
│     Validation: None (just prefix match)
│     Cost: O(n) single pass for all 26 patterns
│     Examples: AKIA (AWS), ghp_ (GitHub), sk- (OpenAI)
│
├── 2. detect_validation()      [45+ patterns]
│     Algorithm: Aho-Corasick + charset LUT validation
│     Validation: Prefix match + token length + charset check
│     Cost: O(n) single pass + O(token) per match
│     Examples: hf_ (HuggingFace, min 40 chars), sk_live_ (Stripe)
│
├── 3. detect_jwt()             [1 pattern]
│     Algorithm: memchr("eyJ") + base64url LUT + dot counting
│     Validation: Exactly 2 dots, min 32 bytes, max 10000
│     Cost: O(n) with early exit on invalid chars
│
├── 4. detect_ssh_keys()        [11+ patterns]
│     Algorithm: PrefixIndex dispatch + bounded lookahead
│     Validation: Start marker → scan for end marker within lookahead
│     Cost: O(n) with PrefixIndex O(1) candidate lookup
│     Examples: SSH keys, certificates, PGP keys
│
└── 5. detect_uri_patterns()    [N patterns]
      Algorithm: Aho-Corasick scheme detection
      Validation: URI structure (scheme://user:pass@host)
      Examples: mongodb://, postgres://, webhook URLs

→ remove_overlaps(): sort by start, keep longest match
```

### Detection Algorithms

**Aho-Corasick Automaton** — Used for multi-pattern prefix matching. Built once at startup via `OnceLock`, then provides O(n) matching for all patterns simultaneously. Used by `detect_simple_prefix()` and `detect_validation()`.

**memchr** — SIMD-accelerated single-byte search. Used for finding first occurrence of a prefix byte (e.g., 'e' in "eyJ" for JWT detection). Falls back to scalar for multi-byte prefix validation.

**Charset LUTs** — 256-byte boolean lookup tables for O(1) per-byte charset membership testing. Used for scanning token boundaries (end of alphanumeric run, end of base64 run). Unrolled 8 bytes at a time for scalar throughput.

**PrefixIndex** — HashMap from first 16 bytes of multiline markers (e.g., `-----BEGIN `) to pattern indices. Enables O(1) candidate lookup instead of checking all 11 patterns at each position.

## Streaming Redaction Protocol

### Chunk Flow

```
Upstream/Client ──body bytes──→ StreamingRedactor::process_chunk()
                                         │
                                   1. Concatenate chunk with lookahead
                                      buffer from previous chunk
                                   2. Run detect_all() on combined data
                                   3. Apply redact_in_place() on matches
                                   4. Emit redacted bytes up to
                                      (chunk_size - lookahead_size)
                                   5. Keep last N bytes as lookahead
                                      for next chunk
                                         │
                                         ▼
                              ──redacted bytes──→ Downstream
```

### Lookahead Buffer

The lookahead buffer (512 bytes) handles pattern boundaries across chunks. Without it, a pattern split across two chunks would be missed. The last 512 bytes of each chunk are held back and prepended to the next chunk before detection.

### Connection-Close Framing

After redaction, the response body byte count may differ from the original Content-Length. Rather than recalculating (which requires buffering the entire body), SCRED strips Content-Length and Transfer-Encoding headers and uses `Connection: close` as the framing mechanism. The client knows the response is complete when the connection closes.

### Chunked Transfer-Encoding

For chunked responses, a `ChunkedParser` state machine reads chunk size lines, chunk data, and trailers. Each chunk is individually redacted through the StreamingRedactor. The chunked structure is flattened — downstream receives a plain body with Connection: close.

## Configuration Model

### Pattern Types

| Type | Fields | Example |
|------|--------|---------|
| `SimplePrefixPattern` | name, prefix, tier | `AKIA` (AWS access key) |
| `PrefixValidationPattern` | name, prefix, tier, min_len, max_len, charset | `hf_` (HuggingFace, min 40 chars, alphanumeric) |
| `GeneralizedMarkerPattern` | name, start_marker, end_marker, max_lookahead, pattern_type | `-----BEGIN RSA PRIVATE KEY-----` → `-----END RSA PRIVATE KEY-----` |
| `JwtPattern` | prefix, charset | `eyJ` + base64url |

### Pattern Tiers

| Tier | Description | Examples |
|------|-------------|---------|
| Critical | High-value secrets | AWS keys, GitHub tokens, OpenAI keys, Stripe keys |
| Infrastructure | System-level credentials | Docker tokens, Vault tokens, database URLs |
| Services | Third-party service keys | SendGrid, Mailgun, Discord webhooks |
| ApiKeys | Developer API keys | npm tokens, Figma tokens, Postman keys |
| Patterns | Generic patterns | X-API-KEY headers, generic env vars |

### Selector Architecture

SCRED maintains two independent pattern selectors:

- **detect_selector**: Controls which patterns appear in logs (broad by default: Critical + ApiKeys + Infrastructure)
- **redact_selector**: Controls which patterns are actually redacted (conservative by default: Critical + ApiKeys)

This enables "detect broadly, redact conservatively" — operators can see warnings for infrastructure patterns without redacting them.

## Deployment Modes

### CLI Tool (`scred`)

```
scred redact file.txt          # Redact secrets in a file
scred redact --stdin           # Redact from stdin
scred detect file.txt          # Detect without redacting
```

Use case: Ad-hoc redaction of files, CI/CD pipeline scanning, debugging.

### Forward Proxy (`scred-proxy`)

```
scred-proxy --upstream proxy.example.com:3128
```

Acts as a forward HTTP proxy with a fixed upstream. Supports policy-based placeholder replacement: secrets detected in requests are replaced with placeholders, and the upstream can restore them in responses.

Use case: Corporate proxy with secret scanning, API gateway with credential redaction.

### MITM Proxy (`scred-mitm`)

```
scred-mitm --listen 0.0.0.0:8080 --ca-key ./ca.key --ca-cert ./ca.crt
```

Full MITM TLS interception. Generates dynamic certificates for each target host. Supports HTTP/1.1 and HTTP/2 (via ALPN negotiation). Per-stream redaction in H2 multiplexed connections.

Use case: Development proxy for debugging API calls, security auditing of third-party services, test environment credential scanning.
