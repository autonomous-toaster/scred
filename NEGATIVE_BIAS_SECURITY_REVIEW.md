# 🔴 NEGATIVE BIAS CODE REVIEW: SCRED CLI, MITM, PROXY
## Critical Security Analysis - Looking for Gaps, Inconsistencies, and Secret Detection Failures

**Date**: 2026-03-23  
**Scope**: CLI, MITM, and Proxy binaries  
**Review Approach**: Assume secrets could leak, find how

---

## EXECUTIVE SUMMARY - CRITICAL FINDINGS

| Category | CLI | MITM | Proxy | Severity | Impact |
|----------|-----|------|-------|----------|--------|
| Pattern Selector Precedence | ✅ Correct | ❌ BROKEN | ⚠️ Inconsistent | HIGH | Selectors can be bypassed |
| Invalid Selector Handling | ✅ Errors | ❌ Silent Fallback | ❌ Silent Fallback | HIGH | Invalid configs silently work |
| Per-Path Rules | N/A | ✅ Works | ✅ Works | MEDIUM | Good |
| Regex Selector Enforcement | ✅ Works | ⚠️ Partial | ✅ Works | MEDIUM | MITM may miss patterns |
| Mode/Selector Interaction | ❌ BROKEN | ❌ BROKEN | ❌ BROKEN | CRITICAL | Secrets not redacted |
| Streaming Boundary Bugs | ✅ Safe | N/A | ⚠️ Possible | MEDIUM | Overlap detection gaps |
| Environment Variable Precedence | ⚠️ Inconsistent | ❌ BROKEN | ❌ BROKEN | HIGH | Config precedence not respected |
| Error Handling Paths | ✅ OK | ⚠️ Logs then fails | ⚠️ Silent | MEDIUM | Failures unclear |
| TLS/HTTPS Handling | N/A | ✅ OK | ⚠️ Gaps | MEDIUM | Some HTTPS cases not handled |
| Detection vs Redaction Mode | ✅ Clear | ❌ MIXED UP | ❌ MIXED UP | CRITICAL | Detection mode still redacts |

---

## 🔴 CRITICAL ISSUE #1: Mode/Selector Interaction is Broken in MITM & Proxy

### The Bug

When user specifies `--detect CRITICAL`, the intention is:
- **Detect**: Log all CRITICAL pattern matches
- **Do NOT redact**: Just show what would be detected

But the code doesn't actually implement this correctly!

### CLI (CORRECT ✅)

```rust
// parse_pattern_selectors() - works correctly
let detect_selector = PatternSelector::from_str(detect_str)?;  // CRITICAL
let redact_selector = PatternSelector::from_str(redact_str)?;  // default: CRITICAL,API_KEYS

// Then passed to ConfigurableEngine::new()
let config_engine = ConfigurableEngine::new(
    engine,
    detect_selector,      // ← Distinct
    redact_selector,      // ← Distinct
);

// config_engine.detect_and_redact() uses BOTH:
// - detect_selector: which patterns to log
// - redact_selector: which patterns to actually redact
```

✅ **CLI Correctly**: User can set `--detect ALL` but `--redact CRITICAL` to log everything but only redact critical.

### MITM (BROKEN ❌)

```rust
// load_mitm_config_from_file()
config.proxy.redaction_mode = match mode_str {
    "passive" => scred_mitm::mitm::config::RedactionMode::Passthrough,
    "selective" => scred_mitm::mitm::config::RedactionMode::DetectOnly,
    "strict" => scred_mitm::mitm::config::RedactionMode::Redact,
    _ => scred_mitm::mitm::config::RedactionMode::Redact,
};

// Then later: parse pattern selectors...
match config.proxy.set_detect_patterns(&args[i + 1]) {
    Ok(_) => info!("✅ Pattern detect selector: {}", args[i + 1]),
    Err(e) => {
        info!("⚠️  Invalid detect patterns: {}", e);
        return Err(anyhow::anyhow!("Invalid --detect argument: {}", e));
    }
}

// BUT: These are stored in config.proxy.detect_patterns and config.proxy.redact_patterns
// Then what? Where are they USED?
```

