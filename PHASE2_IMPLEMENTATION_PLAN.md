# Phase 2: Tier 1 + JWT Generic Implementation (Zig)

## Status: Implementation Plan - Ready to Execute

**Scope**: Implement 72 high-confidence patterns (26 Tier 1 + 1 JWT + 45 Tier 2)  
**Streaming Compatible**: YES - all search-based, no buffering  
**Timeline**: 3-4 hours for complete implementation  
**Test Coverage**: 100% existing tests must pass  

---

## Architecture Overview

### Current State (lib.zig)
- 286 pattern definitions (existing Rust fallback)
- FirstCharLookup optimization (256 entry table)
- Inline prefix matching (fastPrefixMatch)
- FFI exports for C integration

### Proposed Phase 2 Implementation

```
lib.zig structure:
├─ Tier1Patterns[26] (pure prefix only)
├─ JwtPattern (eyJ + 2 dots validation)
├─ Tier2Patterns[45] (prefix + optional validation)
├─ detect_tier1(input: []const u8) → bool
├─ detect_jwt(input: []const u8) → bool
├─ detect_tier2(input: []const u8) → bool
└─ detect_all(input: []const u8) → bool (combines all three)
```

### Key Design Decisions

1. **Search-based detection** (not prefix-based)
   - Use `std.mem.indexOf()` to find patterns anywhere in chunk
   - Compatible with streaming (patterns can span chunks)
   - Works with lookahead buffer (512B)

2. **No length validation on patterns themselves**
   - Tier 1: Pure prefix (no length)
   - JWT: Structure only (2 dots, not length)
   - Tier 2: Length validation ONLY where format is fixed

3. **Streaming-First Implementation**
   - Each detector examines full combined input (lookahead + chunk)
   - Detectors return early on first match
   - Multiple patterns can match in one pass
   - No state retained between chunks

---

## Tier 1: Pure Prefix Patterns (26 patterns)

These have FIXED prefixes with ZERO false positives.

```zig
const Tier1Pattern = struct {
    name: []const u8,
    prefix: []const u8,
};

const TIER1_PATTERNS = [_]Tier1Pattern{
    .{ .name = "age-secret-key", .prefix = "AGE-SECRET-KEY-1" },
    .{ .name = "apideck", .prefix = "sk_live_" },
    .{ .name = "artifactoryreferencetoken", .prefix = "cmVmdGtu" },
    .{ .name = "azure-storage", .prefix = "AccountName" },
    .{ .name = "azure-app-config-connection-string", .prefix = "Endpoint=https://" },
    .{ .name = "coinbase", .prefix = "organizations/" },
    .{ .name = "fleetbase", .prefix = "flb_live_" },
    .{ .name = "flutterwave-public-key", .prefix = "FLWPUBK_TEST-" },
    .{ .name = "linear-api-key", .prefix = "lin_api_" },
    .{ .name = "linearapi", .prefix = "lin_api_" },
    .{ .name = "openaiadmin", .prefix = "sk-admin-" },
    .{ .name = "pagarme", .prefix = "ak_live_" },
    .{ .name = "planetscale-1", .prefix = "pscale_tkn_" },
    .{ .name = "planetscaledb-1", .prefix = "pscale_pw_" },
    .{ .name = "pypi-upload-token", .prefix = "pypi-AgEIcHlwaS5vcmc" },
    .{ .name = "ramp", .prefix = "ramp_id_" },
    .{ .name = "ramp-1", .prefix = "ramp_sec_" },
    .{ .name = "rubygems", .prefix = "rubygems_" },
    .{ .name = "salad-cloud-api-key", .prefix = "salad_cloud_" },
    .{ .name = "sentry-access-token", .prefix = "bsntrys_" },
    .{ .name = "sentryorgtoken", .prefix = "sntrys_" },
    .{ .name = "stripepaymentintent-2", .prefix = "pk_live_" },
    .{ .name = "travisoauth", .prefix = "travis_" },
    .{ .name = "tumblr-api-key", .prefix = "tumblr_" },
    // 2 more patterns to be added from gitleaks analysis
};

fn detect_tier1(input: []const u8) -> bool {
    for (TIER1_PATTERNS) |pattern| {
        if (std.mem.indexOf(u8, input, pattern.prefix) != null) {
            return true;  // Found prefix anywhere in chunk
        }
    }
    return false;
}

fn detect_tier1_with_positions(input: []const u8, allocator: Allocator) -> std.ArrayList(usize) {
    var positions = std.ArrayList(usize).init(allocator);
    
    var i: usize = 0;
    while (i < input.len) {
        for (TIER1_PATTERNS) |pattern| {
            if (i + pattern.prefix.len <= input.len and
                std.mem.eql(u8, input[i..i+pattern.prefix.len], pattern.prefix)) {
                positions.append(i) catch {};
                i += pattern.prefix.len;
                continue;
            }
        }
        i += 1;
    }
    
    return positions;
}
```

