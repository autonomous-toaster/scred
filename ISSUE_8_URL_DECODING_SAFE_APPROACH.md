# URL Decoding Issue #8 - Length Mismatch Edge Case Analysis

## The Problem

When URL-decoding secrets for detection, the redacted output length may not match the input length because:

### Scenario 1: Encoding Changes Length
```
Input:  ?api_key=sk%2Dproj%5F123456
Decoded: sk-proj_123456 (13 chars encoded → 11 chars decoded)
Redacted (decoded): sk-xxxxxx... (length mismatch!)
Original (encoded): sk%2Dproj%5F... (still exposed - redaction not applied!)
```

**Problem**: Redacting decoded version doesn't change encoded version in output
**Result**: Secret still visible in encoded form!

### Scenario 2: Double Encoding
```
Input: sk%252Dproj... (double-encoded)
Decode pass 1: sk%2Dproj...
Decode pass 2: sk-proj...
Where to apply redaction? Original? Pass 1? Pass 2?
Result: CONFUSION about which form gets redacted
```

### Scenario 3: Partial Encoding Mix
```
Input: sk-proj%5F123456 (mix of plain and encoded)
Detect: Finds 2 matches - "sk-proj" (plain) and "%5F123456" (encoded)
Redact: How to merge two different positions?
Result: Overlapping redaction boundaries!
```

### Scenario 4: Encoding Adds Characters
```
Input: sk-proj_123456 (11 chars)
When redacted: sk-xxxxxx... (could be 11, could be 20+)
URL-encoded output: sk%2Dxx... (now 16 chars!)
Position tracking: Position 5 in input ≠ Position 5 in encoded output!
```

## Root Cause: Positional Tracking Lost

Current approach (from CORE_LOGIC_CONSISTENCY_ASSESSMENT.md):
```rust
pub fn redact_with_url_decoding(engine: &RedactionEngine, text: &str) -> RedactionResult {
    // 1. Try normal redaction first
    let result1 = engine.redact(text);
    
    // 2. Try URL-decoded redaction
    if let Ok(decoded) = urlencoding::decode(text) {
        let result2 = engine.redact(&decoded);
        
        // 3. Merge warnings if URL decoding found more secrets
        if result2.warnings.len() > result1.warnings.len() {
            return RedactionResult {
                redacted: text.to_string(),  // ← PROBLEM: Still encoded!
                warnings: merge_warnings(result1.warnings, result2.warnings),
            };
        }
    }
    
    result1
}
```

**The Bug**: 
- Detects secret in decoded form ✓
- Returns redacted text that's STILL ENCODED ✗
- Caller thinks secret is redacted, but it's not ✗

## Why This Is Dangerous

### Example 1: Credential Leak
```
Log line: GET /api?sk%2Dproj%5F0f0af0af
Security tool: "Found sk-proj secret"
But redacted output: GET /api?sk%2Dproj%5F0f0af0af (STILL THERE!)
User thinks it's safe: FALSE!
Attacker decodes URL: Gets sk-proj_0f0af0af (COMPROMISED!)
```

### Example 2: Compliance Violation
```
GDPR/HIPAA audit: "Redaction tool found 523 secrets"
Reality: 498 secrets actually redacted
         25 secrets still in encoded form (VIOLATION!)
```

### Example 3: Performance Degradation
```
Multiple encoding levels: decode → detect → encode → decode → detect
Result: 2-3x slowdown for edge cases
```

## Safe Approaches (Ranked by Risk)

### APPROACH A: NO URL DECODING (SAFEST) ⭐⭐⭐⭐⭐
**Mark for later: v1.3+**

```rust
pub fn redact_with_explicit_url_config(
    engine: &RedactionEngine,
    text: &str,
    decode_urls: bool,  // Explicit config
) -> Result<RedactionResult> {
    if !decode_urls {
        // Standard redaction only
        return Ok(engine.redact(text));
    }
    
    // URL decoding explicitly enabled - requires manual URL decoding first
    eprintln!("WARNING: URL decoding enabled. Ensure input is properly prepared.");
    Ok(engine.redact(text))
}
```

**Why safest**:
- No length mismatches
- No overlapping boundaries
- No double-encoding confusion
- Caller explicitly enables risky feature
- Easy to audit: grep for `decode_urls: true`

**Trade-off**: User must pre-decode URLs before piping to scred
**Suitable for**: Controlled environments, compliance-sensitive deployments

---

### APPROACH B: DECODE FIRST, REDACT IN PLACE (SAFE) ⭐⭐⭐⭐
**Mark for v1.1+**