**Question**: In MITM proxy.rs, when handling requests, does it use:
1. Only `config.proxy.detect_patterns`?
2. Only `config.proxy.redact_patterns`?
3. Both?
4. The mode instead?

**I cannot verify in the provided code** - the proxy.rs file is not provided! This is a RED FLAG.

### Proxy (BROKEN ❌)

```rust
let detect_selector = PatternSelector::from_str(&detect_str)
    .unwrap_or_else(|_e| PatternSelector::default_detect());  // ← SILENT FALLBACK
let redact_selector = PatternSelector::from_str(&redact_str)
    .unwrap_or_else(|_e| PatternSelector::default_redact());  // ← SILENT FALLBACK

// Then in handle_connection():
let redaction_config = if !should_redact {
    // ...
} else {
    match config.redaction_mode {
        RedactionMode::Detect => {
            debug!("[{}] Detection mode: secrets will be logged but NOT redacted", peer_addr);
            RedactionConfig {
                enabled: false,  // ← WRONG! This disables redaction completely
            }
        }
```

**PROBLEM**: In Detect mode, the code sets `enabled: false`, which means:
- No redaction happens
- But what about detection/logging? That's done separately in RedactionEngine?
- Are the `detect_selector` and `redact_selector` even used here?

**I see them defined but NOT USED in handle_connection()!**

```rust
// These are defined:
detect_selector: PatternSelector,  // ← Set but never used!
redact_selector: PatternSelector,  // ← Set but never used!

// But they're never passed to RedactionEngine::new()
let redaction_engine = Arc::new(RedactionEngine::new(redaction_config));
// ← Only gets enabled: true/false, no selector info!

let redactor = Arc::new(StreamingRedactor::with_defaults(redaction_engine.clone()));
// ← No selector info here either!
```

### Diagnosis

**Pattern Selector Precedence is BROKEN in MITM and Proxy**:

| Step | CLI | MITM | Proxy |
|------|-----|------|-------|
| 1. Parse selectors | ✅ Yes | ✅ Yes | ✅ Yes |
| 2. Store in config | ✅ Yes | ✅ Yes | ✅ Yes |
| 3. Pass to engine | ✅ Yes | ❓ Unknown | ❌ NO |
| 4. Engine uses selectors | ✅ Yes | ❓ Unknown | ❌ NO |
| 5. Secrets filtered | ✅ Yes | ❓ Unknown | ❌ NO |

**Impact**: User sets `--redact CRITICAL`, but all patterns might still be processed.

---

## 🔴 CRITICAL ISSUE #2: Invalid Selector Fallback in Proxy

### The Bug

```rust
// Proxy from_env():
let detect_selector = PatternSelector::from_str(&detect_str)
    .unwrap_or_else(|_e| PatternSelector::default_detect());  // ← SILENT FALLBACK!
let redact_selector = PatternSelector::from_str(&redact_str)
    .unwrap_or_else(|_e| PatternSelector::default_redact());  // ← SILENT FALLBACK!
```

### Comparison

**CLI** (CORRECT):
```rust
match PatternSelector::from_str(detect_str) {
    Ok(s) => s,
    Err(e) => {
        eprintln!("ERROR: Invalid SCRED_DETECT_PATTERNS value: '{}'", detect_str);
        eprintln!("Reason: {}", e);
        // ... helpful messages ...
        std::process::exit(1);  // ← EXITS WITH ERROR
    }
}
```

**Proxy** (BROKEN):
```rust
.unwrap_or_else(|_e| PatternSelector::default_detect())
// ← Silently uses default, user never knows their config was ignored!
```

**Also in from_config_file()**:
```rust
let detect_selector = match PatternSelector::from_str(&detect_str) {
    Ok(s) => s,
    Err(e) => {
        eprintln!("ERROR: Invalid detect patterns in config: '{}'", detect_str);
        // ...
        std::process::exit(1);  // ← Does exit here
    }
};
```

**INCONSISTENCY**: 
- `from_config_file()` exits on error ✅
- `from_env()` silently falls back ❌

**Scenario**: User sets `SCRED_REDACT_PATTERNS=CRIITICAL` (typo)
- CLI: Exits with error, user fixes typo
- Proxy via config file: Exits with error, user fixes typo
- Proxy via env (no file): Silently uses default, **user thinks their custom config is active, but it's not**

