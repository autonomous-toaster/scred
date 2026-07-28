## 1. Implement H2 Header Redaction

- [x] 1.1 Add header value redaction to `H2MitmHandler::on_request_received` — redact each header value using `StreamingRedactor::redact_buffer()`, skip pseudo-headers and hop-by-hop headers
- [x] 1.2 Add non-regression tests: secret header values are redacted, non-secret values unchanged, hop-by-hop headers preserved

## 2. Verification

- [x] 2.1 Verify curl headers are redacted through H2 path
- [x] 2.2 Verify Node.js headers are redacted through H2 path (with `NODE_USE_ENV_PROXY=1`)
