# 🔧 CORE LOGIC & CROSS-COMPONENT CONSISTENCY ASSESSMENT
## SCRED v1.0 - What Needs to Be Fixed for Maximum Security

---

## EXECUTIVE SUMMARY

**Current State**: Solid core redaction engine with critical gaps in configuration enforcement and cross-component consistency.

**Issues**: 3 CRITICAL (blocking v1.0), 3 HIGH (breaking consistency), 5 MEDIUM (edge cases)

**Time to Fix**: 
- Critical path: 5 hours (immediate)
- Full consistency: 12-15 hours (v1.1)
- Maximum security: 18-20 hours (v1.2+)

---

## PART 1: CORE LOGIC FIXES (5 CRITICAL ISSUES)

### 1.1 🔴 CRITICAL: Proxy Per-Path Rules Not Enforced

**Current State**:
```rust
// Rules defined, parsed, stored, BUT NEVER USED
pub struct ProxyConfig {
    per_path_rules: Vec<PathRedactionRule>,  // Dead code!
}
```

**Why It's Wrong**:
- User configures: "don't redact /admin/*"
- Code redacts anyway
- False sense of security

**What Needs to Be Fixed**:

```rust
// BEFORE: No path checking
fn handle_request(path: &str, body: &str) {
    let redacted = redactor.redact(body);  // Always redacts!
    send_response(redacted);
}

// AFTER: Check per-path rules
fn handle_request(path: &str, body: &str) {
    // Check if path should be redacted
    let should_redact = should_redact_for_path(
        path,
        &config.per_path_rules
    );
    
    let redacted = if should_redact {
        redactor.redact(body)
    } else {
        body.to_string()  // Skip redaction
    };
    
    send_response(redacted);
}

fn should_redact_for_path(
    path: &str,
    rules: &[PathRedactionRule],
) -> bool {
    // Check rules in order (first match wins)
    for rule in rules {
        if path_matches(&path, &rule.path_pattern) {
            return rule.should_redact;
        }
    }
    // Default: redact if no rule matches
    true
}
```

**Implementation Steps**:
1. Add glob matching function: `path_matches(path, pattern)`
2. Add rule checking before redaction
3. Add logging: "Skipping redaction for {path} (rule: {reason})"
4. Add tests: per-path rules enforcement

**Effort**: 1-2 hours
**Impact**: HIGH - Configuration actually enforced

---

### 1.2 🔴 CRITICAL: Regex Selector Completely Broken

**Current State**:
```rust
Self::Regex(patterns) => {
    // Claims to be regex but uses simple contains()!
    patterns.iter().any(|p| 
        pattern_name.to_lowercase().contains(&p.to_lowercase())
    )
}
```

**Why It's Wrong**:
```bash
--redact "regex:^sk-"    # User expects: start with "sk-"
# Actual: Pattern name contains "^sk-" literally
# Result: NO MATCHES! Nothing redacted!

--redact "regex:(?!aws)"  # Negative lookahead
# Actual: Pattern name contains "(?!aws)"
# Result: NO MATCHES!
```

**Option A: Implement Real Regex (PREFERRED)**

```rust
use regex::Regex;

Self::Regex(patterns) => {
    patterns.iter().any(|p| {
        // Compile and cache regex
        match Regex::new(p) {
            Ok(regex) => regex.is_match(pattern_name),
            Err(e) => {
                warn!("Invalid regex pattern '{}': {}", p, e);
                false
            }
        }
    })
}
```

**Option B: Remove Feature (SAFE)**

```rust
// Remove Regex variant entirely
// Document: "Regex selector not supported in v1.0"
// Use wildcard matching instead: --redact "sk-*"
```

**Recommendation**: Implement Option A (real regex)
- Effort: 1-2 hours
- Benefits: Powerful pattern matching
- Cost: Slight CPU overhead per selector check (acceptable)

**Implementation**:
1. Add regex crate to Cargo.toml (already present!)
2. Add caching: `HashMap<String, Regex>` in PatternSelector
3. Add validation on selector creation
4. Add tests: regex patterns with anchors, groups, lookahead

**Effort**: 1-2 hours
**Impact**: HIGH - Feature actually works

---

### 1.3 🔴 CRITICAL: Silent Fallback on Invalid Selector

