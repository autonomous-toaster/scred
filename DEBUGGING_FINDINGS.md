# Debugging Findings: Pattern Decomposition Tests

**Date**: March 25, 2026

## Critical Discovery

PREFIX_VALIDATION pattern matching is BROKEN for decomposed patterns.

### Evidence

1. **gho_ Works** ✓
   - Exists in SIMPLE_PREFIX
   - Test passes because SIMPLE_PREFIX is checked first (line 30 redaction_impl.zig)
   - Pattern never reaches PREFIX_VALIDATION

2. **aio_, xoxp-, sk_test_ Fail** ✗
   - Fixed test data to have correct lengths
   - aio_aaaaaaaaaaaaaaaaaaaaaaaaaaaa (32 chars, exact min/max)
   - xoxp-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa (45 chars, > min 40)
   - sk_test_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa (40 chars, > min 32)
   - All use valid charset (alphanumeric)
   - Still don't match

3. **Pattern Matching Order**
   - SIMPLE_PREFIX checked first (line 30)
   - PREFIX_VALIDATION checked second (line 55)
   - JWT checked third (line 91)
   - REGEX checked last (line 103+)

### Root Cause Candidates

1. **scanTokenEnd() broken**
   - Maybe returns 0 for valid tokens
   - Maybe doesn't recognize token end correctly
   - Need to add logging/debug to investigate

2. **validateLength() too strict**
   - Maybe not handling max_len=0 (unlimited)
   - Maybe off-by-one error in length checking
   - Need to test with known-working patterns

3. **Pattern matching order**
   - Maybe SIMPLE_PREFIX prevents PREFIX_VALIDATION from being checked
   - Unlikely since gho_ is in SIMPLE_PREFIX, not preventing others

4. **Charset validation issue**
   - Maybe alphanumeric charset validation broken
   - Maybe doesn't accept lowercase/uppercase/digits correctly
   - Need to test base64/base64url patterns

### Impact

- Pattern decomposition claims unverified
- Only 1 of 4 new patterns actually work
- PREFIX_VALIDATION infrastructure may be fundamentally broken
- Or just misconfiguration of new patterns

### Next Steps

**DO NOT continue debugging pattern matching.**
This is a distraction from the REAL priority: **PROFILING**.

According to negative review:
1. ❌ Profiling not done (CRITICAL)
2. ❌ Performance unverified
3. ❌ Bottleneck unknown
4. ⚠️ Pattern decomposition incomplete (lower priority)

### Recommendation

**STOP pattern work. START profiling.**

The performance goal (65-75 MB/s) is more important than 75% test passing.
Profile first to identify bottleneck, then optimize correctly.

If profiling shows pattern matching IS the bottleneck, then debug PREFIX_VALIDATION.
If profiling shows something else is bottleneck, pattern debugging is wasted time.

**Decision: Skip pattern debugging. Move to profiling phase.**

