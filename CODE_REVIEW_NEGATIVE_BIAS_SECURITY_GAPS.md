# 🔴 NEGATIVE BIAS CODE REVIEW: Security Gaps & Inconsistencies
## CLI, MITM, and Proxy Secret Detection/Redaction Paths

---

## CRITICAL SEVERITY ISSUES

### 1. **Pattern Selector Fallback Silently Allows Unredacted Secrets** 🔴

**Location**: `crates/scred-cli/src/main.rs:50-55`, `crates/scred-mitm/src/main.rs:80-90`

**Issue**: When `PatternSelector::from_str()` fails, the code falls back to defaults WITHOUT alerting the user.

```rust
// DANGEROUS: Silent fallback!
let redact_selector = PatternSelector::from_str(redact_str)
    .unwrap_or_else(|e| {
        warn!("Invalid SCRED_REDACT_PATTERNS '{}': {}", redact_str, e);
        PatternSelector::default_redact()  // ← Falls back to defaults silently!
    });
```

**Why It's Bad**:
- User sets `--redact CRITICAL` expecting only CRITICAL secrets to be redacted
- If typo occurs: `--redact CRIITICAL` (misspelled)
- Code silently falls back to `default_redact()` (CRITICAL, API_KEYS, PATTERNS)
- User thinks they're redacting only CRITICAL, but PATTERNS are also redacted
- **Opposite problem**: User sets `--redact NONE` to disable redaction
  - If there's a parsing error, it falls back to default (CRITICAL + API_KEYS redacted!)
  - User's explicit intent to disable is overridden silently

**Attack Scenario**:
```bash
# User tries to whitelist only CRITICAL:
scred --redact CRITICAL --detect NONE   # Intent: Only redact CRITICAL
# But if there's a parsing error:
→ Actually redacts: CRITICAL, API_KEYS, PATTERNS  # DEFAULT!
```

**Fix**: Exit with error instead of silently falling back:
```rust
let redact_selector = PatternSelector::from_str(redact_str)
    .map_err(|e| {
        eprintln!("❌ ERROR: Invalid redact selector '{}': {}", redact_str, e);
        std::process::exit(1);
    })?;
```

---

### 2. **Inconsistent Selector Precedence Across CLI, MITM, and Proxy** 🔴

**Locations**:
- CLI: `crates/scred-cli/src/main.rs:45`
- MITM: `crates/scred-mitm/src/main.rs:118-130`
- Proxy: `crates/scred-proxy/src/main.rs:68-95`

**Issue**: Different precedence orders and fallback behaviors:

| Component | Precedence | Fallback |
|-----------|-----------|----------|
| **CLI** | `CLI flag > ENV > hardcoded "CRITICAL,API_KEYS,PATTERNS"` | Falls back to hardcoded string |
| **MITM** | `File config > CLI flag > ENV > Config::load()` | Falls back to Config::load() |
| **Proxy** | `File config > ENV > Code default` | No CLI flag support! |

**Problem**: User expects same behavior everywhere but gets different results:

```bash
# Set environment variable for redaction
export SCRED_REDACT_PATTERNS=CRITICAL

# Test all three:
scred --env-mode < file.env              # Uses CRITICAL (respects ENV) ✓
./target/release/scred-mitm             # Uses Config::load() defaults? Or ENV?
./target/release/scred-proxy            # Doesn't support CLI flags!
```

**Attack Scenario**:
```bash
# User deploys MITM with specific redaction rules via CLI:
./scred-mitm --redact CRITICAL,API_KEYS

# But simultaneously, config file exists with different rules:
cat ~/.scred/config.yaml
  scred_mitm:
    redaction:
      patterns:
        redact: [CRITICAL, API_KEYS, INFRASTRUCTURE]  # More permissive!

# MITM uses FILE CONFIG (precedence), not CLI flag!
# User thinks INFRASTRUCTURE is NOT redacted, but it IS
# This could expose secrets they wanted to hide!
```

**Fix**: Unified precedence: `CLI > ENV > Config File > Hardcoded Default`

---

### 3. **MITM Pattern Selectors Not Applied in All Code Paths** 🔴

**Location**: `crates/scred-mitm/src/mitm/` - Multiple files