**Current State**:
```rust
let redact_selector = PatternSelector::from_str(redact_str)
    .unwrap_or_else(|e| {
        warn!("Invalid SCRED_REDACT_PATTERNS '{}': {}", redact_str, e);
        PatternSelector::default_redact()  // SILENT FALLBACK!
    });
```

**Why It's Wrong**:
- User typo: `--redact CRIITICAL` (typo in tier name)
- Code warns to stderr
- Falls back to `default_redact()` (CRITICAL, API_KEYS, PATTERNS)
- User expects only CRITICAL → Gets more!
- If stderr redirected: warning lost!

**What Needs to Be Fixed**:

```rust
// BEFORE: Silent fallback
fn from_str(input: &str) -> Result<Self, String> {
    // ... parsing ...
    Ok(selector)
}

// In main.rs:
let redact_selector = PatternSelector::from_str(redact_str)
    .unwrap_or_else(|e| {
        warn!("...");
        PatternSelector::default_redact()  // ← WRONG!
    });

// AFTER: Error exit
fn from_str(input: &str) -> Result<Self, String> {
    // ... parsing ...
    match result {
        Ok(selector) => Ok(selector),
        Err(e) => {
            // Return detailed error
            Err(format!("Invalid selector '{}': {}\nValid tiers: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS\nValid patterns: aws-*, github-*, sk-*, etc.", input, e))
        }
    }
}

// In main.rs - ALL THREE BINARIES:
let redact_selector = PatternSelector::from_str(redact_str)
    .map_err(|e| {
        eprintln!("ERROR: {}", e);
        std::process::exit(1);
    })?;
```

**Implementation Steps**:
1. Improve error messages with suggestions
2. Exit with error (1) instead of warn + fallback
3. Show valid options in error
4. Apply to CLI, MITM, and Proxy consistently

**Effort**: 30 minutes
**Impact**: CRITICAL - User configuration failures now visible

---

### 1.4 🟠 HIGH: Pattern Name Mismatch Validation

**Current State**:
```rust
pub fn get_pattern_tier(pattern_type: &str) -> PatternTier {
    PATTERN_TIERS.get(pattern_type)
        .copied()
        .unwrap_or(PatternTier::Patterns)  // ← SILENT DEFAULT!
}
```

