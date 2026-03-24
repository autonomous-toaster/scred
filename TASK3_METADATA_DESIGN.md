# TASK 3: STREAMING METADATA DESIGN

## Overview

Design comprehensive metadata architecture for SCRED pattern detection system that supports:
- Cross-component validation (Detection → Redaction → Streaming)
- Tier-based pattern selection and filtering
- Streaming-aware metadata (for chunked processing)
- FFI exposure of pattern metadata
- Runtime configuration and caching

**Status**: IN PROGRESS (Estimated 2 hours)
**Started**: 2026-03-23 (after Task 2 completion)

---

## Requirements (From Task 2 Results)

### Mandatory Metadata Fields

From TASK2_PHASE1_CLASSIFICATION_COMPLETE.md:

```
Per-pattern metadata required:
✅ name (pattern identifier)
✅ category (SIMPLE_PREFIX, CAT_A, CAT_B, CAT_C, JWT, CAT_D)
✅ tier (critical, api_keys, infrastructure, services, patterns)
✅ risk_score (1-100, derived from tier)
✅ ffi_function (execution path)
✅ prefix (if applicable)
✅ charset (if applicable)
✅ length_constraints (min, max, fixed)
✅ regex_pattern (if applicable)
✅ example_secret (for validation)
✅ tags (service, protocol, format)
```

### Tier Definitions (From Task 2)

```
Tier 1: CRITICAL
  - AWS Access Keys, GitHub Tokens, Stripe Keys
  - Count: 26 patterns
  - Risk Score: 95
  - Default Redaction: TRUE
  - Impact: Full account compromise

Tier 2: API_KEYS
  - OpenAI, Anthropic, Twilio, SendGrid
  - Count: 200 patterns
  - Risk Score: 80
  - Default Redaction: TRUE
  - Impact: Service compromise

Tier 3: INFRASTRUCTURE
  - K8s, Docker, Grafana, Datadog tokens
  - Count: 20 patterns
  - Risk Score: 60
  - Default Redaction: FALSE
  - Impact: Infrastructure compromise

Tier 4: SERVICES
  - Payment processors, communication services
  - Count: 19 patterns
  - Risk Score: 40
  - Default Redaction: FALSE
  - Impact: Service disruption

Tier 5: PATTERNS
  - Generic JWT, Bearer tokens, Basic auth
  - Count: 9 patterns
  - Risk Score: 30
  - Default Redaction: FALSE
  - Impact: False positives risk
```

### Cross-Component Requirements

**Detection → Redaction Validation**:
- Pattern detected with metadata
- Redaction tier check (is pattern in redaction set?)
- Redaction execution if allowed

**Streaming Metadata**:
- Pattern metadata available during streaming chunk processing
- Efficient tier lookup (O(1) preferred)
- Memory-efficient for long-running processes

**FFI Exposure**:
- Metadata queryable via FFI for Rust components
- Pattern info retrieval (get_pattern_info)
- Tier lookup (get_pattern_tier)

---

## Architecture Design: Three-Layer Metadata System

### Layer 1: Source of Truth (Zig - patterns.zig)

**Location**: `crates/scred-pattern-detector/src/patterns.zig`

**Structure**: Pattern metadata table with all 274 entries

```zig
const PatternMetadata = struct {
    name: []const u8,
    category: PatternCategory,
    tier: RiskTier,
    risk_score: u8,
    ffi_function: FFIPath,
    
    // Prefix validation
    prefix: ?[]const u8,
    prefix_len: u16,
    
    // Charset for validation
    charset: CharsetType,
    
    // Length constraints
    length: LengthConstraint,
    
    // Regex pattern (for CAT_D)
    regex_pattern: ?[]const u8,
    
    // Example secret for testing
    example_secret: []const u8,
    
    // Tags for filtering
    tags: []const []const u8,
};

const RiskTier = enum {
    critical,      // 26 patterns
    api_keys,      // 200 patterns
    infrastructure, // 20 patterns
    services,      // 19 patterns
    patterns,      // 9 patterns
};

const PatternCategory = enum {
    simple_prefix,
    prefix_val_a,   // Fixed length
    prefix_val_b,   // Min length
    prefix_val_c,   // Variable
    jwt_pattern,
    regex,
};

const FFIPath = enum {
    match_prefix,
    prefix_charset,
    prefix_length,
    prefix_minlen,
    regex_match,
    jwt_special,
};

const CharsetType = enum {
    alphanumeric,
    hex,
    base64,
    base64url,
    numeric,
    custom,
};

const LengthConstraint = struct {
    min_length: u16,
    max_length: u16,
    fixed_length: ?u16,
};
```

