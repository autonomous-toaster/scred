# SCRED Docker Compose Testing Stack

Complete testing environment for SCRED (Secret Redaction) with multiple proxy configurations and test scenarios.

## Overview

This Docker Compose stack provides:

- **fake-upstream**: Python HTTP server serving sensitive data (JSON file)
- **httpbin**: Standard backend for general HTTP testing
- **scred-proxy**: Reverse proxy with request-only redaction (in front of httpbin)
- **scred-mitm**: MITM proxy with request-only redaction
- **scred-proxy-response-only**: Reverse proxy with response-only redaction (in front of fake-upstream)
- **scred-mitm-response-only**: MITM proxy with response-only redaction
- **test-client**: curl-based client for running tests

## Quick Start

### Start the Stack

```bash
docker-compose up -d
```

### Verify All Services

```bash
docker-compose ps
```

Expected output:
```
NAME                               STATUS
scred-fake-upstream               Up (healthy)
scred-httpbin                     Up (healthy)
scred-proxy                       Up
scred-mitm                        Up
scred-proxy-response-only         Up
scred-mitm-response-only          Up
scred-test-client                 Up
```

### Cleanup

```bash
docker-compose down
```

## Test Scenarios

### 1. Direct Fake Upstream (No Redaction)

Access the sensitive data without any redaction:

```bash
curl http://localhost:8001/secrets.json | jq
```

**Expected**: Raw AWS keys, API tokens, database passwords visible

### 2. Reverse Proxy with Response Redaction

Access fake upstream through reverse proxy that redacts responses:

```bash
curl http://localhost:9998/secrets.json | jq
```

**Expected**: Same JSON structure, but all secret values replaced with 'x' characters

### 3. MITM Proxy with Response Redaction

Access fake upstream through MITM proxy that redacts responses:

```bash
curl --proxy http://localhost:8889 http://fake-upstream:8001/secrets.json | jq
```

**Expected**: Same as reverse proxy - secrets redacted

### 4. Reverse Proxy with Request Redaction (httpbin)

Test request-only redaction with httpbin:

```bash
curl http://localhost:9999/get?api_key=sk-1234567890abcdefghijklmnopqrstuvwxyz | jq .args
```

**Expected**: Request query parameters redacted before reaching httpbin

### 5. Direct httpbin (No Redaction)

For comparison, direct httpbin access:

```bash
curl http://localhost:8000/get?api_key=sk-1234567890abcdefghijklmnopqrstuvwxyz | jq .args
```

**Expected**: Raw api_key visible in httpbin's response

## Service Details

### fake-upstream (Port 8001)

- **Role**: Serves sensitive data files for testing response-only redaction
- **Image**: `python:3.11-slim`
- **Endpoint**: `GET /secrets.json`
- **Data**: JSON file with 200+ sensitive data patterns
- **Patterns Included**:
  - AWS IAM keys (AKIA...)
  - API tokens (sk-...)
  - GitHub tokens (ghp_, ghu_, glpat-)
  - Database credentials (PostgreSQL, MySQL, MongoDB)
  - JWT/OAuth tokens
  - Private keys (RSA, EC, OpenSSH)
  - GCP/Firebase service accounts
  - Stripe live keys (sk_live_)
  - URLs with embedded credentials
  - Multiple secrets in single values
  - Base64-encoded secrets
  - Secrets in JSON object values

### httpbin (Port 8000)

- **Role**: Standard backend for HTTP testing
- **Image**: `kennethreitz/httpbin:latest`
- **Used By**: scred-proxy
- **Endpoints**: `/get`, `/post`, `/headers`, etc.

### scred-proxy (Port 9999)

- **Role**: Reverse proxy with request-only redaction
- **Upstream**: httpbin (port 8000)
- **Redaction Mode**: Request-only
  - ✓ Redacts requests before reaching httpbin
  - ✗ Does not redact responses
- **Use Case**: Prevent secrets in requests from reaching backend

### scred-mitm (Port 8888)

- **Role**: MITM proxy with request-only redaction
- **Redaction Mode**: Request-only
  - ✓ Redacts all outgoing requests
  - ✗ Does not redact responses