**Issue**: The `detect_patterns` and `redact_patterns` fields in `UpstreamConfig` are defined but NOT verified to be used in all redaction paths.

```rust
// In config.rs: Fields defined but skip_serde
#[serde(skip)]
pub detect_patterns: PatternSelector,
#[serde(skip)]
pub redact_patterns: PatternSelector,
```

**But where are they used?**

```bash
grep -r "detect_patterns\|redact_patterns" crates/scred-mitm/src/mitm/
# Result: NO MATCHES!
```

**Why It's Bad**:
- Pattern selectors are configured via CLI flags
- But the code never passes them to the `ConfigurableEngine`!
- This means pattern filtering is completely ignored in MITM!

```rust
// In h2_mitm_handler.rs (hypothetical redaction):
let redacted = engine.redact(body);  // ← Uses DEFAULT patterns, ignores config!
```

**Attack Scenario**:
```bash
# User starts MITM with specific pattern filtering:
./scred-mitm --redact CRITICAL

# User thinks API_KEYS won't be redacted
# But ConfigurableEngine still redacts API_KEYS (uses defaults)
# Sensitive secrets get redacted unexpectedly!
```

**Verification Needed**:
```bash
grep -n "ConfigurableEngine" crates/scred-mitm/src/mitm/*.rs
grep -n "detect_patterns\|redact_patterns" crates/scred-mitm/src/mitm/*.rs
```

If pattern selectors aren't passed to ConfigurableEngine constructors, this is a BUG.

---

### 4. **Env-Mode Doesn't Respect Pattern Selectors** 🔴

**Location**: `crates/scred-cli/src/env_mode.rs`

**Issue**: The `redact_env_line_configurable()` function passes a ConfigurableEngine but the pattern selector is NEVER extracted or validated.

```rust
pub fn redact_env_line_configurable(line: &str, config_engine: &ConfigurableEngine) -> String {
    redact_env_line_generic(line, |v| config_engine.redact_only(v))
}
```

**Problem**: If `config_engine` was created with restrictive pattern selectors (e.g., `--redact CRITICAL`), the env_mode will still use that engine. BUT:

1. **No validation** that the pattern selector matches user intent
2. **No logging** of which patterns are being used
3. **No guarantee** that the engine has the right selector set

**Attack Scenario**:
```bash
# User expects env-mode to respect --redact CRITICAL:
env | scred --env-mode --redact CRITICAL

# If CRITICAL selector isn't properly passed to engine:
→ API_KEYS still redacted (unexpected)

# Or if selector defaults kick in:
→ Engine uses hardcoded default (CRITICAL,API_KEYS,PATTERNS)
→ User's --redact CRITICAL is ignored!
```

---

## HIGH SEVERITY ISSUES

### 5. **No Validation That Pattern Selector Actually Excludes Patterns** 🟠

**Location**: `crates/scred-http/src/pattern_selector.rs:171-199`

**Issue**: The `matches_pattern()` function has no test coverage for all selector types.

```rust
pub fn matches_pattern(&self, pattern_name: &str, pattern_tier: PatternTier) -> bool {
    match self {
        Self::Tier(tiers) => tiers.contains(&pattern_tier),  // ← What if tiers is empty?
        Self::All => true,
        Self::None => false,
        Self::Whitelist(patterns) => patterns.contains(pattern_name),  // ← What if empty?
        Self::Blacklist(patterns) => !patterns.contains(pattern_name), // ← What if ALL in list?
        Self::Wildcard(patterns) => {
            patterns.iter().any(|p| self.wildcard_match(pattern_name, p))
        }
        Self::Regex(patterns) => {
            // For now, simple prefix matching (regex requires regex crate)
            patterns.iter().any(|p| pattern_name.to_lowercase().contains(&p.to_lowercase()))
        }
    }
}
```

**Problems**:
1. **Empty selector**: If `Tier(vec![])`, nothing gets redacted. Silent failure!
2. **Regex fallback**: Uses simple `contains()` instead of regex! `regex:sk-` might match `ask-` (wrong!)
3. **Wildcard matching**: Uses simple glob matching - no validation of glob syntax