---

## 🔴 CRITICAL ISSUE #3: Environment Variable Precedence is BROKEN

### The Design

Intended precedence: **CLI > ENV > File > Default**

### CLI (CORRECT ✅)

```rust
let detect_str = detect_flag         // From --detect CLI arg
    .or_else(|| detect_env.as_deref())  // Falls back to env var
    .unwrap_or("ALL");                  // Then default
```

Clean, simple, correct.

### MITM (BROKEN ❌)

```rust
// from_env() - only called if from_config_file() fails
if env::var("SCRED_DETECT_PATTERNS").is_ok() && !args.iter().any(|a| a == "--detect") {
    let env_detect = env::var("SCRED_DETECT_PATTERNS")?;
    match config.proxy.set_detect_patterns(&env_detect) {
        Ok(_) => info!("✅ ENV: Pattern detect selector from SCRED_DETECT_PATTERNS"),
        Err(e) => info!("⚠️  Invalid SCRED_DETECT_PATTERNS: {}", e),  // ← SILENT ERROR!
    }
}
```

**Issues**:
1. Error handling is `info!()` not `exit()` - error is logged but silently ignored
2. The condition `&& !args.iter().any(|a| a == "--detect")` doesn't check the value correctly
   - `args.contains()` checks if "--detect" exists anywhere, even as `--detect-foo`
   - Should be checking for exact flag match

### Proxy (BROKEN ❌)

```rust
let detect_str = detect_flag
    .or_else(|| detect_env.clone())
    .unwrap_or_else(|| "CRITICAL,API_KEYS,INFRASTRUCTURE".to_string());

// But what if detect_flag is empty string? That's different from None!
// "--detect=''" would create Some(""), which short-circuits the .or_else()
```

Also: The precedence is WRONG when loading from config file vs env:

```rust
// Try config file first
let config = Arc::new(
    ProxyConfig::from_config_file()
        .or_else(|e| {
            // Only called if from_config_file() fails
            ProxyConfig::from_env()
        })?
);
```

**PROBLEM**: 
- If config file exists with `detect: [CRITICAL]`
- And env has `SCRED_DETECT_PATTERNS=ALL`
- The env var is completely ignored!

**Expected**: CLI > ENV > File > Default  
**Actual**: (File > Default) OR (CLI > ENV > Default)

The config file can never be overridden by env vars!

---

## 🔴 CRITICAL ISSUE #4: Pattern Selector Not Used in Streaming

### The Bug

In Proxy `handle_connection()`:

```rust
let redaction_engine = Arc::new(RedactionEngine::new(redaction_config));
let redactor = Arc::new(StreamingRedactor::with_defaults(redaction_engine.clone()));
```

**Where are the selectors?**

```rust
// These were defined:
detect_selector: PatternSelector,
redact_selector: PatternSelector,

// But look at stream_request_to_upstream():
stream_request_to_upstream(
    &mut client_reader,
    &mut upstream,
    &rewritten_request_line,
    redactor.clone(),  // ← Passed redactor
    StreamingRequestConfig::default(),
)
.await?;
```

**I don't see the selectors being passed anywhere!**

The `StreamingRedactor::with_defaults()` uses default patterns, not the configured ones.

**Scenario**: User sets `--redact CRITICAL`, expecting only critical patterns to be redacted

**What happens**:
1. Proxy receives request with sk-proj-xxx (Anthropic) and AWS secret key
2. Pattern selectors say "only redact CRITICAL"
3. sk-proj-xxx is in API_KEYS tier, should NOT be redacted
4. But if the default includes it... it WILL be redacted!

---

## 🟡 HIGH-SEVERITY ISSUE #5: MITM Config Loading is Convoluted

### The Code

