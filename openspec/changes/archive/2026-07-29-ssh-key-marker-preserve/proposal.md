## Why

When redacting SSH private keys and certificates (pattern_type >= 300), the `apply_redaction_rule` function replaces the entire match — including the `-----BEGIN...-----` and `-----END...-----` markers — with `x`. This destroys the structural context that identifies what was redacted. The markers should be preserved so the output shows `-----BEGIN RSA PRIVATE KEY-----` with only the body content redacted.

## What Changes

- Modify `apply_redaction_rule` in `crates/scred-detector/src/detector/redact.rs` for `pattern_type >= 300`:
  - Find the first newline (end of start marker line)
  - Find the `-----END` marker (start of end marker line)
  - Redact only the body content between markers, preserving newlines

## Capabilities

### New Capabilities
- `ssh-key-marker-preserve`: Preserve `-----BEGIN...-----` / `-----END...-----` markers when redacting private keys

### Modified Capabilities
- (none)

## Impact

- `crates/scred-detector/src/detector/redact.rs`: Change `apply_redaction_rule` for `pattern_type >= 300`