**Test Gap**:
```bash
# These should NOT match (or should error):
--redact ""              # Empty selector
--redact "UNKNOWN_TIER"  # Invalid tier name
--redact "regex:"        # Empty regex
--redact "regex:["       # Invalid regex
```

---

### 6. **Pattern Name Mismatch Detection Not Enforced** 🟠

**Location**: `crates/scred-http/src/pattern_metadata.rs`, `crates/scred-redactor/src/redactor.rs`

**Issue**: If regex detector returns a pattern name that doesn't exist in `pattern_metadata.rs`, it silently defaults to `Patterns` tier.

```rust
// In redactor.rs: Pattern names must match pattern_metadata.rs
("jwt", r"...", "jwt"),  // Must be exact match to "jwt" in metadata!

// But if typo: "jw" (missing 't')
// Result: Pattern name not found in metadata
// Fallback: get_pattern_tier("jw") → PatternTier::Patterns (default)
// User's selector "--redact CRITICAL" won't match "Patterns" tier!
// Secret not redacted!
```

**Attack Scenario**:
```bash
# Pattern detector returns misspelled name "jwt-token" (typo)
# Pattern metadata only has "jwt"
# get_pattern_tier("jwt-token") → returns Patterns tier (default)
# User set --redact CRITICAL (doesn't include Patterns)
# JWT NOT REDACTED! ❌
```

**Current Fallback Code**:
```rust
pub fn get_pattern_tier(pattern_type: &str) -> PatternTier {
    PATTERN_TIERS.get(pattern_type)
        .copied()
        .unwrap_or(PatternTier::Patterns)  // ← SILENT fallback to Patterns!
}
```

**Fix**: Return error or panic on unknown pattern instead of silently defaulting:
```rust
pub fn get_pattern_tier(pattern_type: &str) -> Result<PatternTier> {
    PATTERN_TIERS.get(pattern_type)
        .copied()
        .ok_or_else(|| anyhow!("Unknown pattern type: {}", pattern_type))
}
```

---

### 7. **CLI Auto-Detection Mode Not Enforcing Pattern Selectors** 🟠

**Location**: `crates/scred-cli/src/main.rs:193-220`

**Issue**: Auto-detection mode (`--auto-detect` or no mode specified) may switch between text-mode and env-mode, but pattern selectors are NOT re-validated:

```rust
if auto_detect_enabled {
    // Auto-detect based on first chunk of input
    // But which pattern selector applies to detected mode?
    // The selector was created before mode detection!
    
    let mode = detect_mode_from_chunk(&chunk);  // Returns Text or Env
    // Now use existing selector - which one? They might be different!
}
```

**Problem**: User sets `--redact API_KEYS --auto-detect`
- If detected as env-mode: env_mode uses engine with API_KEYS selector ✓
- If detected as text-mode: text_mode uses engine with API_KEYS selector ✓
- **But what if they set different selectors for different modes?**
  ```bash
  scred --detect ALL --redact CRITICAL \
        --detect-env CRITICAL --redact-env API_KEYS \
        --auto-detect
  ```
  Which selector applies? Not specified in code!

---

## MEDIUM SEVERITY ISSUES

### 8. **No Defense Against Selector Type Confusion** 🟡

**Location**: `crates/scred-http/src/pattern_selector.rs:132-165`

**Issue**: `from_str()` tries multiple parsing strategies with loose matching:

```rust
pub fn from_str(input: &str) -> Result<Self, String> {
    match input.to_lowercase().as_str() {
        "all" => Ok(Self::All),
        "none" => Ok(Self::None),
        _ if input.starts_with("regex:") => { /* ... */ },
        _ if input.contains(',') => {
            // Try parsing as tiers FIRST
            if let Ok(tiers) = PatternTier::parse_list(input) {
                return Ok(Self::Tier(tiers));
            }
            // Fall back to wildcard patterns
            let patterns: Vec<String> = input.split(',').map(|s| s.trim().to_string()).collect();
            Ok(Self::Wildcard(patterns))
        }
        _ if input.contains('-') && !input.contains('*') => {
            // Single pattern - assumed to be wildcard!
            Ok(Self::Wildcard(vec![input.to_string()]))
        }
        // ...
    }
}
```