**Example Entries** (from Task 2):

```zig
const patterns = [_]PatternMetadata{
    // CRITICAL TIER - AWS
    .{
        .name = "aws-access-key",
        .category = .regex,
        .tier = .critical,
        .risk_score = 95,
        .ffi_function = .regex_match,
        .prefix = null,
        .charset = .alphanumeric,
        .length = .{ .min_length = 20, .max_length = 20, .fixed_length = 20 },
        .regex_pattern = "AKIA[0-9A-Z]{16}",
        .example_secret = "AKIA1234567890ABCDEF",
        .tags = &.{ "aws", "critical", "credentials" },
    },
    
    // API_KEYS TIER - GitHub
    .{
        .name = "github-token",
        .category = .prefix_val_b,
        .tier = .api_keys,
        .risk_score = 90,
        .ffi_function = .prefix_minlen,
        .prefix = "ghp_",
        .prefix_len = 4,
        .charset = .alphanumeric,
        .length = .{ .min_length = 36, .max_length = 255, .fixed_length = null },
        .example_secret = "ghp_AbCdEfGhIjKlMnOpQrStUvWxYzAbCdEfGhIjKlMnOpQr",
        .tags = &.{ "github", "api_key", "critical" },
    },
    
    // PATTERNS TIER - JWT
    .{
        .name = "jwt",
        .category = .jwt_pattern,
        .tier = .patterns,
        .risk_score = 30,
        .ffi_function = .jwt_special,
        .prefix = "eyJ",
        .prefix_len = 3,
        .charset = .base64url,
        .length = .{ .min_length = 20, .max_length = 4096, .fixed_length = null },
        .example_secret = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        .tags = &.{ "jwt", "generic", "bearer" },
    },
};
```

### Layer 2: Rust FFI Bindings

**Location**: `crates/scred-pattern-detector/src/lib.rs`

**FFI Functions for Metadata**:

```rust
#[repr(C)]
pub struct PatternMetadataFFI {
    pub name: *const u8,
    pub name_len: usize,
    pub category: u8,          // 0-5: simple, cat_a, cat_b, cat_c, jwt, regex
    pub tier: u8,              // 0-4: critical, api_keys, infrastructure, services, patterns
    pub risk_score: u8,        // 0-100
    pub ffi_function: u8,      // 0-5: match_prefix, charset, length, minlen, regex, jwt
    pub prefix: *const u8,
    pub prefix_len: usize,
    pub charset_type: u8,      // 0-5: alphanumeric, hex, base64, base64url, numeric, custom
    pub min_length: u16,
    pub max_length: u16,
    pub fixed_length: u16,     // 0 if variable
    pub regex_pattern: *const u8,
    pub regex_len: usize,
    pub example_secret: *const u8,
    pub example_len: usize,
}

// FFI Function: Get pattern metadata by name
#[no_mangle]
pub extern "C" fn scred_pattern_get_metadata(
    name: *const u8,
    name_len: usize,
) -> PatternMetadataFFI {
    // Implementation: Lookup in patterns table, return metadata
}

// FFI Function: Get pattern metadata by index
#[no_mangle]
pub extern "C" fn scred_pattern_get_by_index(index: usize) -> PatternMetadataFFI {
    // Implementation: Return pattern[index] metadata
}

// FFI Function: Get tier of pattern
#[no_mangle]
pub extern "C" fn scred_pattern_get_tier(
    name: *const u8,
    name_len: usize,
) -> u8 {
    // Implementation: Return tier (0-4)
}

// FFI Function: Get total pattern count
#[no_mangle]
pub extern "C" fn scred_pattern_count() -> usize {
    // Implementation: Return 274
}

// FFI Function: Check if pattern in tier set
#[no_mangle]
pub extern "C" fn scred_pattern_in_tier(
    name: *const u8,
    name_len: usize,
    tier: u8,
) -> bool {
    // Implementation: pattern.tier == tier
}
```

### Layer 3: Rust Runtime Cache (scred-redactor)

**Location**: `crates/scred-redactor/src/metadata_cache.rs`

**Caching Strategy**:

```rust
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct MetadataCache {
    // By-name lookup: O(1) via HashMap
    patterns_by_name: HashMap<String, PatternMetadata>,
    
    // By-tier lookup: O(1) via pre-computed vectors
    patterns_by_tier: HashMap<RiskTier, Vec<String>>,
    
    // Tag-based lookup: O(1) via HashMap<tag, Vec<pattern_name>>
    patterns_by_tag: HashMap<String, Vec<String>>,
    
    // Total count
    total_patterns: usize,
}

impl MetadataCache {
    pub fn new() -> Self {
        // Initialize from Zig patterns.zig via FFI
        let mut cache = MetadataCache {
            patterns_by_name: HashMap::new(),
            patterns_by_tier: HashMap::new(),
            patterns_by_tag: HashMap::new(),
            total_patterns: unsafe { scred_pattern_count() },
        };
        
        // Load all patterns from Zig
        for idx in 0..cache.total_patterns {
            let metadata_ffi = unsafe { scred_pattern_get_by_index(idx) };
            let metadata = PatternMetadata::from_ffi(&metadata_ffi);
            
            // Index by name
            cache.patterns_by_name.insert(metadata.name.clone(), metadata.clone());
            
            // Index by tier
            cache.patterns_by_tier
                .entry(metadata.tier.clone())
                .or_insert_with(Vec::new)
                .push(metadata.name.clone());
            
            // Index by tags
            for tag in &metadata.tags {
                cache.patterns_by_tag
                    .entry(tag.clone())
                    .or_insert_with(Vec::new)
                    .push(metadata.name.clone());
            }
        }
        
        cache
    }
    
    pub fn get_pattern(&self, name: &str) -> Option<&PatternMetadata> {
        self.patterns_by_name.get(name)
    }
    
    pub fn get_patterns_by_tier(&self, tier: &RiskTier) -> Option<&[String]> {
        self.patterns_by_tier.get(tier).map(|v| v.as_slice())
    }
    
    pub fn get_patterns_by_tag(&self, tag: &str) -> Option<&[String]> {
        self.patterns_by_tag.get(tag).map(|v| v.as_slice())
    }
}

// Global cache singleton
pub static METADATA_CACHE: OnceLock<MetadataCache> = OnceLock::new();

pub fn get_cache() -> &'static MetadataCache {
    METADATA_CACHE.get_or_init(|| MetadataCache::new())
}
```

---

## Cross-Component Validation Flow

### Detection → Redaction Pipeline

```
INPUT: HTTP request/response

STEP 1: DETECTION
  └─ Call: scred_detector_new() → PatternDetector
  └─ Process: Match all 274 patterns via FFI
  └─ Output: Vec<DetectionEvent> with pattern name + position

STEP 2: METADATA LOOKUP (new)
  └─ For each DetectionEvent:
       - Get metadata: metadata_cache.get_pattern(event.name)
       - Read tier: metadata.tier
       - Read risk_score: metadata.risk_score
       - Output: EnrichedDetectionEvent { event, tier, risk_score }

STEP 3: REDACTION FILTER (new)
  └─ Configuration: SCRED_REDACT_TIERS=critical,api_keys (or env/config)
  └─ For each EnrichedDetectionEvent:
       - Check: is metadata.tier in REDACT_TIERS?
       - If YES: mark for redaction
       - If NO: skip redaction (keep in output)

STEP 4: REDACTION
  └─ Call: scred_redactor_redact(...) with enriched events
  └─ Process: Apply redaction only to marked events
  └─ Output: Redacted content

STEP 5: STREAMING (if needed)
  └─ Call: scred_streaming_redact(chunk, metadata_cache)
  └─ Process: Conditional lookahead with metadata awareness
  └─ Output: Redacted chunk
```

### Example: GitHub Token Detection + Redaction

```
REQUEST:
  Authorization: Bearer ghp_AbCdEfGhIjKlMnOpQrStUvWxYzAbCdEfGhIjKlMnOpQr

DETECTION (FFI):
  ✅ Pattern matched: github-token
  └─ Position: 21-68
  └─ Match: ghp_AbCdEfGhIjKlMnOpQrStUvWxYzAbCdEfGhIjKlMnOpQr

METADATA LOOKUP:
  ✅ Pattern found: github-token
  ├─ tier: api_keys
  ├─ risk_score: 90
  ├─ category: prefix_val_b
  ├─ prefix: ghp_
  └─ length: min=36, max=255

REDACTION FILTER:
  Config: SCRED_REDACT_TIERS=critical,api_keys
  ✅ api_keys IN redact_tiers? YES
  └─ Mark for redaction

REDACTION:
  ✅ Apply redaction
  └─ Output: Authorization: Bearer [REDACTED_60]

STREAMING (chunked):
  ├─ Chunk 1 (32B): "Authorization: Bearer ghp_AbCdE"
  │  └─ Metadata: prefix ghp_ found, partial match
  │  └─ Lookahead: maintain buffer
  ├─ Chunk 2 (38B): "fGhIjKlMnOpQrStUvWxYzAbCdEfGhIjKlMnOpQr"
  │  └─ Metadata: complete match now
  │  └─ Output: Apply redaction across lookahead + chunk
```

