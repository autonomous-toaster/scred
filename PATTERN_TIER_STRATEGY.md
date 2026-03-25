# Pattern Tier Strategy: Decision Rules & Deduplication

**Date**: March 25, 2026
**Phase**: A.2 - Remove duplicate pattern definitions

---

## Pattern Tier Classification Rules

### TIER 1: SIMPLE_PREFIX (Fastest)
**When to use**: Fixed prefix, fixed length (or range), no complex validation needed

**Characteristics**:
- Fixed prefix (e.g., "AKIA", "gho_", "AGE-SECRET-KEY-1")
- Token length known or fixed
- Simple alphanumeric charset
- Early-exit friendly (SIMD optimizable)

**Examples**:
- `AKIA` (AWS access key ID) → 4 prefix + 16 chars = 20 fixed
- `gho_` (GitHub OAuth) → 4 prefix + 36 chars = 40 fixed
- `AGE-SECRET-KEY-1` (age encryption) → Full prefix, no token needed

**Matching**: Prefix + fixed-length token validation

---

### TIER 2: PREFIX_VALIDATION (Medium speed)
**When to use**: Prefix with flexible validation (length range, charset rules)

**Characteristics**:
- Clear prefix (e.g., "sk_live_", "sk_test_", "sk-ant-")
- Token length has min/max bounds
- Charset constraints (alphanumeric, base64, base64url, etc.)
- Can't be decomposed to fixed-length

**Examples**:
- `sk_live_` (Stripe live key) → 8 prefix + 32-92 alphanumeric chars
- `sk_test_` (Stripe test key) → 8 prefix + 32-92 alphanumeric chars
- `sk-ant-` (Anthropic API) → 7 prefix + 90-100 base64-url chars

**Matching**: Prefix + charset scan + length validation

---

### TIER 3: REGEX (Slowest)
**When to use**: Complex patterns that can't be decomposed to prefix-based matching

**Characteristics**:
- Prefix with character class (e.g., `[rs]k_`, `[A-Z][a-z]_`)
- Complex format requirements
- Spanning multiple segments
- Optional prefixes or conditional formats

**Examples**:
- `[rs]k_live_...` (Stripe/Razorpay, start with r or s)
- `[A-Za-z0-9+/]+=*` (Base64 with padding)
- `(ghp_|ghu_|ghs_|ghat_)[A-Za-z0-9_]{36,255}` (GitHub token types)

**Matching**: Regex engine (expensive, use last)

---

## Duplicate Pattern Analysis

### Duplicate 1: `sk_live_` prefix

**Current State**:
```zig
// Line 104: SIMPLE_PREFIX
.{ .name = "apideck", .prefix = "sk_live_", .tier = .api_keys },

// Line 216: PREFIX_VALIDATION
.{ .name = "stripe-api-key", .prefix = "sk_live_", .tier = .critical, .min_len = 32, .max_len = 0, .charset = .alphanumeric },

// Line 246: REGEX
.{ .name = "apideck", .pattern = "\\b(sk_live_[a-z0-9A-Z-]{93})\\b" }
```

**Problem**: Matched by SIMPLE_PREFIX first → PREFIX_VALIDATION never checked

**Resolution**:
- DELETE: SIMPLE_PREFIX "apideck" (line 104) - no validation
- DELETE: REGEX "apideck" (line 246) - old/redundant
- KEEP: PREFIX_VALIDATION "stripe-api-key" (line 216) - has proper validation
- Reason: stripe-api-key tokens are 32-92 chars, need validation to prevent false positives

---

### Duplicate 2: `gho_` prefix (GitHub OAuth)

**Current State**:
```zig
// Line 140: SIMPLE_PREFIX
.{ .name = "github-gho", .prefix = "gho_", .tier = .critical },

// Line 225: PREFIX_VALIDATION
.{ .name = "github-oauth-token", .prefix = "gho_", .tier = .critical, .min_len = 36, .max_len = 36, .charset = .alphanumeric },
```

**Problem**: Matched by SIMPLE_PREFIX → PREFIX_VALIDATION validation never applied

**Resolution**:
- DELETE: SIMPLE_PREFIX "github-gho" (line 140) - no validation
- KEEP: PREFIX_VALIDATION "github-oauth-token" (line 225) - has exact validation (36 chars fixed)
- Reason: gho_ tokens are exactly 36 chars, need to prevent false matches

---

### Duplicate 3: AWS patterns (AKIA, ASIA, etc.)

**Current State**:
```zig
// Line 132: SIMPLE_PREFIX
.{ .name = "aws-akia", .prefix = "AKIA", .tier = .critical },
// Similar for ASIA, ABIA, ACCA (not shown)

// Line 256: REGEX
.{ .name = "aws-access-token", .pattern = "((?:A3T[A-Z0-9]|AKIA|ASIA|ABIA|ACCA)[A-Z0-9]{16})" }
```

**Problem**: SIMPLE_PREFIX "AKIA" matches first → REGEX pattern never needed