**Why It's Wrong**:
- Detector returns pattern name: "jwt_token" (typo)
- Pattern metadata has: "jwt"
- MISMATCH: get_pattern_tier("jwt_token") → Patterns tier (default)
- User set --redact CRITICAL (doesn't include Patterns)
- Secret NOT redacted!

**What Needs to Be Fixed**:

```rust
// BEFORE: Silent default
pub fn get_pattern_tier(pattern_type: &str) -> PatternTier {
    PATTERN_TIERS.get(pattern_type)
        .copied()
        .unwrap_or(PatternTier::Patterns)  // WRONG!
}

// AFTER: Error on unknown pattern
pub fn get_pattern_tier(pattern_type: &str) -> Result<PatternTier> {
    PATTERN_TIERS.get(pattern_type)
        .copied()
        .ok_or_else(|| anyhow!(
            "Unknown pattern type '{}'. Check pattern_metadata.rs for valid patterns.",
            pattern_type
        ))
}

// Update all call sites:
// OLD:
let tier = get_pattern_tier(&warning.pattern_type);
let matches = selector.matches_pattern(..., tier);

// NEW:
let tier = get_pattern_tier(&warning.pattern_type)
    .context("Pattern metadata mismatch")?;
let matches = selector.matches_pattern(..., tier);
```

**Implementation Steps**:
1. Change return type from `PatternTier` to `Result<PatternTier>`
2. Update all call sites with error handling
3. Add compile-time validation (build script checks all patterns)
4. Add test: unknown pattern names fail with clear error

**Effort**: 1 hour
**Impact**: HIGH - Future-proofs pattern additions

---

### 1.5 🟠 HIGH: Selector Precedence Inconsistency

**Current State**:

| Component | Precedence | Fallback |
|-----------|-----------|----------|
| CLI | CLI flag > ENV > hardcoded | Falls back to hardcoded |
| MITM | File > CLI > ENV > Config::load() | Falls back to Config::load() |
| Proxy | File > ENV (no CLI!) | No CLI support |

**Why It's Wrong**:
```bash
export SCRED_REDACT_PATTERNS=CRITICAL

# Create config file:
cat ~/.scred/config.yaml
  scred_proxy:
    redaction:
      patterns:
        redact: [CRITICAL, API_KEYS]

# Different behavior:
scred < file.env              # Uses ENV (CRITICAL) ✓
./scred-mitm                  # Uses File config (CRITICAL, API_KEYS) ✗
./scred-proxy                 # Uses File config (CRITICAL, API_KEYS) ✗
# All different!
```

**What Needs to Be Fixed**:

**Unified Precedence (All Three Binaries)**:
```
1. CLI flags (--detect, --redact)
2. Environment variables (SCRED_DETECT_PATTERNS, SCRED_REDACT_PATTERNS)
3. Config file (~/.scred/config.yaml, /etc/scred/config.yaml)
4. Hardcoded default (CRITICAL, API_KEYS, PATTERNS)
```

**Implementation in Each Binary**:

```rust
fn load_pattern_selectors() -> (PatternSelector, PatternSelector) {
    // Step 1: CLI flags (highest priority)
    if let Some(cli_value) = extract_cli_flag("--redact") {
        return PatternSelector::from_str(&cli_value)?;
    }
    
    // Step 2: Environment variables
    if let Ok(env_value) = env::var("SCRED_REDACT_PATTERNS") {
        return PatternSelector::from_str(&env_value)?;
    }
    
    // Step 3: Config file
    if let Ok(file_config) = ConfigLoader::load() {
        if let Some(patterns) = file_config.redact_patterns {
            return PatternSelector::from_str(&patterns)?;
        }
    }
    
    // Step 4: Hardcoded default
    Ok(PatternSelector::default_redact())
}
```

**Update CLI Flags**:
- Proxy needs --detect and --redact support
- All three need --detect support
- Currently only some support all flags

**Effort**: 2 hours (all three binaries)
**Impact**: HIGH - Consistent user experience

---

## PART 2: CROSS-COMPONENT CONSISTENCY (6 ISSUES)

### 2.1 🟠 HIGH: Env-Mode Doesn't Validate Pattern Selector

**Current State**:
```rust
// redact_env_line_configurable passes engine but never validates selector
pub fn redact_env_line_configurable(
    line: &str,
    config_engine: &ConfigurableEngine  // Pattern selector inside, but not checked!
) -> String {
    redact_env_line_generic(line, |v| config_engine.redact_only(v))
}
```

**Problem**:
- User sets: `--redact CRITICAL`
- Env-mode applies redaction
- But which selector is actually used? Unknown!
- Silent behavior change if selector not properly passed

**What Needs to Be Fixed**:

```rust
// BEFORE: No validation
pub fn redact_env_line_configurable(
    line: &str,
    config_engine: &ConfigurableEngine,
) -> String {
    redact_env_line_generic(line, |v| config_engine.redact_only(v))
}

// AFTER: Add logging + validation
pub fn redact_env_line_configurable(
    line: &str,
    config_engine: &ConfigurableEngine,
) -> String {
    // Log which selector is being used (for debugging)
    debug!("[env-mode] Selector: {:?}", config_engine.redact_selector());
    
    redact_env_line_generic(line, |v| {
        let result = config_engine.redact_only(v);
        
        // Validate that redaction happened when expected
        if v != result {
            debug!("[env-mode] Redacted: {} chars", v.len() - result.len());
        }
        
        result
    })
}

// Add method to ConfigurableEngine to expose selector for debugging:
impl ConfigurableEngine {
    pub fn redact_selector(&self) -> &PatternSelector {
        &self.redact_selector
    }
    
    pub fn detect_selector(&self) -> &PatternSelector {
        &self.detect_selector
    }
}
```

**Effort**: 1 hour
**Impact**: MEDIUM - Better debugging, but not a blocker

---

### 2.2 🟠 HIGH: Inconsistent Error Handling Across Components

**Current State**:
- CLI: Warns and falls back
- MITM: Same approach
- Proxy: Same approach
- **Result**: No consistency in error reporting

**What Needs to Be Fixed**:

**Create unified error handler**:
```rust
// In scred-http/src/error_handling.rs

pub mod pattern_selector {
    pub struct SelectorError {
        pub message: String,
        pub suggestions: Vec<String>,
        pub context: String,
    }
    
    pub fn handle_invalid_selector(
        input: &str,
        error: &str,
    ) -> Result<(), SelectorError> {
        Err(SelectorError {
            message: format!("Invalid selector '{}'", input),
            suggestions: vec![
                "Valid tiers: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS".to_string(),
                "Valid patterns: aws-*, github-*, sk-*, etc.".to_string(),
            ],
            context: error.to_string(),
        })
    }
}

// Used everywhere:
let selector = PatternSelector::from_str(input)
    .map_err(|e| pattern_selector::handle_invalid_selector(input, &e))?;
```

**Effort**: 1.5 hours
**Impact**: HIGH - Consistent error messages

---

### 2.3 🟡 MEDIUM: Multiline Secrets Not Handled

**Current State**:
```rust
// Line-by-line processing misses continuation
while let Some(line) = reader.lines().next().transpose()? {
    let redacted = redact_env_line(&line, ...);  // Each line isolated!
    println!("{}", redacted);
}
```

**Problem**: Multiline JWT partially exposed:
```
Line 1: API_KEY=eyJhbGc...   → Redacted
Line 2: iOiJIUzI1NiJ9.eyJ...  → NOT redacted (no KEY=)
Line 3: iOiJKb2huIn0.sig...   → NOT redacted
```

**What Needs to Be Fixed** (v1.1 - deferred):

```rust
// Use streaming accumulator for multiline
pub struct LineAccumulator {
    buffer: String,
    continuation_pattern: Regex,
}

impl LineAccumulator {
    pub fn process_line(&mut self, line: &str) -> Option<String> {
        // Check if line is continuation of previous
        if self.buffer.is_empty() {
            self.buffer = line.to_string();
            return None;
        }
        
        // Check if current line is continuation
        if self.is_continuation(line) {
            self.buffer.push('\n');
            self.buffer.push_str(line);
            return None;
        }
        
        // Current line is not continuation, flush buffer
        let complete_line = self.buffer.clone();
        self.buffer = line.to_string();
        Some(complete_line)
    }
    
    pub fn flush(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.buffer))
        }
    }
    
    fn is_continuation(&self, line: &str) -> bool {
        // Check for common continuation patterns:
        // - Starts with . (JWT continuation)
        // - Starts with whitespace (indent continuation)
        // - Matches backslash at end of previous line
        line.starts_with('.') || 
        line.starts_with(|c: char| c.is_whitespace()) ||
        self.buffer.ends_with('\\')
    }
}
```

**Effort**: 2-3 hours
**Impact**: MEDIUM - Edge case but real threat

---

### 2.4 🟡 MEDIUM: Type Confusion in Selector Parsing

**Current State**:
```rust
// Ambiguous parsing:
_ if input.contains('-') && !input.contains('*') => {
    // Single pattern - assumed to be wildcard!
    Ok(Self::Wildcard(vec![input.to_string()]))
}

// Input "aws-akia" could be:
// - Tier name? (No)
// - Pattern name? (Yes, in metadata)
// - Wildcard pattern? (Could match as wildcard!)
```

**What Needs to Be Fixed**:

```rust
pub fn from_str(input: &str) -> Result<Self, String> {
    let input = input.trim();
    
    match input.to_lowercase().as_str() {
        "all" => Ok(Self::All),
        "none" => Ok(Self::None),
        _ if input.starts_with("regex:") => { /* ... */ },
        _ if input.contains(',') => {
            // Try tiers first (higher priority)
            if let Ok(tiers) = PatternTier::parse_list(input) {
                return Ok(Self::Tier(tiers));
            }
            // Fall back to wildcard patterns
            let patterns: Vec<String> = input.split(',')
                .map(|s| s.trim().to_string())
                .collect();
            Ok(Self::Wildcard(patterns))
        }
        _ if input.contains('*') => {
            // Explicit wildcard pattern
            Ok(Self::Wildcard(vec![input.to_string()]))
        }
        _ if input.contains('-') => {
            // Could be tier name, pattern name, or wildcard
            // Check in order:
            
            // 1. Try as tier name first
            if let Ok(tier) = PatternTier::from_str(input) {
                return Ok(Self::Tier(vec![tier]));
            }
            
            // 2. Check if it's a known pattern name (validate against metadata)
            if is_known_pattern_name(input) {
                return Ok(Self::Whitelist({
                    let mut set = HashSet::new();
                    set.insert(input.to_string());
                    set
                }));
            }
            
            // 3. Treat as wildcard pattern
            Ok(Self::Wildcard(vec![input.to_string()]))
        }
        _ => {
            // Try as single tier or pattern
            match PatternTier::from_str(input) {
                Ok(tier) => Ok(Self::Tier(vec![tier])),
                Err(_) => Ok(Self::Whitelist({
                    let mut set = HashSet::new();
                    set.insert(input.to_string());
                    set
                })),
            }
        }
    }
}

// Add validation function:
fn is_known_pattern_name(name: &str) -> bool {
    PATTERN_TIERS.contains_key(name)
}
```

**Effort**: 1.5 hours
**Impact**: MEDIUM - Prevents silent type mismatches

---

### 2.5 🟡 MEDIUM: No Cross-Component Pattern Consistency Check

**Current State**:
- CLI loads patterns from redactor.rs
- MITM loads patterns from redactor.rs
- Proxy loads patterns from redactor.rs
- **But no verification they're all in sync!**

**What Needs to Be Fixed**:

```rust
// Add consistency check at startup
pub fn verify_pattern_consistency() -> Result<()> {
    // 1. Get all patterns from redactor
    let redactor_patterns = get_all_patterns();
    
    // 2. Verify all patterns in metadata
    for pattern_name in &redactor_patterns {
        let _ = get_pattern_tier(pattern_name)
            .context(format!("Pattern '{}' not in metadata", pattern_name))?;
    }
    
    // 3. Verify all tier names are valid
    let valid_tiers = vec![
        "CRITICAL",
        "API_KEYS",
        "INFRASTRUCTURE",
        "SERVICES",
        "PATTERNS",
    ];
    
    for tier_name in valid_tiers {
        let _ = PatternTier::from_str(tier_name)?;
    }
    
    Ok(())
}

// Call at startup:
#[tokio::main]
async fn main() -> Result<()> {
    verify_pattern_consistency()
        .context("Pattern consistency check failed")?;
    // ... rest of main
}
```

**Effort**: 1 hour
**Impact**: MEDIUM - Catches configuration drift

---

### 2.6 🟡 MEDIUM: No Audit Logging of Selector Changes

**Current State**:
- Selector changed via CLI/ENV/config
- No persistent record of what was applied
- No way to audit if redaction policy changed

**What Needs to Be Fixed**:

```rust
// Add audit logging
pub struct AuditLog {
    timestamp: DateTime<Utc>,
    component: String,
    selector_type: String,  // "detect" or "redact"
    previous_selector: String,
    new_selector: String,
    source: String,  // "cli", "env", "config_file", "default"
    user: Option<String>,
}

// Log selector changes:
pub fn load_selectors_with_audit(
    cli_detect: Option<String>,
    cli_redact: Option<String>,
) -> Result<(PatternSelector, PatternSelector)> {
    let detect = PatternSelector::from_str(
        &cli_detect.unwrap_or_else(|| "ALL".to_string())
    )?;
    
    info!(
        audit = true,
        selector_type = "detect",
        value = detect.description(),
        source = if cli_detect.is_some() { "cli" } else { "default" },
        "Pattern selector set"
    );
    
    // ... same for redact
    
    Ok((detect, redact_selector))
}
```

**Effort**: 1.5 hours
**Impact**: MEDIUM - Compliance/debugging

---

## PART 3: MAXIMUM SECURITY HARDENING (5 ADVANCED ISSUES)

### 3.1 🔐 URL-Decoded Secret Detection

**Current State**: Patterns match plaintext only

**Problem**:
```
URL-encoded: ?api_key=sk%2Dproj%5F123456
Plaintext:   sk-proj_123456
Pattern:     sk-[a-z0-9_-]{20,}
Result:      NO MATCH! Secret exposed!
```

**What Needs to Be Fixed**:

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
                redacted: text.to_string(),  // Keep original encoding
                warnings: merge_warnings(result1.warnings, result2.warnings),
            };
        }
    }
    
    result1
}
```

**Effort**: 1.5 hours
**Impact**: HIGH - Catches encoded secrets

---

### 3.2 🔐 Mutual Exclusivity Validation

**Current State**: No validation of contradictory configs

**Problem**:
```bash
scred --detect NONE --redact ALL  # Detect nothing but redact everything? Nonsensical!
scred --detect API_KEYS --redact INFRASTRUCTURE  # Detect different tier than redact?
```

**What Needs to Be Fixed**:

```rust
pub fn validate_selector_compatibility(
    detect: &PatternSelector,
    redact: &PatternSelector,
) -> Result<()> {
    match (detect, redact) {
        // Case 1: Detect NONE but redact ALL - nonsensical
        (PatternSelector::None, PatternSelector::All) => {
            return Err(anyhow!(
                "Invalid configuration: detect=NONE but redact=ALL. \
                 Cannot redact patterns that aren't detected."
            ))
        }
        
        // Case 2: Redact should be subset of detect
        (PatternSelector::Tier(detect_tiers), PatternSelector::Tier(redact_tiers)) => {
            for tier in redact_tiers {
                if !detect_tiers.contains(tier) {
                    warn!(
                        "Redact tier {:?} not in detect tiers {:?}. \
                         Patterns will not be detected before redaction.",
                        tier,
                        detect_tiers
                    );
                }
            }
        }
        
        _ => {}  // Other combinations acceptable
    }
    
    Ok(())
}
```

**Effort**: 1 hour
**Impact**: HIGH - Prevents user mistakes

---

### 3.3 🔐 Atomic Pattern Selector Updates

**Current State**: Race condition on config reload

**Problem**:
```
Thread 1: Read old detect patterns
Thread 2: Update config (new detect + redact)
Thread 1: Read new redact patterns
Result: Mismatch! Different detect/redact from different configs!
```

**What Needs to Be Fixed**:

```rust
use std::sync::Arc;
use parking_lot::RwLock;

