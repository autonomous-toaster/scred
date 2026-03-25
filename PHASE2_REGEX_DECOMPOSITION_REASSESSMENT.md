# Phase 2: Comprehensive Regex Pattern Decomposition Assessment

## Executive Summary

**CRITICAL FINDING**: Previous work identified that **135-155 REGEX patterns (68-78%) can be decomposed** into simple PREFIX_VALIDATION patterns. Only 25-40 patterns are truly complex and require regex.

### Current State (incomplete decomposition)
```
26  SIMPLE_PREFIX_PATTERNS
47  PREFIX_VALIDATION_PATTERNS (45 patterns + some duplicates)
220 REGEX_PATTERNS (includes 135-155 decomposable patterns!)
1   JWT_PATTERNS
────────────────────────────────
294 Total patterns
```

### What Should Be (after full decomposition)
```
26  SIMPLE_PREFIX_PATTERNS (unchanged)
182 PREFIX_VALIDATION_PATTERNS (+135 decomposed from REGEX)
39  REGEX_PATTERNS (only truly complex patterns)
1   JWT_PATTERNS
────────────────────────────────
248 Total patterns (cleaner, much faster)

Result: 94% of patterns avoid regex! (217/248)
```

### Performance Impact

| Metric | Current | After Decomposition | Gain |
|--------|---------|-------------------|------|
| Regex-using patterns | 220 (75%) | 39 (16%) | **79% reduction** |
| Non-regex patterns | 74 (25%) | 209 (84%) | **282% increase** |
| Estimated throughput | ~35-40 MB/s | ~65-75 MB/s | **50-100% faster** |

---

## Pattern Decomposition Analysis (from TASK1_ULTRA_REFINED_DECOMPOSITION.md)

### Category A: Simple PREFIX + CHARSET + LENGTH (120+ patterns)

These patterns follow ONE of 5 simple structures and can be directly converted:

**Structure 1: PREFIX + FIXED LENGTH**
```
Examples:
- adafruitio: aio_[a-zA-Z0-9]{28} → prefix(aio_) + 28 alphanumeric
- age-secret-key: AGE-SECRET-KEY-1[...]{58} → prefix + 58 bech32
- clojars-api-token: CLOJARS_[a-z0-9]{60} → prefix + 60 lowercase+digits
- databrickstoken: dapi[0-9a-f]{32} → prefix(dapi) + 32 hex
- easypost-api-token: EZAK[a-z0-9]{54} → prefix + 54 alphanumeric

Patterns: ~30-40
Speedup: 13x (vs regex)
Effort: 20-30 min each
```

**Structure 2: PREFIX + CHARSET + MIN_LENGTH**
```
Examples:
- github-pat: ghp_[0-9a-zA-Z]{36,} → prefix(ghp_) + min 36 alphanumeric
- github-oauth: gho_[0-9a-zA-Z]{36,} → prefix(gho_) + min 36
- github-user: ghu_[0-9a-zA-Z]{36,} → prefix(ghu_) + min 36
- github-server: ghs_[0-9a-zA-Z]{36,} → prefix(ghs_) + min 36
- github-refresh: ghr_[0-9a-zA-Z]{36,} → prefix(ghr_) + min 36
- stripe: [rs]k_live_[a-zA-Z0-9]{20,} → 2 prefixes + min 20
- mapbox: [ps]k\.[a-zA-Z0-9]{20,} → 2 prefixes + min 20

Patterns: ~40-50
Speedup: 13x
Effort: 15-20 min each
```

**Structure 3: PREFIX + CHARSET + FIXED LENGTH + SUFFIX**
```
Examples:
- anthropic: sk-ant-[\w-]{93}AA → prefix + 93 + suffix AA verification
- flutterwave-public-key: FLWPUBK_TEST-[a-h0-9]{32}-X → prefix + 32 + suffix -X
- dynatrace-api-token: dt0c01\.[...]{88}\.[...]{64} → 3-part structure

Patterns: ~20-30
Speedup: 10-12x (slightly more complex)
Effort: 30-45 min each
```

**Structure 4: MULTIPLE PREFIXES (Alternation → Multiple Patterns)**
```
Examples:
- digitaloceanv2: (dop|doo|dor)_v1_[a-f0-9]{64} → Split into 3 patterns
  - dop_v1_ prefix + 64 hex
  - doo_v1_ prefix + 64 hex  
  - dor_v1_ prefix + 64 hex

- deno: dd[pw]_[a-zA-Z0-9]{36} → Split into 2 patterns
  - ddp_ prefix + 36 alphanumeric
  - ddw_ prefix + 36 alphanumeric

- huggingface: (hf_|api_org_)[a-zA-Z0-9]{34} → Split into 2 patterns

Patterns: 15+ (becomes 40+ when split)
Speedup: 13x per variant
Effort: 20-30 min for split + decomposition
```