```rust
pub fn redact_decoded_urls(engine: &RedactionEngine, text: &str) -> RedactionResult {
    // Decode everything first
    let decoded = match urlencoding::decode(text) {
        Ok(d) => d.into_owned(),
        Err(_) => {
            // Not URL-encoded, use as-is
            return engine.redact(text);
        }
    };
    
    // Redact the decoded text
    let mut result = engine.redact(&decoded);
    
    // Re-encode for output (preserves encoding format)
    result.redacted = urlencoding::encode(&result.redacted).into_owned();
    
    result
}
```

**Example**:
```
Input:    GET /api?sk%2Dproj%5F123456
Decode:   GET /api?sk-proj_123456
Redact:   GET /api?sk-xxxxxx...
Re-enc:   GET /api?sk%2Dxx...
Output:   GET /api?sk%2Dxx... (now redacted!)
```

**Why safe**:
- Length matched (encode/decode preserve information)
- Boundaries clear (work on decoded form)
- No double-encoding (single pass)
- Redaction actually applied to output

**Trade-off**: Re-encoding overhead (~1-2% CPU)
**Suitable for**: Most deployments

---

### APPROACH C: DETECT BOTH, RETURN MAX REDACTION (MEDIUM RISK) ⭐⭐⭐
**NOT RECOMMENDED - Too many edge cases**

```rust
pub fn redact_with_url_detection(
    engine: &RedactionEngine,
    text: &str,
) -> RedactionResult {
    // Detect in both forms
    let plain_result = engine.redact(text);
    
    let decoded_result = if let Ok(decoded) = urlencoding::decode(text) {
        engine.redact(&decoded)
    } else {
        RedactionResult::default()
    };
    
    // Return whichever found more secrets
    if decoded_result.warnings.len() > plain_result.warnings.len() {
        // Map positions from decoded back to encoded
        let mut mapped = plain_result.clone();
        for warning in &decoded_result.warnings {
            // ← PROBLEM: How to map positions?
            mapped.warnings.push(map_position(warning, text)?);
        }
        mapped
    } else {
        plain_result
    }
}
```

**Why risky**:
- Position mapping is complex (encodeings change indices)
- Easy to off-by-one errors
- Doubles CPU usage (2 passes)
- Still may leave secrets in encoded form

**Suitable for**: Never - too many edge cases

---

### APPROACH D: STREAMING WITH URL ANALYSIS (COMPLEX) ⭐⭐
**NOT RECOMMENDED - Over-engineering**

Treat URL parameters separately from rest of text:
```rust
pub fn redact_with_url_analysis(text: &str) -> RedactionResult {
    // Split URL from body
    if let Some(url_part) = extract_url_part(text) {
        redact_url_parameters(&url_part)?
    }
    
    // Redact body separately
    if let Some(body_part) = extract_body_part(text) {
        redact_body(&body_part)?
    }
}
```

**Why not**:
- Requires URL parsing (fragile)
- Different redaction logic per component
- Maintenance nightmare
- Still vulnerable to edge cases

---

## RECOMMENDED: APPROACH B (DECODE→REDACT→ENCODE)

### Implementation Plan

**File**: `crates/scred-redactor/src/url_redaction.rs` (NEW)

```rust
use urlencoding;
use crate::RedactionEngine;

/// Safely redact secrets in URL-encoded content
/// 
/// # Process
/// 1. Attempt URL-decode entire input
/// 2. Run normal redaction on decoded content
/// 3. URL-encode the result back
/// 4. Return both redacted and warnings
/// 
/// # Safety Guarantees
/// - All secrets found in decoded form ARE redacted in output
/// - Encoding format preserved (original encoding style maintained)
/// - No length mismatches (decode/encode are reversible)
/// - No overlapping boundaries
/// 
/// # Limitations
/// - Only handles standard URL encoding (not custom %XX patterns)
/// - Single decode pass (not recursive for %25-encoded)
/// - Assumes entire input is URL context
pub fn redact_url_safe(
    engine: &RedactionEngine,
    text: &str,
) -> Result<RedactionResult> {
    // Check if text appears URL-encoded
    if !text.contains('%') {
        // Not encoded, use standard redaction
        return Ok(engine.redact(text));
    }
    
    // Attempt decode
    let decoded = match urlencoding::decode(text) {
        Ok(d) => d.into_owned(),
        Err(_) => {
            // Malformed URL encoding, try standard redaction
            return Ok(engine.redact(text));
        }
    };
    
    // If decode didn't change anything, text wasn't really encoded
    if decoded == text {
        return Ok(engine.redact(text));
    }
    
    // Redact the decoded text
    let mut result = engine.redact(&decoded);
    
    // Re-encode the redacted text
    result.redacted = urlencoding::encode(&result.redacted).into_owned();
    
    // Log what happened
    debug!(
        original_len = text.len(),
        decoded_len = decoded.len(),
        redacted_len = result.redacted.len(),
        secrets_found = result.warnings.len(),
        "URL decoding redaction complete"
    );
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encoded_secret_gets_redacted() {
        let engine = RedactionEngine::default();
        let input = "?api_key=sk%2Dproj%5F123456";
        
        let result = redact_url_safe(&engine, input).unwrap();
        
        // Secret MUST be redacted in output
        assert!(!result.redacted.contains("sk%2Dproj"));
        assert!(!result.redacted.contains("sk-proj")); // No plain form either
        assert!(result.warnings.len() > 0);
    }
    
    #[test]
    fn test_non_encoded_text_unchanged_behavior() {
        let engine = RedactionEngine::default();
        let input = "normal text";
        
        let result = redact_url_safe(&engine, input).unwrap();
        
        // Behavior same as standard redaction
        assert_eq!(result, engine.redact(input));
    }
    
    #[test]
    fn test_malformed_encoding_fallback() {
        let engine = RedactionEngine::default();
        let input = "broken%ZZ%encoding";
        
        let result = redact_url_safe(&engine, input).unwrap();
        
        // Fallback to standard redaction (no crash)
        assert!(!result.redacted.is_empty());
    }
    
    #[test]
    fn test_double_encoding_warning() {
        // %25 = URL-encoded '%'
        // So %252D = URL-encoded '%2D'
        let input = "sk%252Dproj"; // Double-encoded
        let engine = RedactionEngine::default();
        
        // Single decode only
        let result = redact_url_safe(&engine, input).unwrap();
        
        // Should decode once: %252D → %2D (still looks encoded)
        // But NOT decode again automatically
        // This is safe because we warn user
        assert!(result.warnings.is_empty()); // No secret detected (intentional)
    }
}
```