**Resolution**:
- DELETE: REGEX pattern (line 256) - redundant, covered by SIMPLE_PREFIX
- KEEP: SIMPLE_PREFIX for AKIA, ASIA, ABIA, ACCA (all 20 chars fixed: 4 prefix + 16 suffix)
- Reason: AWS keys are fixed 20 chars, SIMPLE_PREFIX is sufficient and faster

---

### Duplicate 4: `xoxb-` (Slack bot token)

**Current State**:
```zig
// Line 214: PREFIX_VALIDATION
.{ .name = "slack-token", .prefix = "xoxb-", .tier = .api_keys, .min_len = 40, .max_len = 0, .charset = .alphanumeric },

// Line 404: REGEX
.{ .name = "slack-bot-token", .pattern = "xoxb-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*" }
```

**Problem**: PREFIX_VALIDATION checked first, with min_len=40 and no max → REGEX never needed

**Resolution**:
- DELETE: REGEX pattern (line 404) - covered by PREFIX_VALIDATION
- KEEP: PREFIX_VALIDATION "slack-token" (line 214) - has proper validation
- Reason: Slack tokens format is prefix + flexible segments, PREFIX_VALIDATION sufficient

---

### Duplicate 5: `xoxp-` (Slack app token)

**Current State**:
```zig
// Line 224: PREFIX_VALIDATION
.{ .name = "slack-app-token", .prefix = "xoxp-", .tier = .api_keys, .min_len = 40, .max_len = 0, .charset = .alphanumeric },
```

**Status**: No duplicate! Only in PREFIX_VALIDATION. ✓

---

## Deduplication Summary

| Pattern | Current Tiers | Decision | Action |
|---------|---------------|----------|--------|
| `sk_live_` | SIMPLE_PREFIX + PREFIX_VALIDATION + REGEX | Keep PREFIX_VALIDATION | Delete SIMPLE_PREFIX + REGEX |
| `gho_` | SIMPLE_PREFIX + PREFIX_VALIDATION | Keep PREFIX_VALIDATION | Delete SIMPLE_PREFIX |
| `AKIA/ASIA/ABIA/ACCA` | SIMPLE_PREFIX + REGEX | Keep SIMPLE_PREFIX | Delete REGEX |
| `xoxb-` | PREFIX_VALIDATION + REGEX | Keep PREFIX_VALIDATION | Delete REGEX |
| `xoxp-` | PREFIX_VALIDATION only | No action | Keep |

---

## Implementation Plan

### Step 1: Identify line numbers to delete
- SIMPLE_PREFIX "apideck" for sk_live_ (line 104)
- SIMPLE_PREFIX "github-gho" (line 140)
- REGEX aws-access-token (line 256)
- REGEX slack-bot-token (line 404)
- REGEX apideck (line 246)

### Step 2: Delete duplicates from patterns.zig
- Keep all PREFIX_VALIDATION patterns
- Keep all SIMPLE_PREFIX patterns NOT in duplicates list
- Keep only non-duplicate REGEX patterns

### Step 3: Test redaction
- Run all tests: `cargo test --lib`
- Verify 29/29 core tests pass
- Verify decomposition tests (at least gho_ should work)
- Verify NO REGRESSIONS

### Step 4: Verify pattern count
- SIMPLE_PREFIX: 26 → 24 (remove 2 duplicates)
- PREFIX_VALIDATION: 45 → keep as-is
- REGEX: 140+ → reduce by removing duplicates
- Total: 244 patterns maintained or reduced (no new duplicates)

---

## Tier Decision Guidelines (For Future)

When adding a new pattern:
1. **Does it have a fixed-length token?**
   - YES → Use SIMPLE_PREFIX
   - NO → Go to 2

2. **Does it have a clear prefix + length/charset rules?**
   - YES → Use PREFIX_VALIDATION
   - NO → Go to 3

3. **Is the format complex/conditional?**
   - YES → Use REGEX (last resort)
   - NO → You should have used tier 1 or 2

---

## Regex Decomposition (Future Work)

After Phase A cleanup, analyze REGEX patterns:
- Which can be converted to PREFIX_VALIDATION?
- Which truly need REGEX?
- Goal: Minimize REGEX usage (expensive)

Examples of convertible patterns:
- `sk-ant-[0-9a-zA-Z_-]{90,100}` → PREFIX_VALIDATION with "sk-ant-" prefix
- `ghp_[0-9a-zA-Z_]{36,255}` → PREFIX_VALIDATION with "ghp_" prefix
- `[rs]k_live_[a-zA-Z0-9-]{93}` → Potentially split into 2 patterns: sk_live_ and rk_live_ (both SIMPLE_PREFIX)

---

## Conclusion

**Deduplication removes 5 obsolete pattern definitions while maintaining 100% functionality.**

After cleanup:
- Faster matching (skip unnecessary checks)
- Clearer code (no confusion about which tier to use)
- Better performance (prefer SIMPLE_PREFIX > PREFIX_VALIDATION > REGEX)

**Next Phase**: Audit remaining REGEX patterns for further decomposition opportunities.

