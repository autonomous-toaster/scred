# SCRED Security Audit - CORRECTED ASSESSMENT

## CORRECTION: My Initial Analysis Was Wrong

After reviewing the design intent, I need to correct my previous assessment:

### Issue #1: Character-Preserving Is INTENTIONAL and CORRECT ✅

**Design Intent**: Preserve character count and visible structure for usability

```
Input:  AWS_KEY=AKIAIOSFODNN7EXAMPLE
Output: AWS_KEY=AKIAxxxxxxxxxxxxxxxx
```

**Why This Is Good**:
- ✅ Character count preserved (log parsing not broken)
- ✅ Key structure visible (administrators can see what was redacted)
- ✅ Prefix shows TYPE (AWS AKIA, Stripe sk_live, etc.)
- ✅ This is industry-standard (same as most log redaction tools)
- ✅ Compliance acceptable (PII is redacted, pattern preserved)

**Security Model**:
- Not designed to hide that a secret WAS there
- Designed to hide the CONTENT of the secret
- Appropriate for logs/monitoring (not passwords)

### Issue #2: Selective Un-redaction - DESIGN FLAW ❌

**Current Status**: Feature is incomplete/broken, but INTENTIONAL design

**What it SHOULD do**: 
- User runs: `scred --redact CRITICAL` (only redact critical secrets)
- Result: Critical secrets redacted, others unchanged
- Use case: Admin only cares about CRITICAL patterns, wants to see API keys for debugging

**Current Implementation Problem**:
- `selective_unredate()` uses position-based matching without pattern metadata
- Cannot accurately restore only non-matching patterns
- Works accidentally in single-pattern scenarios

**Solution**: DISABLE THIS FEATURE (not production intent yet)
- Recommendation: `--redact` flag should only support `ALL` (all patterns) and `NONE` (no patterns)
- Future: Implement with proper metadata tracking (not now)

### Issue #3: Pattern Coverage - NEEDS ASSESSMENT

Let me verify which patterns are actually NOT working:

```bash
Test failures:
- GitHub tokens (ghp_) - NOT detected
- Stripe keys (sk_live) - NOT detected  
- MongoDB URLs - NOT detected
- JWT with "Authorization: Bearer" - NOT detected (but "eyJ" IS detected)
```

**Need to investigate**: Why aren't these patterns being detected?

## Plan

1. ✅ **Confirm character-preserving is intentional** → CORRECT (not a bug)
2. ❌ **Disable `selective_unredate()` or fix it properly** → NEEDED
3. ❌ **Verify pattern coverage and fix missing patterns** → NEEDED  
4. ❌ **Ensure CLI, Proxy, and MITM consistency using streaming** → NEEDED

---

## Investigation: Why Are Some Patterns Not Detected?

Let me check if the test inputs match actual regex patterns:
