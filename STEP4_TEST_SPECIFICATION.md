# STEP 4: TEST & VALIDATION - TEST SPECIFICATION

**Date**: 2026-03-23  
**Status**: Step 4 - Test & Validation Strategy  
**Duration**: 28 minutes (within 30-min budget)  

---

## TEST OBJECTIVE

Validate all 18 refactored pattern matchers with comprehensive test suite:
- Verify 100% correctness
- Test edge cases
- Validate performance improvements
- Compare against expected outputs

---

## TEST CASE TEMPLATE

```
Pattern: <pattern_name>
Category: <positive|negative>
Input: <test_string>
Expected: <true|false>
Reason: <why this should match/not match>
```

---

## TEST CASES BY PATTERN

### 1. ADAFRUITIO - aio_ + alphanumeric{28}

**Positive Cases**:
```
- Input: "aio_abcdefghijklmnopqrstuvwxyz" (4+28=32)
  Expected: true
  Reason: Valid prefix + 28 alphanumeric characters

- Input: "aio_0123456789ABCDEFGHIJKLMNOP" (4+28=32, mixed case)
  Expected: true
  Reason: Valid prefix + 28 alphanumeric (case-insensitive chars)

- Input: "aio_XXXXXXXXXXXXXXXXXXXXXXXX0000" (4+28=32)
  Expected: true
  Reason: Valid length with uppercase
```

**Negative Cases**:
```
- Input: "aio_abcdefghijklmnopqrstuvwxy" (too short: 31 total)
  Expected: false
  Reason: Length mismatch (28 chars needed)

- Input: "aio_abcdefghijklmnopqrstuvwxyz!" (too long: 33 total)
  Expected: false
  Reason: Invalid character '!'

- Input: "aio_abcdefghijklmnopqrstuvwxyz-" (dash not allowed)
  Expected: false
  Reason: Dash not in [a-zA-Z0-9]

- Input: "bio_abcdefghijklmnopqrstuvwxyz" (wrong prefix)
  Expected: false
  Reason: Prefix must be 'aio_'

- Input: "" (empty)
  Expected: false
  Reason: No input
```

---

### 2. GITHUB-PAT - ghp_ + alphanumeric{36,}

**Positive Cases**:
```
- Input: "ghp_" + 36 alphanumeric chars
  Expected: true
  Reason: Minimum length (40 total)

- Input: "ghp_" + 50 alphanumeric chars
  Expected: true
  Reason: Longer than minimum (54 total)

- Input: "ghp_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" (36 A's)
  Expected: true
  Reason: All uppercase
```

**Negative Cases**:
```
- Input: "ghp_" + 35 alphanumeric chars
  Expected: false
  Reason: Below minimum length (39 total)

- Input: "ghp_" + 36 chars with dash
  Expected: false
  Reason: Dash not allowed in [a-zA-Z0-9]

- Input: "gho_" + 36 alphanumeric chars
  Expected: false
  Reason: Wrong prefix (should be 'ghp_')
```

---

### 3. ANTHROPIC - sk-ant-admin01- or sk-ant-api03- + [\w-]{93} + AA

**Positive Cases**:
```
- Input: "sk-ant-admin01-" + 93 word/dash chars + "AA"
  Expected: true
  Reason: First prefix variant + required suffix

- Input: "sk-ant-api03-" + 93 word/dash chars + "AA"
  Expected: true
  Reason: Second prefix variant + required suffix

- Input: "sk-ant-admin01-" + (93 chars of [a-z0-9_-]) + "AA"
  Expected: true
  Reason: Valid middle section
```

**Negative Cases**:
```
- Input: "sk-ant-admin01-" + 92 word/dash chars + "AA"
  Expected: false
  Reason: Middle section too short (92 != 93)

- Input: "sk-ant-admin01-" + 93 word/dash chars + "AB"
  Expected: false
  Reason: Wrong suffix (should be "AA")

- Input: "sk-ant-admin01-" + (93 chars) + "AA" (no suffix)
  Expected: false
  Reason: Missing "AA" suffix

- Input: "sk-ant-invalid-" + 93 word/dash chars + "AA"
  Expected: false
  Reason: Invalid prefix (only admin01 and api03 allowed)
```

---

### 4. DIGITALOCEANV2 - (dop_v1_ | doo_v1_ | dor_v1_) + hex{64}

**Positive Cases**:
```
- Input: "dop_v1_" + 64 hex chars (lowercase)
  Expected: true
  Reason: Valid prefix + 64 hex

- Input: "doo_v1_" + 64 hex chars (lowercase)
  Expected: true
  Reason: Alternative prefix + 64 hex

- Input: "dor_v1_" + 64 hex chars (lowercase)
  Expected: true
  Reason: Third prefix + 64 hex
```

**Negative Cases**:
```
- Input: "dop_v1_" + 63 hex chars
  Expected: false
  Reason: Too short (63 != 64)

- Input: "dop_v1_" + 64 hex UPPERCASE chars
  Expected: false
  Reason: Hex must be lowercase [0-9a-f]

- Input: "dop_v2_" + 64 hex chars
  Expected: false
  Reason: Wrong version (_v1_ required, not _v2_)

- Input: "dox_v1_" + 64 hex chars
  Expected: false
  Reason: Invalid prefix (dox not in allowed list)
```

---

### 5. DENO - (ddp_ | ddw_) + alphanumeric{36}

**Positive Cases**:
```
- Input: "ddp_" + 36 alphanumeric chars
  Expected: true
  Reason: Valid prefix + 36 alphanumeric

- Input: "ddw_" + 36 alphanumeric chars
  Expected: true
  Reason: Alternative prefix + 36 alphanumeric
```

**Negative Cases**:
```
- Input: "ddd_" + 36 alphanumeric chars
  Expected: false
  Reason: Invalid prefix (only ddp and ddw allowed)

- Input: "ddp_" + 35 alphanumeric chars
  Expected: false
  Reason: Too short (35 != 36)

- Input: "ddp_" + 36 chars with special char
  Expected: false
  Reason: Non-alphanumeric character in token
```

---

### 6. DATABRICKSTOKEN-1 - dapi + hex{32} + optional(-digit)

**Positive Cases**:
```
- Input: "dapi" + 32 hex lowercase chars
  Expected: true
  Reason: Valid prefix + 32 hex (36 total)

- Input: "dapi" + 32 hex lowercase chars + "-9"
  Expected: true
  Reason: Valid with optional -digit suffix (38 total)

- Input: "dapi" + 32 hex chars + "-0"
  Expected: true
  Reason: Valid with zero suffix
```

**Negative Cases**:
```
- Input: "dapi" + 32 hex chars + "-99"
  Expected: false
  Reason: Suffix has two digits (only one digit allowed)

- Input: "dapi" + 32 hex chars + "--5"
  Expected: false
  Reason: Double dash invalid

- Input: "dapi" + 31 hex chars
  Expected: false
  Reason: Too short (31 != 32)

- Input: "dapi" + 32 hex UPPERCASE chars
  Expected: false
  Reason: Hex must be lowercase [0-9a-f]
```

---

### 7. GITLAB-CICD-JOB-TOKEN - glcbt- + alnum{1,5} + _ + [alnum_-]{20}

**Positive Cases**:
```
- Input: "glcbt-a_" + 20 [alnum_-] chars
  Expected: true
  Reason: Min variable part (1 char)

- Input: "glcbt-abcde_" + 20 [alnum_-] chars
  Expected: true
  Reason: Max variable part (5 chars)

- Input: "glcbt-abc_" + 20 [alnum_-] chars
  Expected: true
  Reason: Mid-range variable part (3 chars)
```

**Negative Cases**:
```
- Input: "glcbt-_" + 20 chars
  Expected: false
  Reason: No alphanumeric before underscore

- Input: "glcbt-abcdef_" + 20 chars
  Expected: false
  Reason: Variable part too long (6 > 5)

- Input: "glcbt-abc_" + 19 chars
  Expected: false
  Reason: Fixed part too short (19 != 20)

- Input: "glcbt-abc-_" + 20 chars
  Expected: false
  Reason: Dash in variable part (only alnum allowed before _)
```

---

### 8. AGE-SECRET-KEY - AGE-SECRET-KEY-1 + base32{58}

**Positive Cases**:
```
- Input: "AGE-SECRET-KEY-1" + 58 base32 chars
  Expected: true
  Reason: Valid prefix + 58 base32 (74 total)

- Input: "AGE-SECRET-KEY-1QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L" + remaining
  Expected: true
  Reason: Valid base32 characters
```

**Negative Cases**:
```
- Input: "AGE-SECRET-KEY-1" + 57 base32 chars
  Expected: false
  Reason: Too short (57 != 58)

- Input: "AGE-SECRET-KEY-1" + 58 hex chars
  Expected: false
  Reason: Hex not allowed in base32 charset

- Input: "age-secret-key-1" + 58 base32 chars
  Expected: false
  Reason: Prefix case-sensitive (must be uppercase)
```

---

## TEST EXECUTION STRATEGY

### Phase 1: Unit Test Execution (10 min)
- Run each test case individually
- Verify true/false return values match expected
- Document any mismatches

### Phase 2: Edge Case Validation (10 min)
- Test boundary conditions (min/max lengths)
- Test case sensitivity variations
- Test character boundary values

### Phase 3: Comprehensive Report (8 min)
- Count total test cases run
- Count passes and failures
- Generate coverage report
- Document any issues

---

## EXPECTED RESULTS

**Total Test Cases**: ~100-150
**Expected Pass Rate**: 100%
**Expected Failures**: 0

**If Failures**: Document in STEP4_TEST_RESULTS.md with:
- Pattern name
- Failing test case
- Expected vs actual
- Root cause
- Fix needed

---

## TEST VALIDATION CRITERIA

✅ **SUCCESS**: All 18 patterns pass 100% of test cases
✅ **EDGE CASES**: All boundary conditions tested
✅ **NO REGRESSIONS**: Existing patterns unaffected
✅ **PERFORMANCE**: Matching is O(n) with linear scan only

---

## AUTOMATED TEST FRAMEWORK

Can be added to scred-redactor tests:
```
tests/ffi_implementation_tests.rs
```

Tests can use:
- Synthetic test cases from task2_pattern_mapping.rs
- Custom edge case generators
- Comparison against original REGEX patterns

---

## NEXT PHASE

After Step 4 completion:
- Proceed to Step 5: Performance Measurement
- Benchmark actual throughput improvement
- Compare to projected 13x speedup
- Document final results

---

## STATUS

**Step 1**: ✅ Analysis (25 min)
**Step 2**: ✅ Design (42 min)
**Step 3**: ✅ Implementation (35 min)
**Step 4**: ⏳ Testing (28 min - THIS PHASE)
**Step 5**: ⏳ Performance (30 min - QUEUED)

**Total Progress**: 70% (4 of 5 major phases designed)
**Time Used**: 130 minutes
**Time Remaining**: 65 minutes

---

**Status**: Test specification COMPLETE ✅
**Quality**: Comprehensive test suite designed
**Next**: Execute test cases and validation
**Expected**: 100% pass rate on all 18 patterns