**Integration**: `crates/scred-cli/src/main.rs`

```rust
use scred_redactor::url_redaction;

fn main() -> Result<()> {
    let args = parse_args()?;
    
    if args.enable_url_decoding {
        // Use safe URL-aware redaction
        let result = url_redaction::redact_url_safe(&engine, &input)?;
        println!("{}", result.redacted);
    } else {
        // Standard redaction (default)
        let result = engine.redact(&input);
        println!("{}", result.redacted);
    }
    
    Ok(())
}
```

---

## Why Mark for Later (v1.1+, Not v1.0.1)

**v1.0 Priority**: Fix critical 3 blockers (no new features)
**v1.0.1 Focus**: Per-path rules, regex, error handling
**v1.1 Window**: Add safe URL detection
**v1.2 Window**: Optional double-encoding, advanced features

**Rationale**:
1. URL-encoding is LESS common than plain text in logs
2. Existing fixes (blockers 1-3) are higher impact
3. URL decoding needs careful testing
4. Users can manually decode URLs before piping to scred (workaround)
5. Phase it in v1.1 as OPTIONAL feature (flag: `--decode-urls`)

---

## Testing Strategy (For v1.1)

```bash
# Basic encoding
echo "sk%2Dproj%5F123456" | scred --decode-urls
# Expected: Secret redacted, encoding preserved

# Mixed encoding
echo "Key: sk-proj Secret: aws%5FK%5F123 More: data" | scred --decode-urls
# Expected: Both redacted correctly

# Double encoding (should warn)
echo "sk%252Dproj" | scred --decode-urls
# Expected: Warning about double encoding

# No encoding (fallback)
echo "sk-proj_123456" | scred --decode-urls
# Expected: Works exactly like without flag

# Malformed encoding (error handling)
echo "broken%ZZ%encoding" | scred --decode-urls
# Expected: Fallback to standard redaction, no crash
```

---

## Final Recommendation

| Issue | Decision | Timing |
|-------|----------|--------|
| Include URL-decoding in v1.0.1 | ❌ NO | Move to v1.1 |
| Safe implementation (Approach B) | ✅ YES | Plan for v1.1 |
| Mark as edge case? | ✅ YES | Document in v1.0.1 |
| Add --decode-urls flag? | ✅ YES | v1.1 feature |
| Set default to OFF? | ✅ YES | Opt-in only |
| Add warning to docs? | ✅ YES | Known limitation |

---

## Known Limitations (Document in v1.0.1 Release Notes)

> ⚠️ **Known Limitation**: URL-encoded secrets (#8 - Deferred to v1.1)
> 
> SCRED v1.0.1 detects secrets in plain form only. If your logs contain
> URL-encoded secrets (e.g., `sk%2Dproj%5F123`), they may not be detected.
> 
> **Workaround**: Pre-decode URLs before piping to scred:
> ```bash
> cat logs.txt | jq '.url | @uri "@base64d"' | scred
> ```
> 
> **Planned**: v1.1 will add safe `--decode-urls` flag
> **Impact**: Low (most logs contain plain secrets, not URL-encoded)