**Structure 5: PREFIX + VARIABLE CHARSET (Charset but straightforward)**
```
Examples:
- contentfulpat: CFPAT-[a-zA-Z0-9_-]{43} → prefix + 43 (alphanumeric+hyphen+underscore)
- gitlab-token: glpat-[a-zA-Z0-9]{40} → prefix + 40 alphanumeric
- mailgun: key-[a-z0-9]{32} → prefix + 32 lowercase+digits
- notion: secret_[A-Za-z0-9]{43} → prefix + 43 alphanumeric
- sendgrid: SG\.[A-Za-z0-9]{20,39} → prefix + 20-39 alphanumeric

Patterns: ~20-30
Speedup: 13x
Effort: 20-30 min each
```

**Summary: Category A**
- Total patterns: 120-150
- Can convert to: PREFIX_VALIDATION_PATTERNS
- Effort: 40-50 hours (20-30 min × 120-150)
- Speedup: 13x average
- Impact: Major performance improvement

### Category B: Complex Alternation (15+ patterns)

These have multiple prefix variants via regex alternation. Solution: split into multiple simple patterns.

**Examples:**
```
digitalocean: (dop|doo|dor)_v1_[a-f0-9]{64}
  → 3 separate PREFIX_VALIDATION patterns

github variants: Already handled (ghp_, gho_, ghu_, ghs_, ghr_)
  → 5 separate PREFIX_VALIDATION patterns (already exist!)

stripe: [rs]k_live_[a-zA-Z0-9]{20,}
  → 2 separate patterns (rk_live_, sk_live_)

mapbox: [ps]k\.[a-zA-Z0-9]{20,}
  → 2 separate patterns (pk., sk.)

slack-bot: xoxb-[0-9]{10,13}-[0-9]{10,13}
  → 1 complex pattern (can decompose with structured validation)

tailscale: tskey-[a-z]+-[0-9A-Za-z_]+-[...]
  → Multiple variants, can split
```

**Summary: Category B**
- Total REGEX patterns with alternation: 15+
- Total patterns after split: 40+
- Effort: 15-20 hours (split + convert)
- Speedup: 13x per pattern
- Impact: +40 additional PREFIX_VALIDATION patterns

### Category C: Truly Complex (25-40 patterns)

These REQUIRE regex engine. Cannot be decomposed:

**Lookahead/Lookbehind (5-10 patterns):**
```
authorization_header: (?i)Authorization:\s*(?:Bearer|Basic|Token)\s+([A-Za-z0-9\-._~+\/]+=*)
api_key_header: (?i)(?:X-API-KEY|X-API-KEY-HEADER):\s*([A-Za-z0-9\-._~+\/]+=*)
private-key patterns: -----BEGIN.*PRIVATE KEY-----[...] 
→ Needs lookbehind for full capture
```

**Complex URL/Path Parsing (5-10 patterns):**
```
mongodb: mongodb+srv://(?P<user>...)@(?P<host>...):(?P<port>...)/(?P<database>...)
auth0-1: ([a-zA-Z0-9\-]{2,16}\.[a-zA-Z0-9_-]{2,3}\.auth0\.com)
coinbase-cicd: organizations\*/\w{8}-\w{4}-...\*/apiKeys\*/...
→ Requires path parsing and validation
```

**JWT-like Structures (5-10 patterns):**
```
jwt: ((?:eyJ|ewog)...)={0,2}\.(?:eyJ|ewo...)...
caflou: eyJhbGciOiJIUzI1NiJ9[a-zA-Z0-9._-]{135}
→ Multiple alternative prefixes + specific structure
```

**Named Captures/Complex Validation (5-10 patterns):**
```
bitbucketapppassword: (?P<username>...):(?P<password>ATBB[a-zA-Z0-9_=.-]+)
aws-session-token: ([A-Za-z0-9/+=]{356,})
azure patterns with account extraction
→ Named captures for multi-field extraction
```

**Summary: Category C**
- Total patterns: 25-40
- MUST keep in REGEX_PATTERNS
- Cannot decompose
- Effort: 0 (no work needed)
- Speedup: N/A
- Impact: These are the only regex patterns needed

---

## Current Implementation Status

### What's Already Marked

Only 18 patterns marked with `// could be` in patterns.zig:
```
✅ adafruitio, age-secret-key, anthropic, apideck, apify
✅ clojars-api-token, contentfulpat, databrickstoken-1, deno, dfuse
✅ digitaloceanv2, doppler-api-token, duffel-api-token, dynatrace-api-token
✅ easypost-api-token, fleetbase, github-*, gitlab-cicd-job-token
```

### What's Missing

**120+ additional patterns** that analysis shows CAN be decomposed:
```
- All fixed-length PREFIX patterns
- All min-length PREFIX patterns  
- All PREFIX+charset validation patterns
- All alternation patterns (can be split)
```

### Analysis Documents Already Exist

- ✅ `TASK1_ULTRA_REFINED_DECOMPOSITION.md` - 5 decomposition structures identified
- ✅ `PHASE4_TASK1_DECOMPOSITION_ANALYSIS.md` - Comprehensive pattern-by-pattern analysis
- ✅ Multiple planning docs with specific pattern lists

### What's NOT Been Done

- ❌ Actual implementation of decomposition (still 18/220 patterns only)
- ❌ Moving patterns from REGEX to PREFIX_VALIDATION
- ❌ Updating patterns.zig with decomposed patterns
- ❌ Performance testing with full decomposition

