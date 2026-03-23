# ⚡ SCRED v1.0 → v1.2 ACTION PLAN
## Core Logic Fixes + Maximum Security (22 Hours Total)

---

## QUICK START: What to Do Right Now

### Option A: Ship v1.0.1 in 5 Hours (Recommended)
```bash
# Fix these 3 blockers immediately:
1. Proxy per-path rules enforcement (2h)
2. Regex selector implementation (2h)  
3. Error on invalid selector (1h)

# Then release v1.0.1 with "CRITICAL FIXES" note
# Continue with v1.1 planning in parallel
```

### Option B: Delay v1.0, Ship Complete v1.1 in 13 Hours
```bash
# Fix everything at once:
# Phase 1 (5h) + Phase 2 (8h) = 13 hours total

# Better for greenfield deployment
# Avoid "patch treadmill" of multiple v1.0.x releases
```

### Option C: Maximum Security in 22 Hours (Best Practice)
```bash
# Plan 3-sprint roadmap:
# Sprint 1 (v1.0.1): Phase 1 critical fixes (5h)
# Sprint 2 (v1.1): Phase 2 consistency (8h)
# Sprint 3 (v1.2): Phase 3 hardening (9h)

# Ship incrementally with security improvements each release
```

**Recommendation**: Option A (5h quick fix) + planning for Options B/C

---

## PHASE 1: CRITICAL FIXES (v1.0.1) - 5 HOURS

### Task 1.1: Proxy Per-Path Rules Enforcement (2 hours)

**What**: Implement path-matching logic before redaction  
**Where**: `crates/scred-proxy/src/main.rs` → `handle_request()`  
**Why**: Rules currently parsed but never checked (dead code)

