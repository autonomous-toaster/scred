# STD-003 · In-Place Zero-Copy Redaction

## Rule

All redaction MUST be performed in-place by replacing matched bytes with the character `x`. No allocation of new output buffers.

Constraints:
- Output length MUST equal input length (byte count is preserved)
- Environment variables (containing `=`) preserve the key and equals sign, keep first 4 characters of the value, redact the rest
- SSH keys, certificates, and PGP keys (pattern_type >= 300) are fully redacted (all bytes replaced with `x`)
- All other patterns keep the first 4 characters (the prefix) and redact the rest

## Rationale

In-place redaction avoids allocation per chunk, which is critical for streaming throughput. The consistent `x` replacement preserves the length invariant, which simplifies downstream framing (no Content-Length recalculation needed). Environment variable detection requires access to the original buffer to check for `=` — in-place redaction with a separate original reference enables this without cloning.

## Compliance

- `redact_in_place()` and `redact_in_place_with_original()` in `scred-detector/src/detector.rs` are the only redaction functions used in proxy paths
- No `Vec::new()` or allocation in the redaction hot path
- The `apply_redaction_rule()` function handles all three cases (env var, SSH key, regular pattern)