---

## JWT: Generic Detector (1 pattern)

Detects ANY valid JWT (all algorithms, all sizes).

```zig
fn is_jwt_delimiter(byte: u8) -> bool {
    return byte == ' ' or byte == '\n' or byte == '\t' or 
           byte == '\r' or byte == ',' or byte == ';' or
           byte == '}' or byte == ')' or byte == ']' or
           byte == '\'' or byte == '"' or byte == '<' or 
           byte == '>' or byte == '&' or byte == '|' or
           byte == ':' or byte == '=' or byte == '/' or
           byte == '?';
}

fn extract_jwt_token(input: []const u8, start: usize) -> []const u8 {
    if (start + 3 > input.len) return "";
    
    var end = start + 3;  // Start after "eyJ"
    while (end < input.len and !is_jwt_delimiter(input[end])) {
        end += 1;
        if (end - start > 10000) break;  // Sanity limit
    }
    
    return input[start..end];
}

fn has_valid_jwt_structure(token: []const u8) -> bool {
    if (token.len < 7) return false;  // Minimum: eyJ.a.b
    
    var dot_count: u8 = 0;
    for (token) |byte| {
        if (byte == '.') dot_count += 1;
    }
    
    return dot_count == 2;
}

fn detect_jwt(input: []const u8) -> bool {
    if (input.len < 7) return false;  // Minimum JWT size
    
    var i: usize = 0;
    while (i + 3 <= input.len) {
        if (input[i] == 'e' and input[i+1] == 'y' and input[i+2] == 'J') {
            const token = extract_jwt_token(input, i);
            if (has_valid_jwt_structure(token)) {
                return true;
            }
        }
        i += 1;
    }
    
    return false;
}

fn detect_jwt_with_positions(input: []const u8, allocator: Allocator) -> std.ArrayList(usize) {
    var positions = std.ArrayList(usize).init(allocator);
    
    var i: usize = 0;
    while (i + 3 <= input.len) {
        if (input[i] == 'e' and input[i+1] == 'y' and input[i+2] == 'J') {
            const token = extract_jwt_token(input, i);
            if (has_valid_jwt_structure(token)) {
                positions.append(i) catch {};
                i = i + token.len;  // Skip past found token
                continue;
            }
        }
        i += 1;
    }
    
    return positions;
}
```

---

## Tier 2: Prefix + Validation (45 patterns)

These require prefix + optional length/charset validation.

```zig
const Tier2Pattern = struct {
    name: []const u8,
    prefix: []const u8,
    min_len: usize,       // Token length after prefix (0 = no validation)
    max_len: usize,       // 0 = no max
    charset: Charset,     // What characters are allowed
};

const Charset = enum {
    alphanumeric,
    base64,
    base64url,
    hex,
    hex_lowercase,
    any,  // Accept anything until delimiter
};

fn is_valid_char_in_charset(byte: u8, charset: Charset) -> bool {
    switch (charset) {
        .alphanumeric => return std.ascii.isAlphanumeric(byte),
        .base64 => return (byte >= 'A' and byte <= 'Z') or
                          (byte >= 'a' and byte <= 'z') or
                          (byte >= '0' and byte <= '9') or
                          byte == '+' or byte == '/' or byte == '=',
        .base64url => return (byte >= 'A' and byte <= 'Z') or
                             (byte >= 'a' and byte <= 'z') or
                             (byte >= '0' and byte <= '9') or
                             byte == '-' or byte == '_' or byte == '=',
        .hex => return (byte >= '0' and byte <= '9') or
                       (byte >= 'a' and byte <= 'f') or
                       (byte >= 'A' and byte <= 'F'),
        .hex_lowercase => return (byte >= '0' and byte <= '9') or
                                  (byte >= 'a' and byte <= 'f'),
        .any => return !is_jwt_delimiter(byte),
    }
}

const TIER2_PATTERNS = [_]Tier2Pattern{
    .{ 
        .name = "1password-service-account-token",
        .prefix = "ops_eyJ",
        .min_len = 250,
        .max_len = 0,  // No max
        .charset = .base64,
    },
    .{ 
        .name = "anthropic",
        .prefix = "sk-ant-",
        .min_len = 90,
        .max_len = 100,
        .charset = .any,
    },
    .{ 
        .name = "artifactory-api-key",
        .prefix = "AKCp",
        .min_len = 69,
        .max_len = 69,  // Exactly 69
        .charset = .alphanumeric,
    },
    // ... 42 more patterns
};

fn detect_tier2(input: []const u8) -> bool {
    for (TIER2_PATTERNS) |pattern| {
        var i: usize = 0;
        while (i + pattern.prefix.len <= input.len) {
            if (std.mem.eql(u8, input[i..i+pattern.prefix.len], pattern.prefix)) {
                // Found prefix, validate token
                var token_end = i + pattern.prefix.len;
                
                while (token_end < input.len and 
                       is_valid_char_in_charset(input[token_end], pattern.charset)) {
                    token_end += 1;
                }
                
                const token_len = token_end - (i + pattern.prefix.len);
                
                // Apply length validation
                if (pattern.min_len > 0 and token_len < pattern.min_len) {
                    i += 1;
                    continue;
                }
                if (pattern.max_len > 0 and token_len > pattern.max_len) {
                    i += 1;
                    continue;
                }
                
                return true;  // Valid token found
            }
            i += 1;
        }
    }
    
    return false;
}
```