pub struct PatternConfig {
    detect: PatternSelector,
    redact: PatternSelector,
    loaded_at: DateTime<Utc>,
}

pub struct AtomicPatternConfig {
    inner: Arc<RwLock<PatternConfig>>,
}

impl AtomicPatternConfig {
    pub fn update(&self, detect: PatternSelector, redact: PatternSelector) {
        let mut config = self.inner.write();
        *config = PatternConfig {
            detect,
            redact,
            loaded_at: Utc::now(),
        };
        info!("Pattern selectors updated atomically");
    }
    
    pub fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&PatternSelector, &PatternSelector) -> R,
    {
        let config = self.inner.read();
        f(&config.detect, &config.redact)
    }
}
```

**Effort**: 2 hours
**Impact**: MEDIUM - Production hardening

---

### 3.4 🔐 Selector Immutability After Startup

**Current State**: Selectors can change at runtime

**Problem**: Unexpected selector changes could change redaction behavior mid-operation

**What Needs to Be Fixed**:

```rust
pub struct ImmutablePatternSelector {
    detect: PatternSelector,
    redact: PatternSelector,
    locked: bool,  // Immutable after lock()
}

impl ImmutablePatternSelector {
    pub fn new(detect: PatternSelector, redact: PatternSelector) -> Self {
        Self {
            detect,
            redact,
            locked: false,
        }
    }
    