---

## Streaming-Aware Design

### Metadata for Streaming Detection

**Problem**: In chunked streaming, patterns might span multiple chunks.

**Solution**: Metadata enables smart lookahead management:

```rust
pub struct StreamingMetadataContext {
    /// Maximum pattern length from metadata
    pub max_pattern_length: usize,
    
    /// Maximum prefix length
    pub max_prefix_length: usize,
    
    /// Patterns to watch for (filtered by tier/category)
    pub patterns_to_match: Vec<String>,
    
    /// For each pattern: how much lookahead needed?
    pub lookahead_requirements: HashMap<String, usize>,
}

impl StreamingMetadataContext {
    pub fn new(tier_filter: Option<&RiskTier>) -> Self {
        let cache = get_cache();
        
        // Determine which patterns to use
        let patterns_to_match = if let Some(tier) = tier_filter {
            cache.get_patterns_by_tier(tier).unwrap_or(&[]).to_vec()
        } else {
            cache.patterns_by_name.keys().cloned().collect()
        };
        
        // Calculate maximum lookahead needed
        let mut max_pattern_length = 0;
        let mut max_prefix_length = 0;
        let mut lookahead_requirements = HashMap::new();
        
        for pattern_name in &patterns_to_match {
            if let Some(metadata) = cache.get_pattern(pattern_name) {
                max_pattern_length = max_pattern_length.max(metadata.max_length);
                max_prefix_length = max_prefix_length.max(metadata.prefix_len);
                
                // Pattern lookahead = max_length (to catch full pattern)
                lookahead_requirements.insert(
                    pattern_name.clone(),
                    metadata.max_length,
                );
            }
        }
        
        StreamingMetadataContext {
            max_pattern_length,
            max_prefix_length,
            patterns_to_match,
            lookahead_requirements,
        }
    }
    
    pub fn lookahead_size(&self) -> usize {
        // Lookahead = max pattern length + some buffer
        self.max_pattern_length + 128  // 128B safety buffer
    }
}
```

**Usage in Streaming Redaction**:

```rust
pub fn streaming_redact_with_metadata(
    input: &[u8],
    chunk_size: usize,
    tier_filter: Option<&RiskTier>,
) -> Vec<u8> {
    // Initialize metadata context
    let metadata_ctx = StreamingMetadataContext::new(tier_filter);
    let lookahead_size = metadata_ctx.lookahead_size();
    
    let mut output = Vec::new();
    let mut lookahead_buffer = Vec::new();
    
    for chunk in input.chunks(chunk_size) {
        // Combine with lookahead
        let search_buffer = [&lookahead_buffer[..], chunk].concat();
        
        // Detect patterns in combined buffer
        let matches = detect_patterns_in_buffer(&search_buffer, &metadata_ctx);
        
        // Calculate safe-to-output
        let safe_to_output = if chunk.len() >= lookahead_size {
            lookahead_buffer.len() + chunk.len() - lookahead_size
        } else {
            lookahead_buffer.len()
        };
        
        // Redact and output
        let redacted = redact_matches(&search_buffer, matches);
        output.extend_from_slice(&redacted[0..safe_to_output]);
        
        // Update lookahead for next iteration
        lookahead_buffer = redacted[safe_to_output..].to_vec();
    }
    
    // Flush remaining lookahead
    output.extend_from_slice(&lookahead_buffer);
    
    output
}
```

---

## Configuration & Selection

### Tier Selection Configuration

**Pattern Selector** (from previous design work):

```rust
pub enum PatternSelector {
    /// All 274 patterns
    All,
    
    /// Specific tiers
    Tiers(Vec<RiskTier>),
    
    /// By pattern names (exact match)
    Patterns(Vec<String>),
    
    /// By tags (e.g., "aws-*", "github-*")
    Tags(Vec<String>),
    
    /// Wildcard (e.g., "aws-*")
    Wildcard(String),
    
    /// Regex matching
    Regex(String),
}

impl PatternSelector {
    pub fn matches(&self, metadata: &PatternMetadata) -> bool {
        match self {
            PatternSelector::All => true,
            
            PatternSelector::Tiers(tiers) => {
                tiers.iter().any(|t| t == &metadata.tier)
            },
            
            PatternSelector::Patterns(names) => {
                names.iter().any(|n| n == &metadata.name)
            },
            
            PatternSelector::Tags(tags) => {
                tags.iter().any(|tag| {
                    if tag.ends_with('*') {
                        // Prefix match: "aws-*" matches "aws-access-key", etc.
                        metadata.tags.iter().any(|t| t.starts_with(&tag[..tag.len()-1]))
                    } else {
                        // Exact match
                        metadata.tags.contains(tag)
                    }
                })
            },
            
            PatternSelector::Wildcard(pattern) => {
                // "aws-*" → matches patterns with tag starting with "aws-"
                // "github-*" → matches patterns with tag starting with "github-"
                metadata.tags.iter().any(|t| {
                    glob_match(pattern, t)
                })
            },
            
            PatternSelector::Regex(regex_pattern) => {
                let re = regex::Regex::new(regex_pattern).ok()?;
                re.is_match(&metadata.name)
            },
        }
    }
    
    pub fn get_matching_patterns(
        &self,
        cache: &MetadataCache,
    ) -> Vec<String> {
        cache.patterns_by_name
            .iter()
            .filter(|(_, metadata)| self.matches(metadata))
            .map(|(name, _)| name.clone())
            .collect()
    }
}
```