---

## Combined Detection (All Tiers)

```zig
pub fn detect_all_patterns(input: []const u8) -> bool {
    // Check in order: Tier 1 (fastest) → JWT → Tier 2
    
    if (detect_tier1(input)) return true;
    if (detect_jwt(input)) return true;
    if (detect_tier2(input)) return true;
    
    return false;
}
```

---

## Integration with Streaming Redactor

When StreamingRedactor processes chunks:

```zig
// Current call from Rust:
// scred_detector_process(detector, input_ptr, input_len, is_eof)

// Internal logic:
fn process_chunk_internal(input: []const u8, is_eof: bool) -> bool {
    // 1. Check Tier 1 (pure prefix)
    if (detect_tier1(input)) {
        // Redact all Tier 1 matches
        return redact_tier1_matches(input);
    }
    
    // 2. Check JWT
    if (detect_jwt(input)) {
        // Redact all JWT matches
        return redact_jwt_matches(input);
    }
    
    // 3. Check Tier 2
    if (detect_tier2(input)) {
        // Redact all Tier 2 matches
        return redact_tier2_matches(input);
    }
    
    // 4. No patterns found, passthrough
    return false;
}
```

---

## Implementation Steps

### Step 1: Add Tier 1 Pattern Definitions
```zig
const TIER1_PATTERNS = [_]Tier1Pattern{ ... };
fn detect_tier1(input: []const u8) -> bool { ... }
```

Estimated time: 30 minutes  
Files: lib.zig (patterns section)  
Tests: Pass if Tier 1 patterns detected

### Step 2: Add JWT Generic Detector
```zig
fn detect_jwt(input: []const u8) -> bool { ... }
```

Estimated time: 1 hour  
Files: lib.zig (jwt section)  
Tests:
- Short JWT (HS256, 100 bytes)
- Long JWT (RS512, 1000+ bytes)
- Multiple JWTs in one chunk
- JWT spanning chunk boundaries (use lookahead)

### Step 3: Add Tier 2 Pattern Definitions
```zig
const TIER2_PATTERNS = [_]Tier2Pattern{ ... };
fn detect_tier2(input: []const u8) -> bool { ... }
```

Estimated time: 1.5 hours  
Files: lib.zig (tier2 section)  
Tests: Pass if Tier 2 patterns detected with validation

### Step 4: Combine All Detectors
```zig
pub fn detect_all_patterns(input: []const u8) -> bool { ... }
```

Estimated time: 30 minutes  
Files: lib.zig (public API)  
Tests: Integration tests with mixed pattern types

### Step 5: Streaming Validation
- Test with 64KB chunks
- Test with patterns spanning chunk boundaries
- Test with lookahead buffer (512B)
- Verify memory usage bounded

Estimated time: 1 hour  
Files: lib.zig + streaming tests  
Tests: Multi-chunk streaming scenarios

---

## Testing Strategy

### Unit Tests (Tier 1)
```zig
test "tier1-prefix-age-secret-key" {
    const input = "AGE-SECRET-KEY-1QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L";
    try std.testing.expect(detect_tier1(input));
}

test "tier1-multiple-matches" {
    const input = "something sk_live_xxx and another rubygems_yyy";
    try std.testing.expect(detect_tier1(input));
}
```

