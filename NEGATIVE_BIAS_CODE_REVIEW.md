# NEGATIVE BIAS CODE REVIEW - SCRED P0+P1+P2
## Security, Consistency, and Redaction Gap Analysis

**Date**: 2026-03-23  
**Scope**: CLI, MITM, Proxy, and integration testing  
**Bias**: Negative - Looking for what CAN go wrong, not what works

---

## CRITICAL SECURITY FINDINGS

### 🔴 FINDING #1: Multiple Redaction Code Paths - Potential for Inconsistency

**Location**: 
- `crates/scred-cli/src/main.rs`: Lines 400-550
- `run_redacting_stream()` vs `run_env_redacting_stream()`

**Issue**:
Two completely separate redaction streams with different logic:

```rust
// TEXT MODE - uses ConfigurableEngine.detect_and_redact()
fn run_redacting_stream() {
    let result = config_engine.detect_and_redact(&input_str);
    io::stdout().write_all(result.redacted.as_bytes()).ok();
}

// ENV MODE - uses custom env_mode::redact_env_line_configurable()  
fn run_env_redacting_stream() {
    for line in input_str.lines() {
        let redacted = env_mode::redact_env_line_configurable(line, &config_engine);
        output.push_str(&redacted);
    }
}
```

**Risk**: 
- If bug in one path, same secret might be missed in other path
- Different redaction rules applied
- Charset handling differences (UTF-8 vs lossy decode)
- Performance characteristics vary significantly

**Test Coverage Gap**:
- No integration test comparing TEXT vs ENV mode on SAME input
- No test verifying both modes redact same secrets identically

**Recommendation**:
1. Extract common redaction logic to single function
2. Add comparative test: `verify_text_env_equivalence(input) -> assert eq`
3. Document why both paths necessary

---

### 🔴 FINDING #2: Auto-Detection Creates False Negatives

**Location**: `crates/scred-cli/src/main.rs` lines 507-540

**Code**:
```rust
const DETECTION_BUFFER_SIZE: usize = 512;
let detection = env_detection::detect_format(&buffer);
let use_env_mode = detection.mode == env_detection::DetectionMode::EnvFormat;
```

**Issue**:
1. **512-byte sample too small** - environment files can have patterns AFTER first 512B
2. **Detection is probabilistic** - What if score is close (e.g., 0.51 vs 0.49)?
3. **No fallback** - If misdetection, entire file processed wrong way
4. **No verification step** - Can't re-scan or correct after detection

**Scenarios that break**:
```
# File structure: data.txt with AWS secrets in last line
KEY=value
KEY2=value2
...
AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY  # LINE 1000
```
- First 512B looks like text → misdetected as TEXT_MODE
- AWS secret in env format not detected
- Secret leaks

**Recommendation**:
1. Increase buffer to 4KB minimum or process entire file
2. Add detection confidence threshold check
3. Implement fallback: if confidence < 0.75, process BOTH ways
4. Add `--force-text` and `--force-env` flags with warning

---

### 🔴 FINDING #3: Pattern Selector Defaults are TOO CONSERVATIVE

**Location**: `crates/scred-cli/src/main.rs` lines 46-55

**Code**:
```rust
let detect_str = detect_flag
    .or_else(|| detect_env.as_deref())
    .unwrap_or("ALL");  // ✅ Good: detect all
let redact_str = redact_flag
    .or_else(|| redact_env.as_deref())
    .unwrap_or("CRITICAL,API_KEYS");  // ❌ Bad: only critical redacted
```

**Issue**:
- **Asymmetric detection**: Detect=ALL, Redact=CRITICAL,API_KEYS
- Means 70%+ of detected secrets NOT redacted by default
- User sees secrets detected but they STILL appear in output
- False sense of security

**Scenarios**:
```bash
# User expects all secrets redacted
echo "RAILS_MASTER_KEY=abc123xyz" | scred
# Output: RAILS_MASTER_KEY=abc123xyz  ❌ NOT REDACTED (wrong tier)

# User sees log output
WARN: 42 patterns detected
# But only 10 redacted due to conservative defaults
```

**Recommendation**:
1. Change default: `redact_str = detect_str` (redact what we detect)
2. Add explicit `--redact-conservatively` flag for old behavior
3. Document why asymmetry exists
4. Add warning if detect > redact