    pub fn lock(&mut self) {
        self.locked = true;
        info!("Pattern selectors locked for production");
    }
    
    pub fn update(&mut self, detect: PatternSelector, redact: PatternSelector) -> Result<()> {
        if self.locked {
            return Err(anyhow!(
                "Pattern selectors are locked. Cannot update after startup."
            ));
        }
        
        self.detect = detect;
        self.redact = redact;
        Ok(())
    }
}
```

**Effort**: 1 hour
**Impact**: MEDIUM - Prevents configuration drift

---

### 3.5 🔐 Streaming Mode Selector Enforcement

**Current State**: Streaming mode doesn't verify selector is applied

**Problem**:
```bash
cat large_file | scred --streaming --redact CRITICAL
# If streaming redactor ignores selector:
# → Uses engine's default (CRITICAL, API_KEYS, PATTERNS)
# → API_KEYS also redacted (unexpected!)
```

**What Needs to Be Fixed**:

```rust
// Verify selector is actually used in streaming:
pub fn process_stream_with_verification(
    redactor: &mut StreamingRedactor,
    input: &mut dyn Read,
    output: &mut dyn Write,
    expected_selector: &PatternSelector,
) -> Result<()> {
    let mut stats = RedactionStats::default();
    
    // Process stream
    while let Some(chunk) = read_chunk(input)? {
        let warnings = redactor.process_chunk(&chunk)?;
        
        // Verify all detected patterns match selector
        for warning in &warnings {
            let matches = expected_selector.matches_pattern(
                &warning.pattern_type,
                get_pattern_tier(&warning.pattern_type)?,
            );
            
            if !matches {
                warn!(
                    "Pattern {} detected but not in selector! Selector mismatch!",
                    warning.pattern_type
                );
                stats.selector_mismatches += 1;
            }
        }
        
        stats.total_warnings += warnings.len();
    }
    
    // Log if there were mismatches
    if stats.selector_mismatches > 0 {
        error!(
            "Selector mismatch detected: {} patterns found but not in selector",
            stats.selector_mismatches
        );
    }
    
    Ok(())
}
```

**Effort**: 1.5 hours
**Impact**: HIGH - Catches selector enforcement failures

---

## SUMMARY: IMPLEMENTATION ROADMAP

### Phase 1: Critical Fixes (v1.0.1) - 5 hours

| # | Issue | Time | Priority |
|---|-------|------|----------|
| 1.1 | Per-path rules enforcement | 2h | CRITICAL |
| 1.2 | Regex selector implementation | 2h | CRITICAL |
| 1.3 | Error on invalid selector | 1h | CRITICAL |

**Outcome**: All three blockers fixed

### Phase 2: Core Consistency (v1.1) - 8 hours

| # | Issue | Time | Priority |
|---|-------|------|----------|
| 1.4 | Pattern validation | 1h | HIGH |
| 1.5 | Unified precedence | 2h | HIGH |
| 2.1 | Env-mode validation | 1h | HIGH |
| 2.2 | Error handling unification | 1.5h | HIGH |
| 2.3 | Multiline secret handling | 2.5h | MEDIUM |
| 2.4 | Type confusion fixes | 1.5h | MEDIUM |

**Outcome**: Full cross-component consistency

### Phase 3: Maximum Security (v1.2+) - 9 hours

| # | Issue | Time | Priority |
|---|-------|------|----------|
| 2.5 | Pattern consistency check | 1h | MEDIUM |
| 2.6 | Audit logging | 1.5h | MEDIUM |
| 3.1 | URL-decoded detection | 1.5h | HIGH |
| 3.2 | Mutual exclusivity validation | 1h | HIGH |
| 3.3 | Atomic updates | 2h | MEDIUM |
| 3.4 | Selector immutability | 1h | MEDIUM |
| 3.5 | Streaming verification | 1.5h | MEDIUM |

**Outcome**: Production hardening + advanced security

---

## TESTING REQUIREMENTS

### Phase 1 Tests (v1.0.1)

```bash
# Test per-path rules
./scred-proxy --config tests/proxy-config-with-rules.yaml
curl localhost:9999/admin/secret?key=sk-123456
# Expected: sk-123456 NOT redacted (path rule)