---

## Phase 2 Corrected Plan

### Step 0: Complete Decomposition Mapping (15 min)
**Goal**: List ALL 135-155 decomposable patterns with their transformations

Using existing analysis:
- Review TASK1_ULTRA_REFINED_DECOMPOSITION.md (5 structures)
- Review PHASE4_TASK1_DECOMPOSITION_ANALYSIS.md (pattern list)
- Extract all 120-150 Category A patterns
- Extract all 15+ Category B patterns
- Verify Category C patterns (keep in regex)

**Output**: Complete mapping file with:
```
Pattern Name | Current Regex | Proposed Structure | Effort
─────────────────────────────────────────────────────────
adafruitio | aio_[a-zA-Z0-9]{28} | PREFIX(aio_) + 28 alphanumeric | 25 min
anthropic | sk-ant-[\w-]{93}AA | PREFIX + 93 + suffix AA | 35 min
...
```

### Step 1: Remove Duplicates (5 min)
Remove 6 patterns already in PREFIX_VALIDATION:
- github-pat, github-oauth, github-user, github-server, github-refresh
- gitlab-token, gitlab-cicd-job-token

### Step 2: Move 18 Marked Patterns (10 min)
Move already-marked patterns to PREFIX_VALIDATION:
- Already identified in patterns.zig
- Clear transformation rules

### Step 3: Move Category A Patterns (120-150 patterns) (120-180 min)
Move 120-150 simple PREFIX+CHARSET+LENGTH patterns:
- Fixed length patterns: ~30-40 patterns
- Min length patterns: ~40-50 patterns
- Fixed+suffix patterns: ~20-30 patterns
- Variable charset patterns: ~20-30 patterns

### Step 4: Move Category B Patterns (15+ patterns) (30-60 min)
Split alternation patterns into multiple simple patterns:
- 15 REGEX patterns → 40+ PREFIX_VALIDATION patterns
- Examples: digitalocean (1→3), stripe (1→2), mapbox (1→2)

### Step 5: Verify Category C (25-40 patterns) (10 min)
Ensure truly complex patterns are correctly identified:
- Lookahead/lookbehind patterns
- Named capture patterns
- Multi-part URL/path patterns

### Step 6: Testing (20 min)
- `cargo test` all 35 tests pass
- Verify redaction still works
- Performance benchmarking

---

## Effort Breakdown

| Task | Patterns | Est. Time | Notes |
|------|----------|-----------|-------|
| Step 0: Mapping | All | 15 min | Analysis work |
| Step 1: Remove duplicates | 6 | 5 min | Quick |
| Step 2: Move marked patterns | 18 | 10 min | Pre-identified |
| Step 3: Move Category A | 120-150 | 120-180 min | Bulk of work |
| Step 4: Move Category B | 15→40 | 30-60 min | Splitting |
| Step 5: Verify Category C | 25-40 | 10 min | Validation |
| Step 6: Testing | - | 20 min | QA |
| **Total** | | **210-300 min (3.5-5 hrs)** | **Single session** |

---

## Revised Phase 2 Plan

### BEFORE Refactoring
```
26  SIMPLE_PREFIX_PATTERNS
47  PREFIX_VALIDATION_PATTERNS (+ some duplicates)
220 REGEX_PATTERNS
1   JWT_PATTERNS
────────────────────────────────
294 Total

Non-regex efficiency: 74/294 = 25%
```

### AFTER FULL Decomposition
```
26  SIMPLE_PREFIX_PATTERNS (unchanged)
182 PREFIX_VALIDATION_PATTERNS (+160 decomposed, -25 duplicates removed)
32  REGEX_PATTERNS (only truly complex)
1   JWT_PATTERNS
────────────────────────────────
241 Total (cleaner!)

Non-regex efficiency: 209/241 = 87%
Expected throughput: 65-75 MB/s (vs current 35-40 MB/s)
```

---

## Decision Matrix

### Option A: Minimal (18 patterns only) - Previous Plan
- Time: 65 min
- Patterns decomposed: 18
- Result: 77% non-regex, ~40-45 MB/s
- Status: Incomplete

### Option B: Comprehensive (135-155 patterns) - RECOMMENDED
- Time: 3.5-5 hours
- Patterns decomposed: 135-155
- Result: 87% non-regex, 65-75 MB/s
- Status: Complete optimization

### Option C: Two-Phase Approach
- Phase 2a: Do 18 marked patterns (65 min)
- Phase 2b: Do Category A patterns (2-3 hrs later)
- Total: Can be split across sessions

---

## RECOMMENDATION

**Proceed with Option B: Comprehensive Decomposition (3.5-5 hours)**

**Why:**
1. Analysis already done (TASK1, PHASE4 docs exist)
2. Clear transformation rules for all 135-155 patterns
3. Massive performance gain: 50-100% throughput improvement
4. Can be done in single focused session
5. Only truly complex 25-40 patterns kept as regex

**Next Step:**
Start with Step 0 - Create complete decomposition mapping file listing all 135-155 patterns with transformation rules.

