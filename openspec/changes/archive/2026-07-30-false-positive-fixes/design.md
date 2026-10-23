## Context

A systematic scan of 16 Wikipedia pages (738KB-1.2MB each) through the scred detector found 1,687 false positives. Each confirmed FP was analyzed to determine root cause and fix.

### Confirmed false positives

| Pattern | Type | Prefix | FPs | Root cause |
|---------|------|--------|-----|------------|
| `sentry-dsn-url` | 354 | `https://` | 1,669 | Matches every HTTPS URL >50 chars. Sentry DSNs are public per official docs |
| `mailchimp-api-key` | 123 | `us` | 8 | `us` in URL paths like `...australia-closing-keynote...` |
| `vault-service-token` | 175 | `s.` | 6 | `s.` in SVG filenames like `Pseudo.Authentication...` |
| `openai-sk` | 22 | `sk-` | 3 | Simple prefix, no validation — matches `sk-` in `task-lists`, `Musk-led` |
| `twitch-oauth-token` | 277 | `oauth-` | 1 | `oauth-` in URL slugs like `understanding-oauth-what-happens...` |

### Fix rationale

**Remove `sentry-dsn-url`:** Sentry DSNs are explicitly documented as "safe to keep public" — they only allow event submission, no read access. The pattern detects non-secrets with a prefix that's guaranteed to false-positive on any HTTPS URL.

**Remove `openai-sk` simple prefix:** This is a `SimplePrefixPattern` with zero validation — no min_len, max_len, or charset check. It matches any `sk-` anywhere. The `mistral-api-key` validation pattern (prefix `sk-`, min_len=40) already covers real `sk-` tokens with proper validation.

**Fix `mailchimp-api-key`:** Prefix `us` is a 2-letter country code. Increasing min_len from 32 to 50 reduces false positives on URL paths while still catching real Mailchimp API keys (which are typically 40+ chars).

**Fix `vault-service-token` / `vault-batch-token`:** Prefixes `s.` and `b.` are very short. Increasing min_len from 30 to 40 reduces false positives on filenames while still catching real Vault tokens (which are 30+ base64url chars).

**Fix `twilio-api-key`:** Twilio Account SIDs are exactly 34 chars (`AC` + 32 alphanumeric). Increasing min_len from 32 to 34 makes the pattern exact.

**Fix `telegram-bot-token`:** Prefix `bot` is a common word. Increasing min_len from 24 to 30 reduces false positives.

**Fix `env-token` / `env-token-suffix`:** min_len=1 matches any occurrence of `TOKEN=` in documentation. Increasing to 8 and changing charset to Alphanumeric filters short placeholders like `TOKEN=abc` while catching real tokens.

## Goals / Non-Goals

**Goals:**
- Eliminate all confirmed false positives from Wikipedia-clean content
- Reduce risk of future false positives from short-prefix patterns
- Keep real secret detection intact

**Non-Goals:**
- Adding new patterns
- Changing the pattern system architecture

## Decisions

### Decision 1: Remove, don't fix, `sentry-dsn-url`
Sentry DSNs are public by design. Even with a more specific prefix, there's no value in detecting non-secrets.

### Decision 2: Remove, don't fix, `openai-sk` simple prefix
The `mistral-api-key` validation pattern already covers `sk-` tokens with proper validation. The simple prefix is redundant and dangerous.

### Decision 3: Bump min_len for short-prefix patterns
For patterns with prefixes <= 4 chars, increasing min_len is the simplest effective fix. The values chosen (40-50) are high enough to filter most false positives while still catching real tokens.

## Risks / Trade-offs

- **[Risk]** Removing `sentry-dsn-url` means real Sentry DSNs won't be detected → **[Mitigation]** Sentry DSNs are public and not secrets.
- **[Risk]** Removing `openai-sk` means real `sk-` tokens without the `sk-ant-` prefix won't be detected → **[Mitigation]** The `mistral-api-key` pattern (prefix `sk-`, min_len=40) still catches generic `sk-` tokens.
- **[Risk]** Increasing min_len may miss shorter real tokens → **[Mitigation]** Real tokens are typically 30+ chars. The new thresholds (34-50) are based on actual token formats.