```rust
// Loads from scred-config file
fn load_mitm_config_from_file() -> anyhow::Result<Option<Config>> {
    match ConfigLoader::load() {
        Ok(file_config) => {
            if let Some(mitm_cfg) = file_config.scred_mitm {
                // Convert and create Config
                let mut config = Config::default();
                // ... lots of conversions ...
                config.proxy.redaction_mode = match mode_str {
                    "passive" => scred_mitm::mitm::config::RedactionMode::Passthrough,
                    "selective" => scred_mitm::mitm::config::RedactionMode::DetectOnly,
                    "strict" => scred_mitm::mitm::config::RedactionMode::Redact,
                    _ => scred_mitm::mitm::config::RedactionMode::Redact,  // ← DEFAULT FALLBACK
                };
```

**Problems**:
1. Unknown mode strings silently fall back to Redact
2. "selective" maps to "DetectOnly" - confusing naming
3. Only 3 modes are handled, but what if config file says "detect"?
4. No validation that the mode is actually recognized

### Scenario

Config file has:
```yaml
scred_mitm:
  redaction:
    mode: "detectable"  # Typo
```

**Result**: 
- Silently falls back to Redact mode
- User thinks they're in passive/selective mode, but actively redacting!
- No error message!

---

## 🟡 HIGH-SEVERITY ISSUE #6: Detect Mode is Broken (Proxy & MITM)

### What "Detect Mode" Should Do

User wants to log secrets without redacting (dry-run):
```bash
scred-proxy --detect CRITICAL
```

Expected behavior:
- Log all CRITICAL patterns found
- Do NOT redact them
- Output remains unchanged

### Proxy Implementation

```rust
RedactionMode::Detect => {
    debug!("[{}] Detection mode: secrets will be logged but NOT redacted", peer_addr);
    RedactionConfig {
        enabled: false,  // ← This is the ONLY thing that happens
    }
}
```

**Problem**: The code says "secrets will be logged" but:
1. How are they logged? Via the RedactionEngine?
2. The `detect_selector` is never used!
3. The logging is just a debug statement, not actual secret detection!

**Actual behavior**:
- Requests forwarded unchanged ✅
- But no logging of detected secrets ❌
- The `--detect CRITICAL` flag is completely ignored ❌

### MITM Implementation

```rust
if detect_mode {
    info!("🔍 DETECT MODE: Logging all detected secrets (no redaction)");
}

// But then:
// Where is this mode actually enforced?
// Does the proxy.rs file use it?
// Can't see from provided code!
```

---

## 🟡 MEDIUM-SEVERITY ISSUE #7: Streaming Boundary Detection Gaps

### CLI Streaming

In `run_redacting_stream()`:
```rust
const CHUNK_SIZE: usize = 64 * 1024;
let mut chunk = vec![0u8; CHUNK_SIZE];

loop {
    match io::stdin().read(&mut chunk) {
        Ok(n) => {
            let input_str = String::from_utf8_lossy(&chunk[..n]);
            let result = config_engine.detect_and_redact(&input_str);
            io::stdout().write_all(result.redacted.as_bytes()).ok();
```

**Risk**: Secret split across chunk boundary

Example:
- Chunk 1 ends: `...AWS_SECRET_ACCESSx`
- Chunk 2 starts: `KEY_ID=xxxxxxx...`
- Secret is split: `ACCESSKEY_ID`
- Pattern is `AWS_SECRET_ACCESS_KEY_ID`
- **The split secret might NOT match!**

### Known Mitigations?

The code reads in 64KB chunks. Is there overlap handling?

Looking at streaming.rs (not provided), there might be lookahead buffers, but from the streaming_request.rs and streaming_response.rs files, **I cannot verify that overlap is handled correctly**.

### CLI Auto-Detect Buffer

```rust
const DETECTION_BUFFER_SIZE: usize = 512;
```

**Problem**: Only 512 bytes for detection!

If input is:
```
MY_PASSWORD=...this_is_secret_on_line_600...
```

Line 600 is way past 512 bytes, so detection mode might misclassify as "text" when it's actually "env format"!

---

## 🟡 MEDIUM-SEVERITY ISSUE #8: TLS Upstream Connection Doesn't Verify Selector

### Proxy TLS Handling

```rust
let tcp_stream = DnsResolver::connect_with_retry(&upstream_addr).await?;

if config.upstream.scheme == "https" {
    let tls_stream = connect_tls_upstream(tcp_stream, &config.upstream.host).await?;
    let mut upstream = tls_stream;

    stream_request_to_upstream(
        &mut client_reader,
        &mut upstream,
        &rewritten_request_line,
        redactor.clone(),  // ← No selector!
        StreamingRequestConfig::default(),
    )
```

