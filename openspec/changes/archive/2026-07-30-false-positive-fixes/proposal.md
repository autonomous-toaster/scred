## Why

A systematic scan of 16 Wikipedia pages (known-clean content) through the scred detector found **1,687 false positives**. Analysis revealed 5 patterns causing confirmed FPs, plus several high-risk patterns with short prefixes.

## What Changes

### Confirmed false positives (fix all)
1. **Remove `sentry-dsn-url`** (prefix `https://`, 1,669 FPs) — Sentry DSNs are public, not secrets
2. **Remove `openai-sk` simple prefix** (prefix `sk-`, 3 FPs) — no validation, matches `sk-` in words like `task-lists`. Already covered by `mistral-api-key` validation pattern
3. **Fix `mailchimp-api-key`** (prefix `us`, 8 FPs) — increase min_len from 32 to 50
4. **Fix `vault-service-token`** (prefix `s.`, 6 FPs) — increase min_len from 30 to 40
5. **Fix `twitch-oauth-token`** (prefix `oauth-`, 1 FP) — increase min_len from 30 to 40

### High-risk patterns (preventive fixes)
6. **Fix `vault-batch-token`** (prefix `b.`) — increase min_len from 30 to 40
7. **Fix `twilio-api-key`** (prefix `AC`) — increase min_len from 32 to 34 (Twilio SIDs are exactly 34 chars)
8. **Fix `telegram-bot-token`** (prefix `bot`) — increase min_len from 24 to 30
9. **Fix `env-token`** (prefix `TOKEN=`) — increase min_len from 1 to 8, charset Any → Alphanumeric
10. **Fix `env-token-suffix`** (prefix `TOKEN_`) — increase min_len from 1 to 8, charset Any → Alphanumeric

## Capabilities

### New Capabilities
- `false-positive-fixes`: Eliminate false positives from Wikipedia-clean content

### Modified Capabilities
- (none)

## Impact

- `crates/scred-detector/src/patterns/prefix_validation.rs`: Remove 1 pattern, fix 9 patterns
- `crates/scred-detector/src/patterns/simple_prefix.rs`: Remove `openai-sk` pattern
