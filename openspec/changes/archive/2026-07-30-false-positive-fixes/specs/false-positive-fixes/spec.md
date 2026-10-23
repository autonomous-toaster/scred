# False Positive Fixes

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Remove `sentry-dsn-url` pattern |
| T1.2 | Remove `openai-sk` simple prefix pattern |
| T1.3 | Fix `mailchimp-api-key`: min_len 32 → 50 |
| T1.4 | Fix `vault-service-token`: min_len 30 → 40 |
| T1.5 | Fix `twitch-oauth-token`: min_len 30 → 40 |
| T1.6 | Fix `vault-batch-token`: min_len 30 → 40 |
| T1.7 | Fix `twilio-api-key`: min_len 32 → 34 |
| T1.8 | Fix `telegram-bot-token`: min_len 24 → 30 |
| T1.9 | Fix `env-token`: min_len 1 → 8, charset Any → Alphanumeric |
| T1.10 | Fix `env-token-suffix`: min_len 1 → 8, charset Any → Alphanumeric |

## ADDED Requirements

### Requirement: Remove sentry-dsn-url (T1.1)

T1.1 SHALL complete BEFORE T1.2, T1.3, T1.4, T1.5, T1.6, T1.7, T1.8, T1.9, T1.10.

#### Scenario: sentry-dsn-url is removed

- **WHEN** the detector scans text containing HTTPS URLs
- **THEN** it SHALL NOT match `sentry-dsn-url`
- **AND** the `sentry-dsn-url` pattern SHALL be removed from `PREFIX_VALIDATION_PATTERNS`

### Requirement: Remove openai-sk simple prefix (T1.2)

T1.2 SHALL complete AFTER T1.1.

#### Scenario: openai-sk is removed

- **WHEN** the detector scans text containing `sk-` in words like `task-lists`
- **THEN** it SHALL NOT match `openai-sk`
- **AND** the `openai-sk` pattern SHALL be removed from `SIMPLE_PREFIX_PATTERNS`

### Requirement: Fix short-prefix patterns (T1.3 through T1.10)

T1.3 through T1.10 SHALL complete AFTER T1.2, in any order.

#### Scenario: Fixed patterns no longer false-positive on Wikipedia

- **WHEN** the detector scans Wikipedia pages
- **THEN** it SHALL produce zero false positives from the fixed patterns