**Problems**:
1. **Ambiguity**: `"aws-akia"` - Is this a tier name or a pattern name?
   - Contains `-` → Parsed as Wildcard pattern
   - But `"aws-akia"` is also a pattern name in metadata!
   - **Result: Inconsistent behavior depending on pattern_metadata contents**

2. **Silent Type Conversion**:
   ```bash
   --redact "aws-*"        # Intended: wildcard pattern
   --redact "aws-akia"     # Intended: specific pattern
   # But both parsed differently!
   ```

3. **No validation** of input against known patterns/tiers:
   ```bash
   --redact "CRITIKUL"     # Typo in tier name
   # Result: Parsed as pattern name via Whitelist
   # No error! Silent fallback to treating as pattern.
   ```

---

### 9. **Regex Selector Not Actually Using Regex** 🟡

**Location**: `crates/scred-http/src/pattern_selector.rs:192-200`

```rust
Self::Regex(patterns) => {
    // For now, simple prefix matching (regex requires regex crate)
    patterns.iter().any(|p| pattern_name.to_lowercase().contains(&p.to_lowercase()))
}
```

**Issue**: Comments say "regex" but implementation uses simple `contains()` matching!

**Dangerous Examples**:
```bash
--redact "regex:aws"       # Intended: ^aws.* (start with aws)
# Actual: Any pattern containing "aws"
# Matches: "aws-akia", "new-aws-token", "not_aws_key" ❌

--redact "regex:^sk-"      # Intended: ^sk- (start with sk-)
# Actual: Any pattern containing "^sk-"
# Matches: pattern named "sk-key", but also...
# NEVER matches! Pattern name doesn't contain "^sk-"
```

**Security Gap**:
```bash
# User thinks:
--redact "regex:^(?!aws)"   # Exclude aws patterns
# But code sees: "^(?!aws)" as literal substring
# Tries to match pattern names containing "^(?!aws)"
# Result: NO patterns matched! Nothing redacted!
```

---

### 10. **Streaming Mode Pattern Selector Not Passed Through** 🟡

**Location**: `crates/scred-cli/src/main.rs:260-280`

**Issue**: Streaming mode uses `StreamingRedactor` but doesn't verify that pattern selectors are applied:

```rust
if streaming_enabled {
    let streaming_redactor = StreamingRedactor::new(
        engine.clone(),
        // ← Pattern selector is NOT a parameter!
        // Streaming redactor uses engine defaults?
    );
    
    streaming_redactor.process_stream(&mut input, &mut output)?;
}
```

**Problem**: StreamingRedactor might not pass pattern selectors to underlying engine.

**Attack Scenario**:
```bash
# User processes large file with specific pattern filtering:
cat large_file.log | scred --streaming --redact CRITICAL

# But if StreamingRedactor ignores pattern selector:
# → Uses engine's default selector instead
# → API_KEYS also redacted (unexpected!)
```

---

### 11. **Proxy Per-Path Rules Not Enforced During Redaction** 🟡

**Location**: `crates/scred-proxy/src/main.rs:86-100`

**Issue**: Per-path redaction rules are parsed but never verified to be applied during actual request processing:

```rust
// Rules loaded:
let per_path_rules: Vec<PathRedactionRule> = proxy_cfg.rules.iter().map(|rule| {
    PathRedactionRule {
        path_pattern: rule.path.clone(),
        should_redact: rule.redact,  // True/false
        reason: rule.reason.clone(),
    }
}).collect();

// But where are they used?
// grep -r "per_path_rules" crates/scred-proxy/src/
// ← Might not be used in actual redaction!
```

**Attack Scenario**:
```yaml
# Config says:
scred_proxy:
  rules:
    - path: "/api/admin/*"
      redact: false  # Don't redact /api/admin/* paths
      reason: "Admin API - pre-sanitized"
```

```bash
# But if rules aren't enforced in actual redaction:
curl "http://localhost:9999/api/admin/secrets"
→ Secrets still redacted! (ignores per-path rule)
```

---

### 12. **No Cross-Component Pattern Consistency Check** 🟡

**Location**: Across all three binaries

**Issue**: No validation that CLI, MITM, and Proxy use consistent pattern metadata:

```bash
# Potential misalignment scenarios:

# Scenario 1: Pattern added to Rust redactor but not Zig
# Scenario 2: Pattern tier changed in metadata but CLI defaults not updated
# Scenario 3: Pattern removed from detector but selector still references it

# No runtime validation!
# Result: Different components detect/redact different secrets!
```

---

## MEDIUM SEVERITY ISSUES (continued)

### 13. **Multiline Secret Continuation Not Handled Across Components** 🟡

**Location**: All three binaries (CLI, MITM, Proxy)

**Issue**: Line-by-line processing misses secrets that span multiple lines:

**CLI**:
```rust
while let Some(line) = reader.lines().next().transpose()? {
    let redacted = redact_env_line(&line, ...);  // Per-line processing!
    println!("{}", redacted);
}
```

**MITM**:
```rust
// Similar line-by-line processing for HTTP bodies
let lines: Vec<&str> = body.split('\n').collect();
for line in lines {
    // Redact each line independently
}
```

**Proxy**:
```rust
// Same issue in reverse proxy
```

**Attack Scenario**:
```
Multiline secret in request:
POST /api/login
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9
 .eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ
 .SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c

# Each line processed independently:
Line 1: "Bearer eyJh..." → JWT pattern matches ✓ Redacted
Line 2: " .eyJ..." → Doesn't start with "eyJ" → Pattern doesn't match ❌ NOT redacted
Line 3: " .Sfk..." → Doesn't start with "eyJ" → Pattern doesn't match ❌ NOT redacted

# Result: JWT token exposed across 3 lines!
```

---

### 14. **No Detection of Secrets Hidden in URL Encoding** 🟡

**Location**: All three binaries

**Issue**: Pattern detector works on plaintext only. URL-encoded or escaped secrets not detected:

```
Examples:
?api_key=sk%2Dproj%5F123456  # sk-proj_123456 encoded
"api_key":"sk-proj\u005f123456"  # Unicode escaped
{Authorization: Bearer%20eyJ...}  # Space-encoded JWT
```

**Why It's Bad**:
```bash
# Secret in URL encoding passes through undetected
# Pattern detector sees: sk%2Dproj...
# Pattern matcher looks for: sk-proj...
# NO MATCH! Secret exposed!
```

---

### 15. **HTTP Response Body Redaction Inconsistent** 🟡

**Location**: `crates/scred-mitm/src/main.rs:65-75` (old code reference)

**Issue**: Response redaction is optional/configurable, but proxy doesn't always apply it:

```rust
// In config:
#[serde(default = "default_redact_responses")]
pub redact_responses: bool,  // Optional!

// But different components might not respect this flag
```

**Problem**:
```bash
# Config says redact_responses = false
scred-mitm --no-redact-responses

# User expects responses NOT redacted
# But if code path doesn't check this flag:
→ Responses still redacted!
```

---

### 16. **No Mutual Exclusivity Check: --detect vs --redact** 🟡

**Location**: `crates/scred-cli/src/main.rs`, `crates/scred-mitm/src/main.rs`

**Issue**: Code allows contradictory configurations:

```bash
# Legal but contradictory:
scred --detect NONE --redact ALL  # Detect nothing but redact everything?
scred --detect API_KEYS --redact INFRASTRUCTURE  # Detect API_KEYS, redact different tier?

# Logical interpretation unclear!
# Should redact be subset of detect? Or independent?
```

**Attack Scenario**:
```bash
# User tries to selectively redact:
scred --detect CRITICAL --redact API_KEYS

# Intended: Detect CRITICAL, but redact API_KEYS (nonsensical)
# Actual behavior: Undefined!
# Might redact API_KEYS even though not detected
# Or might redact nothing
# Or might crash with error
```

---

## LOW SEVERITY ISSUES

### 17. **Pattern Selector `description()` Method Not Accurate** 🟡

**Location**: `crates/scred-http/src/pattern_selector.rs:235-245`

