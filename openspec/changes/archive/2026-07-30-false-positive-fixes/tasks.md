## 1. Remove confirmed false positive patterns

- [x] 1.1 Remove `sentry-dsn-url` from `PREFIX_VALIDATION_PATTERNS` in `crates/scred-detector/src/patterns/prefix_validation.rs`
- [x] 1.2 Remove `openai-sk` from `SIMPLE_PREFIX_PATTERNS` in `crates/scred-detector/src/patterns/simple_prefix.rs`

## 2. Fix short-prefix patterns (increase min_len)

- [x] 1.3 Fix `mailchimp-api-key`: min_len 32 → 50
- [x] 1.4 Fix `vault-service-token`: min_len 30 → 40
- [x] 1.5 Fix `twitch-oauth-token`: min_len 30 → 40
- [x] 1.6 Fix `vault-batch-token`: min_len 30 → 40
- [x] 1.7 Fix `twilio-api-key`: min_len 32 → 34
- [x] 1.8 Fix `telegram-bot-token`: min_len 24 → 30
- [x] 1.9 Fix `env-token`: min_len 1 → 8, charset Any → Alphanumeric
- [x] 1.10 Fix `env-token-suffix`: min_len 1 → 8, charset Any → Alphanumeric

## 3. Fix additional patterns uncovered by sentry-dsn-url removal

- [x] 1.11 Fix `mailchimp-api-key`: min_len 50 → 80
- [x] 1.12 Fix `mailgun-api-key`: min_len 40 → 55
- [x] 1.13 Fix `paypal-signature`: min_len 50 → 100
- [x] 1.14 Fix `telegram-bot-token`: min_len 30 → 40
- [x] 1.15 Fix `unity-api-key`: min_len 30 → 40
- [x] 1.16 Fix `unreal-api-key`: min_len 30 → 60
- [x] 1.17 Fix `twitch-oauth-token`: min_len 40 → 60