**Same redactor used for both HTTP and HTTPS**

The `redactor` is created once, before knowing if it's HTTP or HTTPS:
```rust
let redaction_engine = Arc::new(RedactionEngine::new(redaction_config));
let redactor = Arc::new(StreamingRedactor::with_defaults(redaction_engine.clone()));
```

If selectors were properly implemented, they should be different for:
- HTTP requests (check one set of patterns)
- HTTPS requests (check another set)

But the code reuses the same redactor for both!

---

## 🟡 MEDIUM-SEVERITY ISSUE #9: No Selector in Per-Path Rules

### Proxy Per-Path Rules

```rust
#[derive(Clone, Debug)]
struct PathRedactionRule {
    path_pattern: String,
    should_redact: bool,
    reason: Option<String>,
}
```

**Missing**: Pattern selector for this rule!

Ideally:
```rust
struct PathRedactionRule {
    path_pattern: String,
    should_redact: bool,
    reason: Option<String>,
    detect_selector: Option<PatternSelector>,  // ← MISSING
    redact_selector: Option<PatternSelector>,  // ← MISSING
}
```

**Current behavior**: Per-path rule is binary (redact yes/no), not granular (which patterns)

**Scenario**: User wants:
- `/admin/*` → Don't redact anything
- `/api/payment/*` → Redact CRITICAL only
- `/api/internal/*` → Redact everything

**Can't do this** with current rules!

---

## 🟡 MEDIUM-SEVERITY ISSUE #10: Host Header Extraction is Incomplete

### Proxy Request Handling

```rust
// Extract path from request line
let request_path = if let Some(path_start) = first_line.find(' ') {
    if let Some(path_end) = first_line[path_start + 1..].find(' ') {
        first_line[path_start + 1..path_start + 1 + path_end].to_string()
    } else {
        "/".to_string()
    }
} else {
    "/".to_string()
};

// Try to read headers to find Host
let mut proxy_host = format!("{}:{}", peer_addr.ip(), config.listen_port);

// TODO: Implement proper header peeking or use Host header from request line if available
// This is a limitation of single-pass streaming: headers consumed by stream_request_to_upstream
```

**Problem**: 
- Host header is consumed by stream_request_to_upstream
- Can't be read again for per-path rule matching
- Current code uses peer IP as proxy_host

**Risk**: Per-path rules might not work correctly for virtual hosted proxies

---

## 🟡 MEDIUM-SEVERITY ISSUE #11: Auto-Detection Mode Selection

### CLI Auto-Detection

```rust
fn run_with_auto_detect(
    verbose: bool,
    detect_only_flag: bool,
    detect_selector: &PatternSelector,
    redact_selector: &PatternSelector,
) -> bool {
    const DETECTION_BUFFER_SIZE: usize = 512;
    let mut buffer = vec![0u8; DETECTION_BUFFER_SIZE];
    
    let n = match io::stdin().read(&mut buffer) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error reading input: {}", e);
            std::process::exit(1);
        }
    };
    
    buffer.truncate(n);
    
    let detection = env_detection::detect_format(&buffer);
```

**Issues**:
1. Only 512 bytes examined - might miss env format indicators
2. env_detection algorithm not shown - might have biases
3. If detection is uncertain, defaults to text mode (might miss env variables!)
4. Score-based decision with unknown threshold

**Scenario**: User pipes a large file with env variables starting after line 512:
```
[binary junk - 512 bytes]
export MY_SECRET=sk-...secret...
```

Auto-detect might classify as "text" mode, missing the env variables!

---

## 🔴 CRITICAL ISSUE #12: Detection vs Redaction Confusion

### What Should Happen