**Usage Example**:

```rust
// CLI: scred --detect tier1-critical,tier2-api-keys input.txt
let selector = PatternSelector::Tiers(vec![
    RiskTier::critical,
    RiskTier::api_keys,
]);

// MITM: detect all, redact critical only
let detect_selector = PatternSelector::All;
let redact_selector = PatternSelector::Tiers(vec![RiskTier::critical]);

// Env var: SCRED_PATTERNS="aws-*,github-*"
let selector = PatternSelector::Wildcard("aws-*".into());
let selector = PatternSelector::Wildcard("github-*".into());
```

---

## Implementation Plan

### Phase 1: Metadata Structure in Zig (30 minutes)
1. Add PatternMetadata struct to patterns.zig
2. Add RiskTier, PatternCategory, FFIPath enums
3. Populate all 274 pattern entries (use Task 2 classification)
4. Build and verify compilation

### Phase 2: FFI Bindings (30 minutes)
1. Create PatternMetadataFFI struct in lib.rs
2. Implement scred_pattern_get_metadata() function
3. Implement scred_pattern_get_by_index() function
4. Implement scred_pattern_count() function
5. Write FFI tests

### Phase 3: Runtime Cache (30 minutes)
1. Create metadata_cache.rs in scred-redactor
2. Implement MetadataCache struct with HashMap indices
3. Add METADATA_CACHE singleton with OnceLock
4. Implement get_pattern(), get_patterns_by_tier(), get_patterns_by_tag()
5. Write cache tests

### Phase 4: Pattern Selector (20 minutes)
1. Implement PatternSelector enum
2. Add matches(), get_matching_patterns() methods
3. Implement wildcard and regex matching
4. Write selector tests

### Phase 5: Integration (10 minutes)
1. Update redactor.rs to use metadata cache
2. Add tier filtering to redaction pipeline
3. Document configuration format
4. Create example configs

**Total Estimated Time**: 120 minutes (2 hours)

---

## Deliverables

✅ **TASK3_METADATA_DESIGN.md** (this document)
- Comprehensive architecture design
- Three-layer system (Zig source of truth → FFI bindings → Rust cache)
- Cross-component validation flow
- Streaming-aware design
- Configuration examples

⏳ **patterns.zig** (to be implemented Phase 1)
- PatternMetadata struct
- All 274 entries populated
- Risk tiers and categories

⏳ **lib.rs FFI functions** (to be implemented Phase 2)
- scred_pattern_get_metadata()
- scred_pattern_get_by_index()
- scred_pattern_count()
- Related functions

⏳ **metadata_cache.rs** (to be implemented Phase 3)
- MetadataCache struct
- METADATA_CACHE singleton
- Lookup methods

⏳ **pattern_selector.rs** (to be implemented Phase 4)
- PatternSelector enum
- Matching logic
- Configuration support

---

## Next Steps

1. **Execute Phase 1**: Add PatternMetadata struct to Zig patterns.zig
2. **Execute Phase 2**: Create FFI bindings
3. **Execute Phase 3**: Implement metadata cache
4. **Execute Phase 4**: Implement pattern selector
5. **Execute Phase 5**: Integrate with redactor

After Task 3 completion, Task 5 (Comprehensive Test Suite) can proceed with validated metadata support.

---

## Success Criteria

✅ All 274 patterns have metadata
✅ Metadata exposed via FFI
✅ Runtime cache with O(1) lookups
✅ Pattern selector supports all modes
✅ Streaming detection uses metadata lookahead
✅ Cross-component validation works
✅ Configuration format defined
✅ All tests passing
✅ Zero blockers identified
✅ Ready for Task 5

---

**Status**: Design COMPLETE - Ready for implementation
**Next**: Execute phases 1-5 (2 hours estimated)
