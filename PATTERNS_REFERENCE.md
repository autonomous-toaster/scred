# SCRED Pattern Detector - 43 Curated Patterns Reference

## Pattern Categories & Details

### Cloud Authentication (2 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| aws-access-token | `AKIA` | 20 | AKIAIOSFODNN7EXAMPLE |
| aws-session-token | (none) | 356 | Long Base64 session tokens |

**AWS Note**: Access tokens always start with AKIA/ASIA/ABIA/ACCA. Session tokens are much longer (356 chars).

### Git & VCS (6 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| github-pat | `ghp_` | 36 | Personal Access Token |
| github-oauth | `gho_` | 36 | OAuth application token |
| github-app-token | `ghu_` | 40 | GitHub App installation token |
| github-refresh-token | `ghr_` | 36 | OAuth refresh token |
| gitlab-pat | `glpat-` | 40 | GitLab personal access token |
| gitlab-ci-token | `glcip-` | 40 | GitLab CI/CD token |

**GitHub Note**: Each token type has a unique prefix. PAT (ghp_) most common.

### Payment Processing (5 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| stripe-live-key | `sk_live_` | 32 | Production secret key |
| stripe-test-key | `sk_test_` | 32 | Test secret key |
| stripe-restricted-key | `rk_` | 32 | Restricted API key |
| stripe-publishable-key | `pk_` | 32 | Public key (safe to expose) |
| stripe-webhook-secret | `whsec_` | 40 | Webhook endpoint secret |

**Stripe Note**: Uses underscores (sk_live_, sk_test_) unlike OpenAI (sk-). No conflict.

### AI/ML APIs (4 patterns) ⭐ Expanded

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| openai-api-key-proj | `sk-proj-` | 48 | Project-scoped key |
| openai-api-key-svc | `sk-svcacct-` | 48 | Service account key |
| openai-api-key-org | `sk-` | 48 | Organization-level key |
| anthropic-api-key | `sk-ant-` | 95 | Claude API key |

**OpenAI Note**: Multiple key types all use `sk-` prefix with hyphen (not underscore like Stripe).
- `sk-proj-*`: Modern project API keys
- `sk-svcacct-*`: Service account tokens
- `sk-*`: Legacy organization keys
All have min_len=48 to avoid false positives.

### Authentication Headers (3 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| bearer-token | `Bearer ` | 20 | HTTP Authorization header |
| authorization-header | `Authorization:` | 20 | HTTP header key |
| jwt-token | `eyJ` | 50 | Base64-encoded JWT |

**JWT Note**: Always starts with `eyJ` (base64 of `{"` prefix).

### Private Keys (3 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| private-key-rsa | `-----BEGIN RSA` | 50 | RSA private key PEM |
| private-key-ec | `-----BEGIN EC` | 50 | Elliptic curve private key |
| private-key-openssh | `-----BEGIN OPENSSH` | 50 | OpenSSH format key |

**Private Keys Note**: PEM format detection. All extremely sensitive.

### Messaging & Communication (5 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| slack-bot-token | `xoxb-` | 40 | Bot token for Slack app |
| slack-user-token | `xoxp-` | 40 | User token (legacy) |
| slack-webhook | `https://hooks.slack.com` | 40 | Incoming webhook URL |
| discord-bot-token | `Bot ` | 30 | Discord bot token header |
| twilio-account-sid | `AC` | 34 | Twilio Account ID |

**Slack Note**: Bot and user tokens distinct (xoxb vs xoxp).
**Twilio Note**: Account SID pattern is `AC` followed by 32 chars, always 34 total.

### SaaS & Platform APIs (6 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| sendgrid-api-key | `SG.` | 69 | SendGrid API key |
| mailgun-api-key | `key-` | 40 | Mailgun API key |
| digitalocean-token | `dop_v1` | 40 | DigitalOcean personal token |
| mapbox-token | `pk.` | 40 | Mapbox public/secret token |
| firebase-api-key | `AIza` | 39 | Firebase API key |
| heroku-api-key | (none) | 36 | Heroku auth token |

