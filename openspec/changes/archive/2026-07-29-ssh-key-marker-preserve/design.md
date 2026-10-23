## Context

`apply_redaction_rule` in `crates/scred-detector/src/detector/redact.rs` handles three cases:

1. **Environment variables** (contains `=`): keep `key=`, preserve first 4 chars of value
2. **SSH keys / certificates** (pattern_type >= 300): fully redact with `x`
3. **Regular patterns** (API keys, tokens): keep first 4 chars (prefix), redact rest

For case 2, the entire match region is replaced with `x`, including the `-----BEGIN...-----` and `-----END...-----` markers. This loses the structural context.

## Current behavior

```
xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
...
xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

## Desired behavior

```
-----BEGIN RSA PRIVATE KEY-----
xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
...
-----END RSA PRIVATE KEY-----
```

## Goals / Non-Goals

**Goals:**
- Preserve `-----BEGIN...-----` and `-----END...-----` markers
- Redact all body content between markers
- Preserve newlines in the body for structural readability

**Non-Goals:**
- Changing the detection logic (markers are already detected correctly)
- Changing other redaction cases (env vars, regular patterns)

## Decisions

### Decision 1: Find markers by scanning the match region
The start marker ends at the first `\n`. The end marker starts at `-----END`. These are reliable heuristics for all PEM-encoded keys and certificates.

### Decision 2: Preserve newlines in the body
Only non-newline characters in the body are replaced with `x`. This keeps the output readable and preserves line structure.

## Risks / Trade-offs

- **[Risk]** If a match doesn't contain `-----END` (malformed key), the body won't be redacted → **[Mitigation]** This is an edge case; the match itself would be unusual. The function falls back to no redaction in that region.
- **[Risk]** If a match has no newlines (single-line match), nothing is redacted → **[Mitigation]** SSH keys and certificates are always multiline. Single-line matches would be a detection bug.