- **Use Case**: Client-side transparent proxy with request redaction
- **Configuration**: Can be used as `HTTP_PROXY=http://localhost:8888`

### scred-proxy-response-only (Port 9998)

- **Role**: Reverse proxy with response-only redaction
- **Upstream**: fake-upstream (port 8001)
- **Redaction Mode**: Response-only
  - ✗ Does not redact requests
  - ✓ Redacts responses before returning to client
- **Use Case**: Hide secrets exposed by upstream (API docs, error messages)

### scred-mitm-response-only (Port 8889)

- **Role**: MITM proxy with response-only redaction
- **Redaction Mode**: Response-only
  - ✗ Does not redact outgoing requests
  - ✓ Redacts incoming responses before returning to client
- **Use Case**: Client-side transparent proxy with response redaction
- **Configuration**: Can be used as `HTTP_PROXY=http://localhost:8889`

## Redaction Modes

### Request-Only Mode
```
Redaction Flow:
Client → Proxy → [REDACT] → Backend → Response → [PASS] → Client
```
- Hides client's secrets from backend
- Useful for protecting untrusted upstream servers
- Common use case: credentials to external APIs

### Response-Only Mode
```
Redaction Flow:
Client → Proxy → [PASS] → Backend → Response → [REDACT] → Client
```
- Hides backend's exposed secrets from client
- Useful for protecting from upstream's poor practices
- Common use cases: API docs with examples, error messages with keys

## Key Features Tested

### 1. Pattern Detection
SCRED detects 273+ patterns across 8 categories:
- Classical secrets (AWS, GitHub, Stripe, OpenAI)
- Infrastructure secrets (SSH keys, private keys)
- Structured formats (JWT, database URLs)
- Validation patterns
- Multiline patterns
- Regex-based patterns

### 2. Character Preservation
All redaction maintains input length:
```
Before: "aws_key": "AKIAIOSFODNN7EXAMPLE"
After:  "aws_key": "AKIAxxxxxxxxxxxxxx"
        ↑ Same length (20 chars) ↑
```

### 3. Streaming
Redaction operates on streaming chunks (no buffering):
- 64KB chunks for memory efficiency
- Handles GB-scale files
- Character preservation per chunk
- Real-time response streaming

### 4. MITM TLS Interception
MITM proxy can:
- Generate certificates on-the-fly
- Cache certificates per domain
- Intercept HTTPS traffic
- Preserve character length in encrypted streams

## Configuration

### Environment Variables

All services support environment variable configuration. See `.env.example` for complete reference:

```bash
# For reverse proxy response-only:
SCRED_PROXY_RESPONSE_ONLY_REDACT_REQUEST=false
SCRED_PROXY_RESPONSE_ONLY_REDACT_RESPONSE=true

# For MITM response-only:
SCRED_MITM_RESPONSE_ONLY_REDACT_REQUESTS=false
SCRED_MITM_RESPONSE_ONLY_REDACT_RESPONSES=true
```

### Logging

All services default to debug logging:

```
RUST_LOG=debug
SCRED_LOG_LEVEL=debug
SCRED_LOG_FORMAT=json
```

View logs:
```bash
docker-compose logs -f scred-proxy-response-only
```

## Testing Best Practices

### 1. Verify Patterns in secrets.json

```bash
# Check if AWS key is present
curl -s http://localhost:8001/secrets.json | grep AKIA

# Count total keys
curl -s http://localhost:8001/secrets.json | jq '. | keys | length'
```

### 2. Verify Redaction Through Proxy

```bash
# Unredacted (direct)
curl -s http://localhost:8001/secrets.json | grep -o "AKIA[^\"]*"

# Redacted (through proxy)
curl -s http://localhost:9998/secrets.json | grep -o "AKIA[^\"]*"
# Should show: AKIAxxxxxxxxxxxxxx
```

### 3. Compare Request vs Response Redaction

```bash
# Request redaction (httpbin echoes back)
curl -s http://localhost:9999/get?secret=sk-test | jq .args.secret

# Response redaction (upstream response redacted)
curl -s http://localhost:9998/secrets.json | jq .api_tokens
```