### Unit Tests (JWT)
```zig
test "jwt-valid-hs256" {
    // Short JWT
    const input = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    try std.testing.expect(detect_jwt(input));
}

test "jwt-valid-rs512" {
    // Long JWT
    const input = "eyJhbGciOiJSUzUxMiIsInR5cCI6IkpXVCJ9." + very_long_payload;
    try std.testing.expect(detect_jwt(input));
}

test "jwt-invalid-single-dot" {
    const input = "eyJ.something";
    try std.testing.expect(!detect_jwt(input));
}

test "jwt-spanning-chunks" {
    // Lookahead=512B, chunk=64KB
    // JWT split at position 500 (in lookahead + chunk boundary)
    const input = make_jwt_at_position(500);
    try std.testing.expect(detect_jwt(input));
}
```

### Unit Tests (Tier 2)
```zig
test "tier2-anthropic" {
    const input = "sk-ant-admin01-" + ("x" * 93) + "AA";
    try std.testing.expect(detect_tier2(input));
}

test "tier2-artifactory-exact-length" {
    const input = "AKCp" + ("x" * 69);
    try std.testing.expect(detect_tier2(input));
}
```

### Integration Tests (Streaming)
```zig
test "streaming-tier1-in-64kb-chunk" {
    const chunk = make_chunk_with_tier1_pattern(64 * 1024);
    try std.testing.expect(detect_all_patterns(chunk));
}

test "streaming-pattern-spanning-chunks" {
    const lookahead = make_pattern_prefix(512);
    const chunk = make_pattern_suffix(64 * 1024);
    const combined = lookahead ++ chunk;
    try std.testing.expect(detect_all_patterns(combined));
}

test "streaming-multiple-patterns-same-chunk" {
    const chunk = "...AGE-SECRET-KEY-1xxx...ops_eyJ...sk-ant-yyy...";
    try std.testing.expect(detect_all_patterns(chunk));
}
```

---

## Success Criteria

✅ Phase 2 Complete when:
1. All 26 Tier 1 patterns defined and detectable
2. JWT generic detector working for all JWT sizes
3. All 45 Tier 2 patterns defined and detectable (with validation)
4. All existing 458 tests still passing (zero regressions)
5. Streaming tests pass (patterns spanning chunks)
6. Build succeeds: `cargo build --release`
7. Benchmarks maintain throughput (50+ MB/s)

---

## Tier 2 Patterns (Complete List for Reference)

1. 1password-service-account-token → ops_eyJ (base64, 250+ chars)
2. anthropic → sk-ant- (90-100 chars, any)
3. artifactory-api-key → AKCp (exactly 69 chars, alphanumeric)
4. assertible → assertible_ (any length)
5. atlassian-token → AAAAA (specific format)
6. checkr-personal-access-token → chk_live_ (base64url)
7. circleci-personal-access-token → ccpat_ (base64url)
8. contentfulpersonalaccesstoken → CFPAT- (exactly 43 chars)
9. databrickstoken-1 → dapi (hex(32) + optional dash+digit)
10. digicert-api-key → d7dc (any length)
11. docusign-api-key → (docusign specific)
12. duffel-api-token → duffel_ (test|live mode + 43 chars)
... (35 more)

---

## Estimated Total Time

| Step | Duration | Status |
|------|----------|--------|
| 1. Tier 1 patterns | 30 min | Ready |
| 2. JWT detector | 1 hour | Ready |
| 3. Tier 2 patterns | 1.5 hours | Ready |
| 4. Integration | 30 min | Ready |
| 5. Streaming validation | 1 hour | Ready |
| **TOTAL** | **4.5 hours** | **Ready to Start** |

---

## Next Actions

1. ✅ TODO-798afb2b updated with streaming compatibility
2. ⏳ **NOW**: Execute Phase 2 implementation (Steps 1-5 above)
3. ⏳ Create comprehensive test suite
4. ⏳ Validate all 72 patterns detected correctly
5. ⏳ Run benchmark suite
6. ⏳ Verify no regressions (458/458 tests passing)

---

## Files to Modify

- `crates/scred-pattern-detector/src/lib.zig` - Add 72 pattern definitions + detectors
- `crates/scred-pattern-detector/tests/` - Add streaming validation tests
- `crates/scred-pattern-detector/benches/` - Verify throughput maintained

---

## References

- TODO-798afb2b: Full streaming compatibility assessment
- PATTERN_TIERS.md: Complete pattern categorization (11,800 lines)
- REASSESSMENT_JWT_CONSOLIDATION.md: JWT consolidation rationale

**Status**: 🟢 READY FOR IMPLEMENTATION
