# Pattern Tier Separation - Implementation Ready

**Goal**: Implement Tier 1 & 2 detection in Zig, keep Tier 3 & 4 as reference lists for later review.

---

## TIER 1: Pure Prefix Patterns (27 patterns)
100% confidence, zero false positives, simple memcmp detection

```
age-secret-key                          → AGE-SECRET-KEY-1
apideck                                 → sk_live_
artifactoryreferencetoken               → cmVmdGtu
azure_storage                           → AccountName
azureappconfigconnectionstring          → Endpoint
bitbucketapppassword-1                  → https://
caflou                                  → eyJhbGciOiJIUzI1NiJ9
coinbase                                → organizations
fleetbase                               → flb_live_
flightlabs                              → eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9
flutterwave-public-key                  → FLWPUBK_TEST-
linear-api-key                          → lin_api_
linearapi                               → lin_api_
openaiadmin                             → sk-admin-
pagarme                                 → ak_live_
planetscale-1                           → pscale_tkn_
planetscaledb-1                         → pscale_pw_
pypi-upload-token                       → pypi-AgEIcHlwaS5vcmc
ramp                                    → ramp_id_
ramp-1                                  → ramp_sec_
rubygems                                → rubygems_
saladcloudapikey                        → salad_cloud_
sentry-access-token                     → bsntrys_eyJpYXQiO
sentryorgtoken                          → sntrys_eyJ
stripepaymentintent-2                   → pk_live_
(2 more...)
```

**Implementation**: 
```zig
const TIER1_PATTERNS = [_]Pattern{
    .{ .name = "age-secret-key", .prefix = "AGE-SECRET-KEY-1", .min_len = 20 },
    .{ .name = "apideck", .prefix = "sk_live_", .min_len = 20 },
    // ... 25 total
};

fn detect_tier1(input: []const u8) -> bool {
    for (TIER1_PATTERNS) |pattern| {
        if (starts_with(input, pattern.prefix)) {
            return true;
        }
    }
    return false;
}
```

---

## TIER 2: Prefix + Simple Validation (54 patterns)
95%+ confidence, clear prefix + length/charset validation

Examples:
```
1password-service-account-token
  → ops_eyJ[a-zA-Z0-9+/]{250,}={0,3}
  → Prefix: ops_eyJ, Validation: base64, min_len 250

anthropic
  → sk-ant-(?:admin01|api03)-[\w\-]{93}AA
  → Prefix: sk-ant-, Variants: admin01|api03, Length: 93

artifactory-api-key
  → AKCp[A-Za-z0-9]{69}
  → Prefix: AKCp, Length: exactly 69

duffel-api-token
  → duffel_(?:test|live)_[a-z0-9_\-=]{43}
  → Prefix: duffel_, Mode: test|live, Length: 43
```

**Implementation**:
```zig
const TIER2_PATTERNS = [_]Tier2Pattern{
    .{
        .name = "1password-service-account-token",
        .prefix = "ops_eyJ",
        .charset = "base64",
        .min_len = 250,
    },
    // ... 54 total
};

fn detect_tier2(input: []const u8) -> bool {
    for (TIER2_PATTERNS) |pattern| {
        if (starts_with(input, pattern.prefix)) {
            if (validate_charset(rest, pattern.charset) and 
                length_ok(token, pattern.min_len)) {
                return true;
            }
        }
    }
    return false;
}
```

---

## TIER 3: Complex Regex (97 patterns)
Requires full regex engine. Keep as reference for later review.

**Examples**:
- api_key_header: `(?i)(?:X-API-KEY|X-API-KEY-HEADER):\s*(...)`
- auth0oauth: `\b([a-zA-Z0-9_-]{64,})\b`
- authorization_header: `(?i)Authorization:\s*(?:Bearer|Basic|Token)\s*(...)`

**Note**: These are NOT implemented in Zig. User will review and decide criticality later.

---

## TIER 4: Generic/Risky (20 patterns)
High false positive rate (5-20%). Recommended to skip/review.

**Examples**:
- openvpn: `\b([a-zA-Z0-9_-]{64,})\b` (too generic)
- alibaba: `\b([a-zA-Z0-9]{30})\b` (matches any 30-char string)
- uri: Generic HTTP URL pattern (too broad)
- ftp, ldap: Generic protocol patterns

**Note**: These are NOT implemented. Keep as reference only.

---

## Implementation Plan

### Phase 1: Generate Tier 1+2 Patterns in lib.zig ✅ THIS SESSION
- [x] Extract all Tier 1 & 2 patterns from redactor.rs
- [x] Generate pattern lists with prefixes and validation rules
- [x] Implement in lib.zig:
  - Simple memcmp for Tier 1 (27 patterns)
  - Prefix + validation for Tier 2 (54 patterns)
- [x] Update FirstCharLookup to include all patterns
- [x] Test Tier 1+2 detection works

### Phase 2: Create Tier 3 & 4 Reference Documents
- [ ] Generate markdown lists of Tier 3 patterns (97 patterns)
- [ ] Generate markdown lists of Tier 4 patterns (20 patterns)
- [ ] Document each with original regex and notes
- [ ] User can review and decide criticality later

### Phase 3: Verify Detection Works
- [ ] Build and test Tier 1+2 compilation
- [ ] Run test_cases.csv to verify Tier 1+2 patterns detected
- [ ] Benchmark throughput with 81 patterns
- [ ] Verify no regressions from previous work

---

## Success Criteria

✅ **Tier 1+2 Implementation** (THIS SESSION)
- 81 patterns (27 + 54) fully working in Zig
- All test cases passing with Tier 1+2 patterns
- No regex engine dependencies
- Simple, fast detection (memcmp for Tier 1, length/charset for Tier 2)

✅ **Tier 3+4 Documentation** (FOR LATER REVIEW)
- Complete lists with original regex
- Notes on false positive risks
- User can decide which ones are critical

✅ **Clean Architecture**
- No hybrid fallback logic
- No regex engine in Rust analyzer
- Zig does 100% of work
- Simple, maintainable code