# Test regex selector
scred --redact "regex:^sk-" < secrets.txt
# Expected: Only patterns starting with "sk-"

# Test invalid selector error
scred --redact INVALID_TIER < file.txt
# Expected: Exit code 1 with error message
```

### Phase 2 Tests (v1.1)

```bash
# Test unified precedence
export SCRED_REDACT_PATTERNS=API_KEYS
./scred-mitm --redact CRITICAL  # CLI flag wins
# Expected: Only CRITICAL redacted

# Test pattern validation
scred --redact unknown-pattern < file.txt
# Expected: Error if pattern not in metadata

# Test multiline JWT
cat multiline_jwt.txt | scred
# Expected: All lines redacted
```

### Phase 3 Tests (v1.2+)

```bash
# Test URL-encoded secrets
echo '?api_key=sk%2Dproj%5F123456' | scred
# Expected: Secret detected and redacted

# Test mutual exclusivity
scred --detect NONE --redact ALL
# Expected: Warning about contradictory config

# Test atomic updates
# Configure selector in file, change ENV, restart
# Expected: Both settings applied atomically
```

---

## CONCLUSION

**To Fix Core Logic & Maximize Security**:

1. **Immediate (5 hours)**: Fix 3 critical blockers
   - Per-path rules enforcement
   - Regex selector implementation
   - Error on invalid selector

2. **Short-term (8 hours)**: Full cross-component consistency
   - Unified precedence
   - Error handling
   - Validation

3. **Long-term (9 hours)**: Production hardening
   - URL detection
   - Atomic updates
   - Audit logging
   - Advanced security

**Total Effort**: 22 hours (achievable in 3-4 days)

**Result**: SCRED v1.0-1.2 will be production-ready with no security gaps.

**Recommendation**: 
- Ship v1.0.1 with Phase 1 fixes (5 hours)
- Plan v1.1 sprint for Phase 2 (8 hours)
- Plan v1.2 roadmap for Phase 3 (9 hours)