```rust
pub fn description(&self) -> String {
    match self {
        Self::Tier(tiers) => {
            let tier_names: Vec<String> = tiers.iter().map(|t| format!("{:?}", t)).collect();
            format!("Tiers: {}", tier_names.join(", "))
        }
        Self::All => "All patterns".to_string(),
        Self::None => "No patterns".to_string(),
        Self::Whitelist(patterns) => format!("Whitelist: {:?}", patterns),
        Self::Blacklist(patterns) => format!("Blacklist: {:?}", patterns),
        Self::Wildcard(patterns) => format!("Wildcard: {:?}", patterns),
        Self::Regex(patterns) => format!("Regex: {:?}", patterns),
    }
}
```

**Problem**: Uses `{:?}` debug format which might not be user-friendly:
```
Output: "Wildcard: [\"aws-*\", \"github-*\", \"sk-*\"]"
        Tier(vec![Critical, ApiKeys])  # Not human readable!
```

**Why It's Bad**: Logging and debugging show unreadable format. Users can't verify their config is correct.

---

### 18. **No Validation of TLS Certificate Pattern Selector** 🟡

**Location**: `crates/scred-mitm/src/mitm/tls_acceptor.rs`

**Issue**: TLS certificate generation doesn't consider pattern selectors. Certificates generated the same way regardless of which patterns will be redacted.

**Why It Matters**: If pattern selector changes certificates should potentially be regenerated, but they're not.

---

### 19. **Proxy Connection Pool Size Not Configurable Per Pattern Selector** 🟡

**Location**: `crates/scred-proxy/src/main.rs:40-50`

**Issue**: Connection pool size is fixed regardless of expected redaction load:

```rust
max_connections: usize,  // Fixed value

// If all patterns redacted: High CPU usage, smaller pool needed
// If few patterns redacted: Low CPU usage, larger pool possible
// But pool size never adjusts!
```

---

### 20. **No Audit Log of Pattern Selector Changes** 🟡

**Location**: All three binaries

**Issue**: When pattern selectors change (via CLI, ENV, or config), no persistent audit trail is created.

**Attack Scenario**:
```bash
# MITM started with: --redact CRITICAL,API_KEYS
# Later changed to: --redact NONE (disable redaction)
# No audit log! No way to detect when redaction was disabled!
# Secrets now flowing unredacted with no evidence.
```

---

## INCONSISTENCY MATRIX

| Feature | CLI | MITM | Proxy | Consistent? |
|---------|-----|------|-------|-------------|
| **Precedence: CLI > ENV > File > Default** | ✓ Explicit | ✓ Explicit | ✗ Missing CLI | ❌ NO |
| **Error on invalid selector** | ✓ Warns only | ✓ Warns only | ✓ Warns only | ⚠️ PARTIAL |
| **Pattern selector validation** | ✗ None | ✗ None | ✗ None | ❌ NO |
| **Per-path redaction rules** | ✗ N/A | ✗ Not enforced | ⚠️ Defined but untested | ❌ NO |
| **Multiline secret handling** | ✗ Line-by-line | ✗ Line-by-line | ✗ Line-by-line | ✓ Consistent (but wrong!) |
| **Response body redaction** | ✗ N/A | ⚠️ Optional | ⚠️ Optional | ⚠️ PARTIAL |
| **Streaming mode selector** | ✗ Not passed | ✗ N/A | ✗ N/A | ❌ NO |
| **URL-encoded secret detection** | ✗ None | ✗ None | ✗ None | ✓ Consistent (but wrong!) |
| **Audit logging** | ✗ None | ✗ None | ✗ None | ✓ Consistent (but wrong!) |

---

## SUMMARY: ATTACK SCENARIOS

### Scenario 1: Silent Fallback to Default Redaction
```bash
# User deploys MITM with restricted redaction:
./scred-mitm --redact CRITICAL

# Typo in environment variable:
export SCRED_REDACT_PATTERNS="CRIITICAL"  # Typo!

# Code ignores CLI flag, uses fallback default:
# API_KEYS also redacted (unexpected)
# User doesn't notice - logs show API_KEYS being redacted
```

### Scenario 2: Pattern Selector Ignored in MITM
```bash
# User starts MITM with limited redaction:
./scred-mitm --redact CRITICAL

# But code never passes selector to ConfigurableEngine
# Engine uses default selector (CRITICAL, API_KEYS, PATTERNS)
# API_KEYS unexpectedly redacted

# User sees logs showing only CRITICAL patterns detected
# But actually more patterns redacted than shown in logs!
```