**Implementation Checklist**:
- [ ] Add glob matching: `path_matches(path, pattern)` function
- [ ] Extract request path before redaction
- [ ] Check `should_redact_for_path()` before calling redactor
- [ ] Add logging: "Skipping redaction for {path} (rule: {reason})"
- [ ] Add test: curl localhost:9999/admin/* expects no redaction

**Code Template**:
```rust
fn should_redact_for_path(path: &str, rules: &[PathRedactionRule]) -> bool {
    for rule in rules {
        if path_matches(&path, &rule.path_pattern) {
            return rule.should_redact;
        }
    }
    true  // Default: redact if no rule
}

// In request handler:
let should_redact = should_redact_for_path(
    &request_path,
    &config.per_path_rules
);

if should_redact {
    body = redactor.redact(&body);
}
```

**Effort**: 2 hours  
**Test**: `cargo test proxy_per_path_rules`

---

### Task 1.2: Regex Selector Implementation (2 hours)

**What**: Implement actual regex matching for `--redact "regex:^sk-"`  
**Where**: `crates/scred-http/src/pattern_selector.rs` → `matches_pattern()`  
**Why**: Currently uses string `contains()` instead of regex (completely broken)

**Implementation Checklist**:
- [ ] Add regex crate to Cargo.toml (already exists)
- [ ] Compile regex patterns on selector creation
- [ ] Add error handling for invalid regex
- [ ] Add caching: HashMap<String, Regex>
- [ ] Add validation tests: anchors, groups, lookahead

**Code Template**:
```rust
use regex::Regex;

Self::Regex(patterns) => {
    for pattern_str in patterns {
        match Regex::new(pattern_str) {
            Ok(regex) => {
                if regex.is_match(pattern_name) {
                    return true;
                }
            }
            Err(e) => {
                warn!("Invalid regex '{}': {}", pattern_str, e);
            }
        }
    }
    false
}
```

**Effort**: 2 hours  
**Test**: `cargo test regex_selector_matching`

---

### Task 1.3: Error on Invalid Selector (1 hour)

**What**: Exit with error instead of silently falling back  
**Where**: `crates/scred-cli/src/main.rs:50-55`, `crates/scred-mitm/src/main.rs:80-90`, same in proxy  
**Why**: User typos silently change behavior (e.g., "CRIITICAL" → falls back to defaults)

**Implementation Checklist**:
- [ ] Change unwrap_or_else() to error handling in ALL THREE binaries
- [ ] Improve error messages with suggestions
- [ ] Show valid tier names in error
- [ ] Exit with code 1 on error
- [ ] Add test: invalid selector exits with error

**Code Template**:
```rust
let redact_selector = PatternSelector::from_str(redact_str)
    .map_err(|e| {
        eprintln!("ERROR: Invalid selector '{}': {}", redact_str, e);
        eprintln!("Valid tiers: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS");
        std::process::exit(1);
    })?;
```

**Effort**: 1 hour  
**Test**: `cargo test invalid_selector_error`

---

### Phase 1 Validation

```bash
# After Phase 1 fixes:
cargo build --release --all 2>&1 | grep -E "error|warning"
cargo test --all 2>&1 | grep -E "test result|FAILED"

# Manual verification:
./target/release/scred-proxy --config test-config-with-rules.yaml &
curl localhost:9999/admin/secret?key=sk-123456
# Expected: sk-123456 NOT redacted

./target/release/scred --redact "regex:^sk-" < test-secrets.txt
# Expected: Only SK- patterns match

./target/release/scred --redact INVALID
# Expected: Error exit, message printed
```

**Release**: v1.0.1 with commit message:
```
v1.0.1: Critical security fixes

- Implement per-path rules enforcement in proxy
- Fix regex selector (now uses actual regex)
- Error on invalid selector instead of silent fallback

Fixes #1, #2, #3 from security audit
```

---

## PHASE 2: CROSS-COMPONENT CONSISTENCY (v1.1) - 8 HOURS

### Task 2.1: Pattern Name Validation (1 hour)

**What**: Return error on unknown pattern names instead of silent default  
**Where**: `crates/scred-http/src/pattern_metadata.rs`

**Checklist**:
- [ ] Change return type: `PatternTier` → `Result<PatternTier>`
- [ ] Update all call sites (3-5 places)
- [ ] Add error context
- [ ] Add compile-time validation in build.rs

**Effort**: 1 hour

---

### Task 2.2: Unified Selector Precedence (2 hours)

**What**: Make all three binaries follow same precedence: CLI > ENV > File > Default  
**Where**: All three: `crates/scred-cli/src/main.rs`, `crates/scred-mitm/src/main.rs`, `crates/scred-proxy/src/main.rs`

**Checklist**:
- [ ] Add --detect and --redact flags to proxy (currently missing)
- [ ] Implement consistent precedence function
- [ ] Add logging at each step
- [ ] Test all combinations

**Effort**: 2 hours

---

### Task 2.3: Env-Mode Selector Validation (1 hour)

**What**: Add logging + validation to env-mode redaction  
**Where**: `crates/scred-cli/src/env_mode.rs`

**Checklist**:
- [ ] Log which selector is being used
- [ ] Add debug logging of redaction results
- [ ] Expose selector methods on ConfigurableEngine

**Effort**: 1 hour

---

### Task 2.4: Unified Error Handling (1.5 hours)

**What**: Consistent error messages and suggestions across all components  
**Where**: Create `crates/scred-http/src/error_handling.rs`

**Checklist**:
- [ ] Create structured error types
- [ ] Add suggestion generation
- [ ] Use consistently in all three binaries
- [ ] Format errors with context

**Effort**: 1.5 hours

---

### Task 2.5: Multiline Secret Handling (2.5 hours)

**What**: Detect and redact secrets that span multiple lines  
**Where**: `crates/scred-cli/src/env_mode.rs`, MITM request/response handlers

**Checklist**:
- [ ] Implement LineAccumulator struct
- [ ] Detect continuation patterns (., whitespace, \)
- [ ] Buffer incomplete lines
- [ ] Flush on complete line
- [ ] Add tests for multiline JWT

**Effort**: 2.5 hours

---

### Phase 2 Validation

```bash
# All Phase 1 tests still pass
cargo test --all 2>&1 | grep "test result"

# New tests pass
cargo test selector_precedence
cargo test multiline_jwt
cargo test unknown_pattern_error

# Manual verification of precedence:
export SCRED_REDACT_PATTERNS=API_KEYS
./target/release/scred-cli --redact CRITICAL < test.txt
# Expected: CRITICAL redaction (CLI flag wins)
```

**Release**: v1.1 with commit message:
```
v1.1: Cross-component consistency and hardening

- Unify pattern selector precedence across all binaries
- Add pattern name validation
- Implement multiline secret detection
- Improve error handling consistency
- Add selector validation logging
```

---

## PHASE 3: MAXIMUM SECURITY (v1.2+) - 9 HOURS

### Task 3.1: URL-Decoded Secret Detection (1.5 hours)

**What**: Detect secrets in URL-encoded form  
**Where**: New function in `crates/scred-redactor/src/redactor.rs`

**Checklist**:
- [ ] Add urlencoding crate to Cargo.toml
- [ ] Implement `redact_with_url_decoding()`
- [ ] Compare warnings from plain vs decoded
- [ ] Merge results
- [ ] Test: sk%2Dproj... detects as sk-proj...

**Effort**: 1.5 hours

---

### Task 3.2: Mutual Exclusivity Validation (1 hour)

**What**: Warn on contradictory selector configurations  
**Where**: New function, called after selector parsing

**Checklist**:
- [ ] Implement validation function
- [ ] Check detect NONE + redact ALL (nonsensical)
- [ ] Check redact ⊂ detect
- [ ] Log warnings
- [ ] Test contradictory configs

**Effort**: 1 hour

---

### Task 3.3: Atomic Pattern Updates (2 hours)

**What**: Prevent race conditions on selector updates  
**Where**: New `AtomicPatternConfig` struct

**Checklist**:
- [ ] Use Arc<RwLock<>> for selectors
- [ ] Implement atomic update method
- [ ] Add loaded_at timestamp
- [ ] Prevent partial updates
- [ ] Test concurrent access

**Effort**: 2 hours

---

### Task 3.4: Selector Immutability (1 hour)

**What**: Lock selectors after startup to prevent runtime drift  
**Where**: New `ImmutablePatternSelector` struct

**Checklist**:
- [ ] Add locked flag
- [ ] Implement lock() method
- [ ] Block updates after lock
- [ ] Call lock() after startup
- [ ] Test locked selector rejects updates

**Effort**: 1 hour

---

### Task 3.5: Streaming Mode Verification (1.5 hours)

**What**: Verify selector is actually applied in streaming mode  
**Where**: `crates/scred-cli/src/main.rs` streaming handler

**Checklist**:
- [ ] Collect redaction statistics
- [ ] Verify patterns match selector
- [ ] Log mismatches as errors
- [ ] Add RedactionStats struct
- [ ] Test streaming with restricted selector

**Effort**: 1.5 hours

---

### Task 3.6: Cross-Component Consistency Check (1 hour)

**What**: Verify all patterns are in metadata at startup  
**Where**: New `verify_pattern_consistency()` function

**Checklist**:
- [ ] Check all redactor patterns in metadata
- [ ] Check all tier names valid
- [ ] Call at binary startup
- [ ] Exit on mismatch
- [ ] Log verification success

**Effort**: 1 hour

---

### Task 3.7: Audit Logging (1.5 hours)

**What**: Log all selector changes for compliance  
**Where**: Add audit logging to selector loading

**Checklist**:
- [ ] Create AuditLog struct
- [ ] Log selector changes with timestamp
- [ ] Include source (CLI/ENV/file/default)
- [ ] Optional user field
- [ ] Persist to audit.log if configured

**Effort**: 1.5 hours

---

### Phase 3 Validation

```bash
# All Phase 1+2 tests pass
cargo test --all

# New Phase 3 tests
cargo test url_decoded_secrets
cargo test mutual_exclusivity
cargo test atomic_selector_updates
cargo test selector_immutability
cargo test streaming_verification
cargo test audit_logging

# Compliance check
./target/release/scred --redact INVALID
# Logs to audit.log with error

# Manual: URL-encoded secret
echo '?api_key=sk%2Dproj%5F123456' | ./target/release/scred
# Expected: sk-proj_123456 detected
```

**Release**: v1.2 with commit message:
```
v1.2: Maximum security hardening

- Detect URL-encoded secrets
- Validate mutual exclusivity of selectors
- Implement atomic pattern updates
- Lock selectors after startup
- Verify streaming mode selector enforcement
- Add audit logging for compliance
- Cross-component consistency checks
```

---

## TOTAL EFFORT SUMMARY

| Phase | Tasks | Hours | Release | Focus |
|-------|-------|-------|---------|-------|
| **1** | 3 critical fixes | 5h | v1.0.1 | Blockers |
| **2** | 5 consistency fixes | 8h | v1.1 | Architecture |
| **3** | 7 hardening tasks | 9h | v1.2 | Security |
| **Total** | 15 tasks | 22h | All | Complete |

---

## EXECUTION STRATEGY

### Week 1: Phase 1 (Quick Win)
```
Monday: Tasks 1.1, 1.2 (4h)
Tuesday: Task 1.3 (1h) + testing (1h)
Release v1.0.1 Tuesday night
Continue planning Phase 2
```

### Week 2: Phase 2 (Consistency)
```
Monday-Tuesday: Tasks 2.1-2.3 (4h)
Wednesday: Tasks 2.4-2.5 (3.5h)
Thursday-Friday: Testing + release prep
Release v1.1 Friday
```

### Week 3: Phase 3 (Hardening)
```
Monday-Tuesday: Tasks 3.1-3.4 (5.5h)
Wednesday-Thursday: Tasks 3.5-3.7 (4h)
Friday: Final testing + release
Release v1.2
```

**Total Project Time**: 3 weeks to production hardening

---

## TESTING STRATEGY

### Unit Tests (Per-Task)
- Each task includes 2-3 new tests
- Run `cargo test --all` after each task
- Target: 100% pass rate maintained

### Integration Tests
- Test full CLI workflow: `cat file | scred`
- Test MITM with real HTTPS connections
- Test proxy with per-path rules
- Test all three binaries in parallel

### Manual Verification
- Live HTTPS redaction (httpbin.org)
- Corporate proxy chaining
- Configuration precedence
- Error messages

---

## ROLLBACK PLAN

If Phase N breaks v1.0:
1. Revert commits for Phase N
2. Maintain Phase N-1 fixes
3. Investigate blocker
4. Plan Phase N restart

Example: If Phase 2 breaks CLI, revert Phase 2 but keep Phase 1 (v1.0.1)

---

## RISK ASSESSMENT

| Phase | Risk | Mitigation |
|-------|------|-----------|
| 1 | Medium (3 critical changes) | Careful review, extensive testing |
| 2 | Medium (5 consistency changes) | Incremental implementation, backward compat |
| 3 | Low (7 advanced hardening) | New features, no breaking changes |

**Overall Risk**: LOW (all changes backward compatible, extensive testing)

---

## SUCCESS CRITERIA

### v1.0.1
- ✅ Per-path rules actually enforced
- ✅ Regex selector works with real regex
- ✅ Invalid selectors exit with error
- ✅ All Phase 1 tests pass
- ✅ No regressions to Phase 0

### v1.1
- ✅ All three binaries use consistent precedence
- ✅ Multiline secrets detected
- ✅ Error messages unified
- ✅ All Phase 1+2 tests pass
- ✅ Configuration validated at startup

### v1.2
- ✅ URL-encoded secrets detected
- ✅ Selector immutability enforced
- ✅ Audit logging working
- ✅ All Phase 1+2+3 tests pass
- ✅ Production hardening complete

---

## DELIVERABLES PER RELEASE

### v1.0.1
- Fixed code (3 components updated)
- New tests (3+ test cases)
- Updated CHANGELOG.md
- Release notes with "CRITICAL FIXES"

### v1.1
- Fixed code (6 components updated)
- New tests (5+ test cases)
- Updated CHANGELOG.md
- Updated README with consistency info
- Migration guide (if any breaking changes)

### v1.2
- Fixed code (9 components updated)
- New tests (7+ test cases)
- Updated CHANGELOG.md
- Security hardening guide
- Audit logging documentation

---

## GO/NO-GO DECISION POINTS

### Before v1.0.1 Release
- [ ] All Phase 1 tests pass (3/3 tasks)
- [ ] No new regressions
- [ ] Code review approved
- [ ] Manual testing complete

### Before v1.1 Release
- [ ] All Phase 1+2 tests pass (8/8 tasks)
- [ ] Consistency verified across all binaries
- [ ] No new regressions
- [ ] Performance acceptable (< 1ms overhead)

### Before v1.2 Release
- [ ] All Phase 1+2+3 tests pass (15/15 tasks)
- [ ] Security hardening verified
- [ ] Audit logging tested
- [ ] No known vulnerabilities

---

## CONCLUSION

**From v1.0 to v1.2 in 22 hours achievable pathway**:

1. **v1.0.1 (5h)**: Ship critical fixes immediately
2. **v1.1 (8h)**: Full cross-component consistency
3. **v1.2 (9h)**: Maximum security hardening

**Result**: Production-ready SCRED with:
- No security gaps
- Full configuration enforcement
- Complete cross-component consistency
- Advanced security features
- Comprehensive audit logging

**Ready to proceed?** Start with Phase 1 immediately.