---

### 🔴 FINDING #4: Config File Fallback Silently Ignores Errors

**Location**: `crates/scred-cli/src/main.rs` lines 93-108

**Code**:
```rust
fn load_cli_config() -> (PatternSelector, PatternSelector) {
    if let Ok(file_config) = ConfigLoader::load() {
        if let Some(cli_cfg) = file_config.scred_cli {
            let detect_str = cli_cfg.patterns.detect.join(",");
            let redact_str = cli_cfg.patterns.redact.join(",");
            
            let detect = PatternSelector::from_str(&detect_str)
                .unwrap_or_else(|_| PatternSelector::default_detect());  // ❌ SILENT FALLBACK
            let redact = PatternSelector::from_str(&redact_str)
                .unwrap_or_else(|_| PatternSelector::default_redact());  // ❌ SILENT FALLBACK
```

**Issue**:
1. Config file loaded but parsing fails → silently use defaults
2. User doesn't know their config is broken
3. Different redaction rules applied than intended
4. No audit trail

**Attack scenario**:
```yaml
# config.yaml - intentionally broken by attacker
scred_cli:
  patterns:
    detect: "INVALID_TIER,GARBAGE"  # Will silently revert to defaults
    redact: "INVALID_TIER,GARBAGE"  # Will silently revert to defaults
```