### Scenario 3: Per-Path Rules Not Enforced
```yaml
# Config excludes sensitive path from redaction:
scred_proxy:
  rules:
    - path: "/internal/*"
      redact: false
```

```bash
# But if per_path_rules not actually checked during redaction:
curl http://localhost:9999/internal/secret
# Secret still redacted! Rule ignored!
```

### Scenario 4: Unknown Pattern Type Silently Defaults
```rust
// Typo in redactor.rs:
("jwt_token", r"...", "jwt_token")  // Should be "jwt"

// Pattern metadata doesn't have "jwt_token"
// get_pattern_tier("jwt_token") silently returns Patterns tier
// User set --redact CRITICAL (doesn't include Patterns)
// JWT not redacted!
```

### Scenario 5: Multiline Secret Exposed
```
Request with continuation:
Authorization: Bearer eyJhbGc...
 iOiJIUzI1NiJ9.eyJzdWI...
 iOiJKb2huIn0.signature...

# Each line processed independently
# First line redacted, others not
# JWT exposed!
```

---

## RECOMMENDATIONS

### Immediate (Critical Fixes)

1. **Change fallback behavior to error**: Exit with error instead of silent fallback
2. **Pass pattern selectors to ConfigurableEngine**: Verify in MITM handler
3. **Add pattern selector validation**: Test all unknown patterns return error
4. **Enforce per-path rules**: Add runtime checks to proxy
5. **Unify precedence order**: CLI > ENV > File > Hardcoded Default for all three

### Short-term (High Priority)

6. **Implement regex selector properly**: Use actual regex matching
7. **Add audit logging**: Track selector changes with timestamps
8. **Validate mutual exclusivity**: warn on contradictory detect/redact settings
9. **Support URL-decoding**: Detect secrets in encoded URLs
10. **Fix streaming mode**: Pass selectors through to StreamingRedactor

### Medium-term (Good to Have)

11. **Implement multiline buffering**: Support secrets spanning lines
12. **Add cross-component consistency check**: Verify all use same patterns
13. **Response body redaction consistency**: Make behavior uniform across MITM/Proxy
14. **Configuration validation**: Test all combinations of selectors at startup
15. **Improved error messages**: Show what selector would have matched

---

## TESTING NEEDED

```bash
# Test 1: Invalid selector fallback
scred --redact INVALID_TIER < test.txt
# Expected: Error exit
# Actual: Probably falls back to default

# Test 2: MITM selector application
./scred-mitm --redact CRITICAL
# curl localhost:8888/secret?api_key=sk-123456
# Expected: sk-123456 NOT redacted
# Actual: Probably still redacted

# Test 3: Per-path rule enforcement
# Set rule: /admin/* should not redact
# Request: GET /admin/secret?api_key=sk-123456
# Expected: sk-123456 NOT redacted
# Actual: Unknown (rules may not be checked)

# Test 4: Pattern selector validation
scred --redact "regex:[invalid-regex"
# Expected: Error or warning
# Actual: Probably silently parsed as literal string

# Test 5: Multiline secret detection
echo "Bearer eyJ...\n.eyJ...\n.signature" | scred
# Expected: Entire JWT redacted
# Actual: Only first line redacted

# Test 6: URL-encoded secret
echo "?api_key=sk%2D123456" | scred
# Expected: Secret detected and redacted
# Actual: Probably not detected (pattern looks for "sk-", not "sk%2D")
```

---

## CONCLUSION

**Current State**: Multiple critical gaps in pattern selector enforcement across CLI, MITM, and proxy. Selectors are partially implemented but not consistently enforced, creating scenarios where:

- Users think they've restricted redaction but haven't
- Pattern selectors are configured but ignored
- Secrets remain unredacted due to typos or parsing failures
- Different components behave differently despite same input

**Risk Level**: **CRITICAL** - Intended redaction policies may silently fail, causing secrets to be exposed or to pass through unredacted.

**Recommendation**: Treat pattern selector enforcement as security-critical code path. Add comprehensive test coverage and validation at every entry point.