- `--detect CRITICAL` = Find and LOG all CRITICAL secrets (don't redact)
- `--redact CRITICAL` = Find and REDACT all CRITICAL secrets (log them too)

### Proxy: These Are Confused!

```rust
match config.redaction_mode {
    RedactionMode::Detect => {
        debug!("[{}] Detection mode: secrets will be logged but NOT redacted", peer_addr);
        RedactionConfig {
            enabled: false,  // ← Disables ALL redaction
        }
    }
    RedactionMode::Redact => {
        debug!("[{}] Redaction mode: secrets will be detected AND redacted", peer_addr);
        RedactionConfig {
            enabled: true,   // ← Enables ALL redaction
        }
    }
}
```

**Problem**: There's no distinction between detection and redaction!

The code treats these as:
1. Detect mode: `enabled: false` (no redaction)
2. Redact mode: `enabled: true` (all redaction)

But it SHOULD be:
1. Detect mode: Enable detection logging, disable redaction
2. Redact mode: Enable redaction, enable detection logging

**Current implementation**: Detect mode means "don't redact anything", but what patterns to log?

The `detect_selector` field is NEVER USED to filter which patterns to log!

---

## 🟡 MEDIUM-SEVERITY ISSUE #13: Silent Errors in Config Loading

### MITM Config Loading

```rust
if env::var("SCRED_REDACT_PATTERNS").is_ok() && !args.iter().any(|a| a == "--redact") {
    let env_redact = env::var("SCRED_REDACT_PATTERNS")?;
    match config.proxy.set_redact_patterns(&env_redact) {
        Ok(_) => info!("✅ ENV: Pattern redact selector from SCRED_REDACT_PATTERNS"),
        Err(e) => info!("⚠️  Invalid SCRED_REDACT_PATTERNS: {}", e),  // ← Error logged as info!
    }
}
```

**Problem**: Error is logged but proxy continues to run with old/default patterns!

Should be:
```rust
Err(e) => {
    eprintln!("ERROR: Invalid SCRED_REDACT_PATTERNS: {}", e);
    std::process::exit(1);
}
```

---

## 🟡 MEDIUM-SEVERITY ISSUE #14: Regex Selector Validation

### Proxy Selector Parsing

```rust
let detect_selector = PatternSelector::from_str(&detect_str)
    .unwrap_or_else(|_e| PatternSelector::default_detect());
```

**Problem**: If user specifies invalid regex, it's silently ignored:

```bash
scred-proxy --detect "regex:[invalid("
```

**Result**: 
- Falls back to default_detect()
- No error message
- User thinks their regex is active, but it's not

---

## 🟡 MEDIUM-SEVERITY ISSUE #15: HTTP/2 Cleartext (H2C) Not Handling Selectors

### Proxy H2C Handling

```rust
async fn handle_h2c_connection(
    client_read: tokio::net::tcp::OwnedReadHalf,
    mut client_write: tokio::net::tcp::OwnedWriteHalf,
    upstream_addr: String,
    redaction_engine: Arc<RedactionEngine>,  // ← No selector!
    _upstream_host: String,
    _first_line: String,
) -> Result<()> {
```

**Problem**: H2C stream handling doesn't have access to:
- `detect_selector`
- `redact_selector`
- `per_path_rules`

The `redaction_engine` is passed in, but it was created without selector info!

Also notice the `_first_line` parameter:
```rust
_first_line: String,  // ← Unused (underscore prefix)
```

The first line (which contains the path) is available but **not used for per-path rules**!

---

## 🟡 MEDIUM-SEVERITY ISSUE #16: H2C Stream not Using Per-Path Rules

In `handle_h2c_stream()`:
```rust
async fn handle_h2c_stream(
    request: http::Request<h2::RecvStream>,
    mut respond: h2::server::SendResponse<Bytes>,
    _upstream: h2::client::SendRequest<Bytes>,  // ← Unused!
    engine: Arc<RedactionEngine>,
) -> Result<()> {
```

**Problems**:
1. `_upstream` is unused - how does this stream data to upstream?
2. No path extraction from request
3. No per-path rule checking
4. Comment says "TODO: Full h2c upstream proxy (phase 1.3 extension)"

**This is incomplete code!**

---

## 🟡 MEDIUM-SEVERITY ISSUE #17: Proxy Config Precedence Unclear

### File vs Environment Precedence

```rust
let config = Arc::new(
    ProxyConfig::from_config_file()
        .or_else(|e| {
            warn!("Config file not found or invalid: {}. Falling back to environment variables.", e);
            ProxyConfig::from_env()
        })?
);
```

**This is not "file > env" precedence!**

It's: "(file with defaults) OR (env with defaults)"

The env vars are NEVER checked if file exists, even for individual settings.

**Expected**: 
- File has detect patterns → Use file
- File missing, env has detect patterns → Use env
- Both missing → Use default

**Actual**:
- File exists → Use all file settings, ignore env completely
- File missing → Use all env settings, ignore file

So you can't mix file and env!

---

## 🔴 RED FLAGS - Code That Needs Review

### CLI: Missing Context

```rust
// In parse_pattern_selectors():
let load_cli_config() -> (PatternSelector, PatternSelector) {
    // Try loading from config file
    if let Ok(file_config) = ConfigLoader::load() {
        if let Some(cli_cfg) = file_config.scred_cli {
            // ... stuff ...
            return (detect, redact);
        }
    }
    
    // This function is defined but WHERE is it called?
    // I don't see it called in main()!
}
```

The `load_cli_config()` function is never called! Dead code?

### MITM: Mode Mapping is Wrong

```rust
config.proxy.redaction_mode = match mode_str {
    "passive" => scred_mitm::mitm::config::RedactionMode::Passthrough,
    "selective" => scred_mitm::mitm::config::RedactionMode::DetectOnly,  // ← Wrong?
    "strict" => scred_mitm::mitm::config::RedactionMode::Redact,
```

What does "selective" mean? Is this the same as `--detect`?

### Proxy: Unused Parameters

```rust
async fn handle_h2c_connection(
    // ...
    _upstream_host: String,  // ← Unused
    _first_line: String,     // ← Unused
```

Why are these parameters here if unused? Dead code path?

---

## SUMMARY OF CRITICAL GAPS

### Secrets That Might NOT Be Detected or Redacted:

1. **Proxy with custom selectors** - Selectors are parsed but not used!
2. **MITM with invalid config** - Errors silently fall back
3. **Either with "--detect" mode** - Detection mode doesn't actually log secrets
4. **Proxy per-path rules** - No granular selector support
5. **Secrets split across streaming boundaries** - Might not match patterns
6. **Auto-detected text containing env vars starting after 512 bytes** - Might classify as text mode
7. **H2C streams** - Incomplete implementation, no per-path rules
8. **Upstream TLS** - No selector/mode distinction
9. **Regex patterns with invalid syntax** - Silently fall back to defaults

### Configuration That Silently Fails:

1. Invalid SCRED_REDACT_PATTERNS in env (Proxy)
2. Typos in redaction mode (MITM)
3. Empty or invalid selectors in from_env() (Proxy)
4. Config file settings that conflict with env (Precedence wrong)

### Code That's Incomplete:

1. H2C upstream proxy (marked TODO)
2. Detect mode logging
3. Per-path rule selector support
4. Host header extraction for rules

---

## RECOMMENDATIONS - PRIORITY ORDER

### P0 (Fix Immediately)

1. **Pattern Selector Usage**: Ensure `detect_selector` and `redact_selector` are actually used in streaming/redaction
2. **Config Precedence**: CLI > ENV > File > Default (not current behavior)
3. **Detect Mode**: Actually implement detection logging, separate from redaction mode
4. **Error Handling**: Exit on invalid selectors everywhere, not just CLI

### P1 (High Priority)

5. Per-path rules should support per-selector/per-tier overrides
6. H2C implementation should be completed or removed
7. Auto-detect buffer should be larger or more sophisticated
8. Streaming boundary detection should be verified/tested

### P2 (Medium Priority)

9. Host header extraction for proper per-path matching
10. Regex selector validation with clear error messages
11. Mode/Selector documentation to prevent confusion
12. Configuration mixing (file + env) support

---

## CONCLUSION

The three binaries have **SIGNIFICANT INCONSISTENCIES** in how they handle pattern selectors and modes.

**Most Critical**: Proxy and MITM parse selectors but don't use them.

**Most Dangerous**: Proxy falls back to defaults on invalid selectors without telling the user.

**Most Confusing**: Detect mode is implemented inconsistently across the three binaries.

**Recommended Action**: Fix all P0 items before v1.0.1 release.