**SendGrid Note**: Very long (69 chars) - highly specific pattern.
**Firebase Note**: Starts with `AIza` (Google API key prefix).

### More SaaS (4 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| shopify-token | `shpat_` | 32 | Shopify personal access token |
| datadog-api-key | `dd_` | 40 | Datadog API key |
| new-relic-api-key | `NRAPI-` | 40 | New Relic REST API key |
| okta-api-token | (none) | 40 | Okta authentication token |

### Databases (3 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| postgres-connection | `postgres://` | 30 | PostgreSQL connection string |
| mysql-connection | `mysql://` | 30 | MySQL connection string |
| mongodb-connection | `mongodb://` | 30 | MongoDB connection string |

**Database Note**: Connection strings with embedded credentials. Min_len=30 catches full URLs.

### Generic Fallbacks (2 patterns)

| Pattern | Prefix | Min Len | Examples |
|---------|--------|---------|----------|
| api-key-generic | `api_key` | 20 | Catch-all for variable names |
| api-token-generic | `api_token` | 20 | Catch-all for variable names |

**Generic Note**: Lower specificity, used as safety net for unknown patterns.

---

## Design Principles

### 1. Distinctive Prefixes (Not Generic)
- ✅ `sk_live_` (Stripe) - specific format
- ✅ `ghp_` (GitHub) - unique to GitHub
- ❌ `password` - catches everything
- ❌ `token` - too generic

### 2. Reasonable Min Length
- Prevents false positives on short strings
- `AKIA` token: min_len=20 (prevents "AKIA" alone)
- `jwt-token`: min_len=50 (full JWT minimum)
- Heroku: min_len=36 (UUID length)

### 3. No Overlaps
- Stripe: `sk_live_` (underscore)
- OpenAI: `sk-proj-` (hyphen)
- No conflict!

### 4. Service-Specific
- Each pattern targets a known service
- Real-world usage prioritized
- Production-tested patterns

---

## Coverage Analysis

**Total Patterns**: 43
**Estimated Real-World Coverage**: 85%+

| Service Category | Count | Coverage |
|------------------|-------|----------|
| Cloud/DevOps | 8 | AWS, DigitalOcean, Heroku, Okta |
| Git/Version Control | 6 | GitHub, GitLab |
| Payments | 5 | Stripe only (most common) |
| APIs/SaaS | 10 | OpenAI (4), Auth (3), Messaging (5) |
| Auth Headers | 3 | Bearer, JWT, Authorization |
| Private Keys | 3 | RSA, EC, OpenSSH |
| Databases | 3 | PostgreSQL, MySQL, MongoDB |
| Fallback | 2 | Generic api_key, api_token |

---

## Known Limitations

### Not Detected (By Design)

1. **Generic passwords/usernames**: min_len too short, too many FP
2. **UUID patterns**: Catch-all would flag every UUID
3. **Email addresses**: Too generic
4. **SSH hostnames**: Filesystem paths not secrets
5. **Package names**: Not sensitive data

### Why This Matters

- **False Positives** > **False Negatives** in alerts
- 1 incorrect redaction = ops burden
- 1 missed secret = catchable via incremental updates
- Better to have 85% coverage with 0% FP than 99% with 20% FP

---

## Usage Example (Rust)

```rust
let mut detector = Detector::new()?;

// Process data stream
let result = detector.process(
    b"api_key=AKIAIOSFODNN7EXAMPLE",
    true  // is_eof
)?;

// Inspect events
for event in result.events {
    println!("Detected: {} at position {}", 
        event.pattern_name(), 
        event.position
    );
}
```

---

## Extending Patterns

To add more patterns:

1. **Update `src/lib.zig`**: Add to `ALL_PATTERNS` array
2. **Set min_len**: Based on actual token length
3. **Choose prefix**: Must be distinctive
4. **Update docs**: Add to this reference
5. **Test**: Run Zig + Rust tests

Example:
```zig
.{ .name = "example-api-key", .prefix = "example_", .min_len = 32 },
```

Then run:
```bash
zig test src/lib.zig
cargo test --lib
```