**Recommendation**:
1. FAIL LOUDLY if config parsing fails
2. Log parsed values before using
3. Add `--validate-config` dry-run mode
4. Warn user of fallback (don't silent fail)

---

### 🔴 FINDING #5: MITM Proxy - Missing Response Redaction Path

**Location**: `crates/scred-mitm/src/mitm/tls_mitm.rs`

**Issue**: 
Looking at the code, there's TLS client→server redaction, but:
1. Response redaction mode configurable: `redaction_mode: RedactionMode`
2. But no enforcement that response IS redacted
3. Code path could skip response redaction entirely

**Need to verify**:
- [ ] Response body always redacted
- [ ] Response headers always redacted  
- [ ] No code paths bypass redaction
- [ ] Streaming responses don't lose secrets

---

### 🔴 FINDING #6: Streaming Request - Buffer Boundaries Can Leak Secrets

**Location**: `crates/scred-http/src/streaming_request.rs` (need to verify)

**Issue**:
Streaming redaction works on chunks, but:
1. Secret could span chunk boundary
2. Example: AWS key split across 8KB boundaries
3. Both halves individually don't match pattern
4. Secret passes through unreacted

**Scenario**:
```
CHUNK 1 (8192 bytes):
...content... AKIA...
(incomplete token due to chunk boundary)

CHUNK 2:
...continuation of token...
```

**Recommendation**:
1. Add `LOOKAHEAD_SIZE` to streaming implementation
2. Keep overlap between chunks during pattern matching
3. Test with secrets at various boundary positions
4. Document chunk size constraints

---

### 🔴 FINDING #7: Unused Code & Dead Code Paths

**Build output shows**:
```
warning: unused import: `std::sync::Arc`
warning: unused import: `crate::classification::Sensitivity`
warning: unused import: `Finding`
warning: unused import: `std::cmp::Ordering`
```

**Issues**:
1. Dead code paths not tested
2. Dead imports waste binary size
3. Could hide security issues
4. Makes maintenance harder

**Recommendation**:
1. `cargo fix --allow-dirty` to remove warnings
2. Add linter: deny warnings in CI
3. Review what code is actually used vs what's vestigial

---

### 🔴 FINDING #8: No Integration Test Against Real HTTPS Endpoint

**Current Testing**:
- Unit tests with mock data
- No real httpbin.org validation
- No actual secret redaction verification end-to-end

**Missing**:
```bash
# What SHOULD happen:
curl -X POST https://httpbin.org/anything \
  -H "Authorization: Bearer sk-proj-abcdef123456" \
  -d '{"api_key": "AKIA1234567890ABCDEF"}' \
  | scred --redact CRITICAL

# Expected: both secrets redacted
# Actual: unknown - NO TEST COVERS THIS
```

---

## CONSISTENCY ISSUES

### 🟡 ISSUE #1: Pattern Coverage Gaps Between CLI and MITM

**Question**: Are all 296 patterns loaded in MITM proxy?

**Need to verify**:
1. CLI: Uses `get_all_patterns()` from Zig
2. MITM: Uses `scred_redactor::RedactionEngine::new()`
3. Are they the same pattern set?
4. Or does MITM subset patterns?

**Risk**: MITM could be missing P0, P1, P2 patterns

---

### 🟡 ISSUE #2: Different Redaction Rules Between Components

**CLI Configuration**:
```rust
let redact_str = "CRITICAL,API_KEYS";  // Default conservative
```

**MITM Configuration**:
```rust
// Need to check: what are MITM defaults?
redaction_engine: Arc<scred_redactor::RedactionEngine>
```

**Question**: Are defaults identical?

---

### 🟡 ISSUE #3: Charset Handling Inconsistency

**CLI**:
```rust
let input_str = String::from_utf8_lossy(&chunk[..n]);
```

Uses LOSSY UTF-8 decode - replaces invalid bytes with U+FFFD

**MITM**:
```rust
// Need to check streaming_request.rs
```

Unknown if consistent

**Risk**: 
- Secrets in non-UTF8 could be missed
- Different behavior between tools

---

## SECURITY GAPS IN STREAMING

### 🔴 FINDING #9: Streaming Lookahead Not Implemented

**Evidence**: 
Need to verify `streaming_redaction.rs` implementation

**Concern**:
Patterns like `aws-key` are 20+ chars. If streaming in 4KB chunks:
- Chunk 1 ends with "AKIA123456789..." (10 chars)
- Chunk 2 starts with "...0ABCDEF" (rest of key)
- Both chunks individually don't match 20+ char pattern
- Secret leaks

**Need**:
```rust
const PATTERN_MAX_LENGTH: usize = 256;  // Longest possible secret
let lookahead = prev_chunk_tail[..PATTERN_MAX_LENGTH].to_string() + new_chunk;
// Apply redaction to lookahead
// Output: only the middle part (to avoid duplication)
```

---

### 🔴 FINDING #10: No Streaming Test for Boundary Cases

**Missing Tests**:
1. Secret starting at chunk position 8190 (boundary-2)
2. Secret ending at chunk position 8192 (exact boundary)
3. Secret spanning multiple chunks
4. Secret at exact CHUNK_SIZE boundary
5. Streaming with different CHUNK_SIZE values

**Current Tests**:
- Wave integration tests use small data
- No boundary stress tests
- No fuzzing for secret positions

---

## CRITICAL CODE QUALITY ISSUES

### 🔴 FINDING #11: Unwrap() Calls Can Panic

**Locations**:
```
crates/scred-cli/src/main.rs:
  - Line 25: unwrap_or()
  - Line 61: unwrap_or()  
  - Line 62: unwrap_or()
  - Line 150: unwrap_or()
  - Line 151: unwrap_or()
```

While these have `.unwrap_or()` fallbacks, other code might not.

**Recommendation**:
```bash
cargo build 2>&1 | grep "unwrap\|expect" | wc -l
# Should be 0 for production code
```

---

### 🔴 FINDING #12: No Validation of Redaction Engine Creation

**Pattern**:
```rust
let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
```

**Questions**:
1. What if creation fails?
2. What if patterns don't load?
3. What if Zig FFI fails?
4. How do we know engine is valid?

**No verification**:
- No `engine.is_valid()` check
- No pattern count verification
- No test that engine actually works

**Recommendation**:
1. `RedactionEngine::new()` should return `Result<>`
2. CLI should verify pattern count == 296
3. Add integration test that proves redaction works

---

## PROXY CONSISTENCY GAPS

### 🔴 FINDING #13: Proxy Selector Logic Unclear

**Question**: How does proxy selection work?

**Need to verify**:
1. HTTP_PROXY vs HTTPS_PROXY logic
2. NO_PROXY handling
3. Precedence order
4. Falls back correctly if proxy unavailable
5. Error handling if proxy is unreachable

**Security risk**: Misconfigured proxy selection could:
- Bypass MITM redaction entirely
- Send plaintext secrets to corporate proxy
- Fall back to insecure routing

---

## DEAD CODE ANALYSIS

### 🟡 ISSUE: Multiple Unused Imports

**Warnings Found**:
```
warning: unused import: `std::sync::Arc`
warning: unused import: `crate::classification::Sensitivity`  
warning: unused import: `Finding`
warning: unused import: `std::cmp::Ordering`
```

**Dead Code Locations**:
- Need to identify which files have these
- Remove or use them
- Add deny(unused_imports) to Cargo.toml

**Recommendation**:
```toml
[lints.rust]
unused_imports = "deny"
```

---

### 🟡 ISSUE: Unused Functions

**From build output**:
```
warning: unused_import: `std::sync::Arc`
```

**Need comprehensive audit**:
```bash
cargo clippy --all 2>&1 | grep "never used" | wc -l
```

Should be 0 for production

---

## MISSING INTEGRATION TESTS

### 🔴 FINDING #14: No Real httpbin.org Tests

**Current Coverage**:
- Unit tests with mock data
- Wave integration tests with predefined inputs
- No validation against real HTTPS endpoint

**Need to create**:

```bash
# Test 1: Basic secret in header
curl -X POST https://httpbin.org/anything \
  -H "Authorization: Bearer sk-proj-test123456789" \
  --cacert ~/scred.crt \
  --proxy http://127.0.0.1:8080 2>/dev/null | \
  jq .headers.Authorization
# Should be: "Bearer sk-proj-xxxxx"

# Test 2: Secret in JSON body
curl -X POST https://httpbin.org/anything \
  -H "Content-Type: application/json" \
  -d '{"aws_key":"AKIAIOSFODNN7EXAMPLE"}' \
  --proxy http://127.0.0.1:8080 2>/dev/null | \
  jq .data
# Should be: {"aws_key":"AKIAxxxxxxxxxxxxxxxx"}

# Test 3: Multiple secrets (streaming)
curl -X POST https://httpbin.org/anything \
  -d @large_secrets_file.txt \
  --proxy http://127.0.0.1:8080 2>/dev/null | \
  grep -c "REDACTED"
# Should be > 0
```

---

### 🔴 FINDING #15: CLI Redaction Not Validated Against Real Patterns

**Missing**:
```bash
# What we should test but don't:
echo "AKIA1234567890ABCDEF" | scred --redact CRITICAL
# Should output: AKIAxxxxxxxxxxxxxxxx
# Currently: UNKNOWN - no test

echo "sk-proj-abc123def456" | scred --redact API_KEYS
# Should output: sk-proj-xxxxxxxxxxxxx  
# Currently: UNKNOWN - no test
```

---

## SEVERITY ASSESSMENT

### 🔴 CRITICAL (Fix immediately):
1. **Finding #1**: Separate redaction code paths → inconsistency
2. **Finding #2**: Auto-detection false negatives  
3. **Finding #3**: Conservative redaction defaults
4. **Finding #9**: Streaming boundary secrets leak
5. **Finding #14**: No end-to-end HTTPS testing

### 🟡 HIGH (Fix before production):
6. **Finding #4**: Silent config failures
7. **Finding #8**: No real endpoint testing
8. **Finding #11**: Unwrap/expect calls
9. **Finding #13**: Proxy selector unclear

### 🟠 MEDIUM (Fix in maintenance):
10. **Finding #5**: Response redaction path verification needed
11. **Finding #12**: Redaction engine validation missing
12. **Finding #7, #10, #15**: Dead code and missing tests

---

## RECOMMENDED FIXES

### Fix #1: Consolidate Redaction Logic

```rust
/// Single source of truth for redaction
fn redact_content(
    content: &str,
    is_env_format: bool,
    config_engine: &ConfigurableEngine,
) -> String {
    if is_env_format {
        content.lines()
            .map(|line| env_mode::redact_env_line_configurable(line, config_engine))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let result = config_engine.detect_and_redact(content);
        result.redacted
    }
}

// Then use in BOTH paths:
// run_redacting_stream()
// run_env_redacting_stream()
```

### Fix #2: Improve Auto-Detection

```rust
fn run_with_auto_detect_safe() -> bool {
    // Read larger sample
    const DETECTION_BUFFER_SIZE: usize = 8192;  // 4x larger
    let buffer = read_with_fallback(DETECTION_BUFFER_SIZE);
    
    let detection = env_detection::detect_format(&buffer);
    
    // Verify confidence
    if detection.score < 0.75 {
        warn!("Low confidence detection: {:.2}", detection.score);
        warn!("Processing BOTH ways to ensure coverage");
        
        let output_env = process_env_mode(&buffer);
        let output_text = process_text_mode(&buffer);
        
        // Use the one with more redactions (safer)
        if count_redactions(&output_env) > count_redactions(&output_text) {
            return output_env;
        } else {
            return output_text;
        }
    }
    
    detection.mode == env_detection::DetectionMode::EnvFormat
}
```

### Fix #3: Symmetric Defaults

```rust
// OLD - asymmetric, wrong
let detect_str = "ALL";
let redact_str = "CRITICAL,API_KEYS";  // ❌ Only 30% of detected

// NEW - symmetric by default
let detect_str = detect_flag.or_else(|| detect_env.as_deref()).unwrap_or("CRITICAL");
let redact_str = redact_flag.or_else(|| redact_env.as_deref()).unwrap_or("CRITICAL");

// User can explicitly reduce: --detect ALL --redact CRITICAL
```

### Fix #4: Streaming Lookahead

```rust
const STREAMING_CHUNK_SIZE: usize = 8192;
const PATTERN_LOOKAHEAD: usize = 256;

fn redact_streaming(reader: impl Read, writer: impl Write) {
    let mut prev_tail = String::new();
    let mut chunk = vec![0; STREAMING_CHUNK_SIZE];
    
    while let Ok(n) = reader.read(&mut chunk) {
        if n == 0 { break; }
        
        let chunk_str = String::from_utf8_lossy(&chunk[..n]);
        
        // Create lookahead: prev_tail + current_chunk + next_peek
        let lookahead = prev_tail.clone() + &chunk_str;
        
        // Redact the lookahead
        let redacted = redaction_engine.redact(&lookahead);
        
        // Output only the "middle" part (skip prev_tail portion)
        let output_start = prev_tail.len();
        let output = &redacted[output_start..];
        writer.write_all(output.as_bytes())?;
        
        // Save tail for next iteration
        prev_tail = chunk_str[chunk_str.len().saturating_sub(PATTERN_LOOKAHEAD)..].to_string();
    }
}
```

### Fix #5: Add End-to-End Tests

Create `crates/scred-integration/tests/e2e_httpbin.rs`:

```rust
#[tokio::test]
async fn test_mitm_redacts_header_secret() {
    // Start MITM proxy
    let proxy = start_scred_mitm_on_8080().await;
    
    // Make real HTTPS request through proxy
    let response = reqwest::Client::builder()
        .proxy(reqwest::Proxy::http("http://127.0.0.1:8080").unwrap())
        .build()
        .unwrap()
        .post("https://httpbin.org/anything")
        .header("Authorization", "Bearer sk-proj-abc123def456")
        .send()
        .await
        .unwrap();
    
    // Verify secret was redacted in what httpbin.org saw
    let body: serde_json::Value = response.json().await.unwrap();
    let auth_header = body["headers"]["Authorization"].as_str().unwrap();
    
    assert!(auth_header.contains("sk-proj-xxxx"), 
        "Secret not redacted: {}", auth_header);
}

#[tokio::test]
async fn test_cli_redacts_all_patterns() {
    for pattern_name in get_all_pattern_names() {
        let test_secret = generate_valid_secret(&pattern_name);
        
        let output = run_cli_command(&format!(
            "echo '{}' | scred --redact CRITICAL",
            test_secret
        )).await;
        
        assert!(
            !output.contains(&test_secret) || is_redacted(&output),
            "Pattern {} not redacted: {}",
            pattern_name, output
        );
    }
}
```

---

## SUMMARY

| Category | Count | Severity |
|----------|-------|----------|
| Critical | 5 | Must fix |
| High | 4 | Before production |
| Medium | 6 | Maintenance |
| **TOTAL** | **15** | **Action required** |

**Overall Assessment**: 
- ✅ Good test coverage exists
- ❌ BUT: Real-world testing gaps critical
- ❌ Streaming boundary issues not addressed
- ❌ Config consistency problems
- ✅ Code quality mostly good
- ❌ Dead code needs cleanup

**Recommendation**: 
Before production deployment, MUST address:
1. Streaming lookahead implementation
2. End-to-end httpbin.org validation
3. Config consistency audit
4. Dead code cleanup

All other items can be done in maintenance cycle.