### 4. Performance Testing

```bash
# Time direct access
time curl -s http://localhost:8001/secrets.json > /dev/null

# Time through proxy (similar overhead)
time curl -s http://localhost:9998/secrets.json > /dev/null
```

## Architecture

```
                    ┌─────────────┐
                    │ Test Client │
                    └──────┬──────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
    ┌───▼────┐        ┌───▼────┐        ┌───▼────┐
    │ Direct │        │ Proxy  │        │ MITM   │
    │ Access │        │        │        │        │
    └───┬────┘        └───┬────┘        └───┬────┘
        │ (no redact)     │               │
        │            ┌────▼────┐      ┌──▼────┐
        │            │ Redact  │      │Redact │
        │            │Response │      │ Req   │
        │            └────┬────┘      └──┬────┘
        │                 │              │
        └──────┬──────────┴──────┬───────┘
               │                 │
        ┌──────▼─────┐   ┌──────▼───────┐
        │ Fake       │   │ httpbin      │
        │ Upstream   │   │              │
        │ (8001)     │   │ (8000)       │
        └────────────┘   └──────────────┘
```

## Troubleshooting

### Service Won't Start

Check logs:
```bash
docker-compose logs scred-proxy-response-only
```

Common issues:
- Port already in use: Change port in docker-compose.yml
- Missing build context: Ensure Dockerfile.proxy and Dockerfile.mitm exist
- Network issues: Check docker network is created

### Response Not Redacted

1. Verify service is running
2. Check logs for pattern detection
3. Verify REDACT_RESPONSE=true is set
4. Ensure sensitive data matches known patterns

### MITM Proxy Not Intercepting

1. Check TLS certificate directory has write permissions
2. Verify proxy environment variables are set
3. Test with curl and explicit proxy setting
4. Check firewall rules

### Character Preservation Issues

1. Verify output length matches input length
2. Check JSON is valid after redaction
3. Look for encoding issues in logs

## Integration with External Upstream

To use with your own upstream:

1. Create environment file:
```bash
cp .env.example .env
```

2. Update upstream URL:
```bash
SCRED_PROXY_RESPONSE_ONLY_UPSTREAM_URL=http://your-service:port
```

3. Restart:
```bash
docker-compose up -d scred-proxy-response-only
```

## Performance Notes

- **Memory**: Each service uses ~100-200MB at rest
- **CPU**: Minimal during idle, scales with traffic
- **Network**: Single-hop for proxies, minimal latency overhead
- **Streaming**: Chunk-based redaction, no buffering

## Security Notes

⚠️ **WARNING**: This stack includes intentionally exposed secrets for testing only.

**Do NOT**:
- Expose ports to untrusted networks
- Use secrets.json file in production
- Run this stack on internet-facing machines
- Share docker-compose output with external parties

**Production Considerations**:
- Use TLS/HTTPS for all connections
- Limit proxy access to trusted networks
- Enable audit logging
- Monitor pattern detection for false positives
- Regular key rotation

## Development Workflow

```bash
# Start stack
docker-compose up -d

# Make changes to scred code
# Rebuild proxy
docker-compose build scred-proxy-response-only

# Restart service
docker-compose up -d scred-proxy-response-only

# Test changes
curl http://localhost:9998/secrets.json | jq

# View logs
docker-compose logs -f scred-proxy-response-only
```

## Files

- `docker-compose.yml`: Service definitions
- `.env.example`: Environment configuration template
- `secrets.json`: Sensitive data file for fake-upstream
- `index.html`: HTML page served by fake-upstream
- `start-fake-upstream.sh`: Startup script for fake-upstream
- `README.md`: This file

## Related Documentation

- [AGENT.md](../AGENT.md): Architecture and design
- [PRODUCTION_PATTERNS_V2.md](../crates/scred-pattern-detector/PRODUCTION_PATTERNS_V2.md): Pattern definitions
- [Integration Tests](../integration_test.py): Python test examples

## License

MIT - See LICENSE file in repository root
