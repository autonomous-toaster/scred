# Bug Assessment: scred-cli with SCRED_DETECT_PATTERNS

## TL;DR: Not a Bug, By Design (Phase 6 Deferred Feature)

**Status:** ❌ **NOT A BUG** - Feature is deferred to Phase 6  
**Severity:** N/A (Not implemented yet)  
**Workaround:** Use `scred-mitm` instead of `scred-cli`  

---

## The Issue You Encountered

```bash
echo 'curl command' | SCRED_DETECT_PATTERNS=all cargo run --bin scred
# Result: Command runs, but SCRED_DETECT_PATTERNS is ignored
# Expected: scred CLI respects tier selection
# Actual: scred CLI redacts ALL patterns (no filtering)
```

---

## Root Cause Analysis

### Why This Doesn't Work

**The scred-cli architecture is simpler than scred-mitm:**

```
scred-cli dependencies:
├── scred-redactor (242 patterns, no tier support)
└── NO scred-http (no PatternSelector/PatternTier)

scred-mitm dependencies:
├── scred-redactor (242 patterns)
├── scred-http (includes PatternSelector, PatternTier)
└── PLUS: CLI flag parsing for --detect, --redact
```

**Specifically:**
1. scred-cli does NOT depend on scred-http
2. scred-http contains PatternSelector and PatternTier
3. scred-cli has no way to parse SCRED_DETECT_PATTERNS
4. scred-cli has no knowledge of tiers

---

## Why It's Not Implemented

From the session summary: **Phase 6 (scred-cli tier support) was explicitly DEFERRED**

```
### Short-term (Follow-up - After Phase 5)
2. **Phase 6: scred-cli Tier Support** (2-3 hours, DEFERRED)
   - Add same --detect/--redact flags to CLI
   - Use default: detect all, redact CRITICAL+API_KEYS
```

**Reason for deferral:**
- scred-cli is simpler and does one thing: redact ALL patterns
- Pattern tier filtering is more important for scred-mitm (production proxy)
- scred-cli can stay "simple and dumb" (all-or-nothing)
- If needed, users can use scred-mitm instead

---

## What You Actually Have

### For scred-cli (Current)
```bash
# Works (redacts everything)
echo 'secret content' | scred

# Does NOT work (ignored)
SCRED_DETECT_PATTERNS=all scred
SCRED_REDACT_PATTERNS=CRITICAL scred
scred --detect CRITICAL
```

### For scred-mitm (Implemented in Phase 3)
```bash
# All of these work
./scred-mitm --detect CRITICAL,API_KEYS
./scred-mitm --redact CRITICAL
./scred-mitm --list-tiers
SCRED_DETECT_PATTERNS=all ./scred-mitm
SCRED_REDACT_PATTERNS=CRITICAL,API_KEYS ./scred-mitm
```

---

## What It Would Take to Fix (Phase 6)

### Implementation Requirements

| Step | Time | Details |
|------|------|---------|
| 1. Add scred-http dep to scred-cli | 15min | Update Cargo.toml |
| 2. Parse env vars | 30min | SCRED_DETECT_PATTERNS, SCRED_REDACT_PATTERNS |
| 3. Add CLI flags | 30min | --detect, --redact, --list-tiers |
| 4. Filter RedactionEngine output | 30min | Apply tier selector to patterns |
| 5. Testing | 1-2hr | Create test cases |
| **TOTAL** | **2-3 hours** | Full Phase 6 implementation |

### Implementation Approach

**Option A: Lightweight (Recommended)**
- Add pattern list filtering (no tiers, just allow --patterns pattern1,pattern2)
- Doesn't require tier system
- Simpler than full tier support

**Option B: Full Tier Support**
- Add scred-http dependency to scred-cli
- Import PatternSelector, PatternTier, get_pattern_tier()
- Parse CLI flags and env vars
- Filter RedactionEngine patterns before redaction

**Option C: Standalone (Most Work)**
- Duplicate tier system in scred-redactor
- No cross-crate dependency
- More maintenance burden

---

## Workarounds

### If You Need Tier Control NOW

**Option 1: Use scred-mitm Instead**
```bash
# Start MITM proxy on localhost:8080
./scred-mitm --detect all --redact CRITICAL,API_KEYS &

# Then route your traffic through it
curl -x http://127.0.0.1:8080 https://httpbin.org/anything \
  -H 'x-custom: hello' \
  -d 'aaa=bbb' \
  -H 'x-secret: test_key'
```

**Option 2: Use scred-cli as-is (All or Nothing)**
```bash
# scred-cli redacts ALL patterns - no filtering
echo 'some content with secrets' | scred > output.txt

# If you want selective redaction, this isn't available yet
```

**Option 3: Pre-filter Your Input**
```bash
# Manually construct input with only patterns you want redacted
# (workaround, not practical for large scale)
```

---

## My Recommendation

### Short-term (For Your Use Case)
- Use `scred-mitm --detect all` instead of `scred-cli`
- scred-mitm has full tier support
- Can be used as a forward proxy

### Medium-term (If CLI Tier Support Needed)
- Implement Phase 6 (2-3 hours)
- Add tier support to scred-cli
- Then SCRED_DETECT_PATTERNS will work

### Long-term
- Keep scred-cli simple (no filtering)
- Or add optional tier support
- Decision depends on use cases

---

## Decision

### Is This a Bug?
**No.** This is a deferred feature (Phase 6).

### Should It Be Fixed?
**Maybe.** Depends on your needs:
- If you're using scred-cli for end-user tool: Implement Phase 6
- If you're using scred-mitm as proxy: Already works ✅
- If you just want to redact everything: scred-cli already works ✅

### My Suggestion
Use **scred-mitm** for now. It has full tier support and is more powerful:
```bash
./scred-mitm --detect CRITICAL,API_KEYS,INFRASTRUCTURE --redact CRITICAL,API_KEYS
# Then route traffic through proxy on 8080
```

---

## Summary Table

| Feature | scred-cli | scred-mitm |
|---------|-----------|-----------|
| Pattern redaction | ✅ All | ✅ All |
| Tier filtering | ❌ No | ✅ Yes |
| --detect flag | ❌ No | ✅ Yes |
| --redact flag | ❌ No | ✅ Yes |
| Env var support | ❌ No | ✅ Yes |
| HTTPS interception | ❌ No | ✅ Yes |
| Proxy mode | ❌ No | ✅ Yes |

**Best tool for tier-based filtering: scred-mitm ✅**

