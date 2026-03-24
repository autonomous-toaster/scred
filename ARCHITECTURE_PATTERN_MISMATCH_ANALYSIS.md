# SCRED Architecture Assessment - Pattern Mismatch

## THE REAL ISSUE: Pattern Coverage Discrepancy

### Zig Patterns (Source of Truth)
- **Total Patterns**: 270+ defined in `crates/scred-pattern-detector/src/patterns.zig`
- **Categories**:
  - SIMPLE_PREFIX_PATTERNS (26): Pure prefix detection
  - JWT_PATTERNS (1): Generic JWT detector
  - PREFIX_VALIDATION_PATTERNS (45): Prefix + validation
  - REGEX_PATTERNS (198): Complex patterns

### Rust Redaction Engine (What Actually Runs)
- **Location**: `crates/scred-redactor/src/redactor.rs`
- **Total Patterns**: ONLY 10 hardcoded in `redact_with_regex()`
- **Patterns**:
  ```rust
  ghp_           (GitHub PAT)
  gho_           (GitHub OAuth)
  ghu_           (GitHub User)
  AKIA           (AWS AKIA)
  ASIA           (AWS Access Token)
  sk-            (OpenAI key) ← WRONG! Should be sk_live for Stripe
  glpat-         (GitLab)
  xoxb-          (Slack bot)
  xoxp-          (Slack user)
  jwt            (Generic JWT with 2 dots) ← WRONG! Requires 2 dots
  ```

### Missing Patterns in Rust Engine
- ❌ sk_live_ (Stripe) - Only checks sk- not sk_live or sk_test
- ❌ sk_test_ (Stripe Test)
- ❌ rk_live_ (Stripe Restricted)
- ❌ pk_live_ (Stripe Public)
- ❌ MongoDB URLs
- ❌ Database credentials
- ❌ Private keys
- ❌ ALL PREFIX_VALIDATION patterns
- ❌ ALL 198 REGEX patterns

**Result**: 260+ patterns are defined but NOT USED

---

## WHY THIS HAPPENED

### Architecture Mismatch

**Original Design** (Intended):
```
Zig patterns.zig (270 patterns)
    ↓
FFI C boundary (scred_detector_*)
    ↓
Rust analyzer calls Zig functions
    ↓
Full pattern coverage
```

**Actual Implementation**:
```
Zig patterns.zig (270 patterns defined but NOT USED)
    ↓
Rust redactor.rs (hardcoded 10 patterns)
    ↓
CLI/Proxy/MITM use Rust engine
    ↓
Only 10 patterns work
```

### Root Cause

1. **Phase mismatch**: Zig patterns expanded to 270, but Rust engine not updated
2. **Two separate codepaths**:
   - Zig FFI path: Intended to use all 270 patterns (not wired in Rust)
   - Rust hardcoded path: Only 10 patterns, actually used by CLI
3. **No integration**: CLI doesn't use ZigAnalyzer, uses RedactionEngine instead

---

## VERIFICATION

### What WORKS (10 hardcoded patterns)
```
✅ AWS AKIA - detected and redacted
✅ AWS ASIA - detected and redacted  
✅ GitHub ghp_ - detected and redacted
✅ GitHub gho_ - detected and redacted
✅ GitHub ghu_ - detected and redacted
✅ GitLab glpat- - detected and redacted
✅ Slack xoxb- - detected and redacted
✅ Slack xoxp- - detected and redacted
✅ OpenAI sk- - detected and redacted (but pattern wrong)
✅ JWT with 2 dots - detected and redacted (but pattern requires 2 dots)
```

### What DOESN'T WORK (260+ missing patterns)
```
❌ Stripe sk_live_
❌ Stripe sk_test_
❌ Stripe rk_live_
❌ Stripe pk_live_
❌ AWS Secret Keys (wJalrXUtnFEM...)
❌ MongoDB URLs (mongodb+srv://...)
❌ Database passwords
❌ Private Keys (-----BEGIN...)
❌ All other 250+ defined patterns
```

---

## CROSS-COMPONENT CONSISTENCY

### CLI Path
```
input → env_mode.rs → ConfigurableEngine → RedactionEngine.redact()
            ↓
        redact_with_regex() [hardcoded 10 patterns]
            ↓
        Output: Only 10 patterns redacted
```

### Proxy Path
```
HTTP request → StreamingRedactor → RedactionEngine.redact()
            ↓
        redact_with_regex() [hardcoded 10 patterns]
            ↓
        Output: Only 10 patterns redacted
```

### MITM Path
```
TLS decrypted → h2_mitm_handler → StreamingRedactor → RedactionEngine.redact()
            ↓
        redact_with_regex() [hardcoded 10 patterns]
            ↓
        Output: Only 10 patterns redacted
```

**Result**: All three components ARE consistent - they all use the same 10-pattern subset!

But consistency at the wrong level - should be 270 patterns, not 10.

---

## THE SOLUTION

### Option A: Use Zig FFI (Correct Architecture)

**Why**: Zig patterns are source of truth with 270 patterns

**Changes Required**:
1. Replace `redact_with_regex()` hardcoded patterns
2. Call Zig FFI functions instead:
   - `scred_detector_simple_prefix()`
   - `scred_detector_jwt()`
   - `scred_detector_prefix_validation()`
   - `scred_detector_regex_patterns()`
3. Collect results and apply redaction
4. Stream-compatible (returns matches with positions)

**Benefits**:
- ✅ All 270 patterns available
- ✅ Single source of truth (patterns.zig)
- ✅ Easy to update patterns (just change Zig, not Rust)
- ✅ Streaming support via metadata

**Timeline**: 2-3 days (integrate Zig pattern detector properly)

### Option B: Expand Rust Hardcoded Patterns

**Why**: Keep it simple, no FFI calls

**Changes Required**:
1. Add remaining 260 patterns as regexes in Rust
2. Update `redact_with_regex()` patterns array
3. Performance testing (260 regexes per chunk)

**Cons**:
- ❌ Maintenance nightmare (copy patterns twice)
- ❌ Zig source of truth abandoned
- ❌ Performance cost (260 regex matches per chunk)
- ❌ Consistency issues when patterns updated

**Timeline**: 1-2 weeks (tedious copy/paste work)

---

## RECOMMENDATION

**Option A is correct**: Use Zig FFI to leverage the 270 patterns

**Plan**:
1. Audit which Zig FFI functions exist and work
2. Call them from RedactionEngine instead of hardcoded regex
3. Collect detection events and apply streaming redaction
4. Test with all 270 patterns
5. Ensure CLI, Proxy, MITM consistency

**Timeline**: 3-5 days for full integration

**Result**: 
- ✅ All patterns work consistently across CLI/Proxy/MITM
- ✅ Streaming-first architecture preserved
- ✅ Single source of truth
- ✅ Production ready

---

## NEXT STEPS

1. **Today**: Document pattern availability
   - [ ] List available Zig FFI functions
   - [ ] List patterns they support
   - [ ] Create test for each pattern

2. **Tomorrow**: Integrate Zig FFI into RedactionEngine
   - [ ] Replace hardcoded regex with FFI calls
   - [ ] Preserve streaming metadata
   - [ ] Update CLI/Proxy/MITM to use new engine

3. **Day 3**: Comprehensive testing
   - [ ] Test all 270 patterns
   - [ ] Cross-component consistency
   - [ ] Performance benchmarking
   - [ ] Integration with real services

4. **Day 4**: Production verification
   - [ ] Deploy to staging
   - [ ] Monitor redaction accuracy
   - [ ] Compliance verification
