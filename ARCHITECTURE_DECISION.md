# SCRED Architecture Decision - Selector Filtering in Streaming

## Status: DOCUMENTED LIMITATION

### Current State

**Selector fields exist in StreamingRedactor but are NOT used in process_chunk()**

```rust
pub struct StreamingRedactor {
    engine: Arc<RedactionEngine>,
    config: StreamingConfig,
    selector: Option<PatternSelector>,  // ← EXISTS but UNUSED
}

pub fn with_selector(..., selector: PatternSelector) -> Self {
    Self {
        engine,
        config,
        selector: Some(selector),  // ← Can set it
    }
}

pub fn process_chunk(...) -> (String, u64, u64) {
    // ... selector field is IGNORED
    // Always redacts ALL patterns regardless of selector
}
```

### Why Selective Filtering is Complex

Streaming redaction doesn't support selective un-redaction because:

1. **Lookahead buffer complicates position tracking**
   - Pattern positions shift when lookahead is involved
   - Original offset ≠ redacted offset
   - Need bidirectional mapping (original↔redacted)

2. **Cannot un-redact partially redacted data**
   - RedactionEngine returns fully redacted text (all x's)
   - To selectively redact, need to know WHERE each pattern is
   - Then un-redact only patterns NOT in selector
   - Requires character-by-character position tracking

3. **Streaming chunking prevents full context**
   - Pattern might span chunk boundaries
   - Lookahead buffer catches spanning patterns
   - But position mapping becomes complex

### Why CLI is Different

CLI supports selective redaction because:

1. **Input is buffered (not streamed)**
   - Entire input available at once
   - No lookahead complications
   - Position tracking straightforward

2. **Uses ConfigurableEngine for selective un-redaction**
   - Detects all patterns
   - Un-redacts patterns NOT in selector
   - Possible because input is complete

3. **Performance acceptable**
   - CLI processes one-shot inputs
   - Buffering is OK for small-to-medium files
   - Streaming not needed for CLI

### Solution Chosen: Conservative Approach

**Streaming always redacts ALL patterns (no selector filtering)**

Rationale:
- Simple, reliable, performant
- Safer (redact more rather than less)
- Positions tracking through lookahead is complex
- Selector filtering applies to detection/logging, not streaming bodies

Trade-off:
- Users can't control streaming redaction
- But streaming preserves security (redacts conservatively)
- CLI still supports selective redaction for non-streaming input

### How Each Tool Works

#### CLI (scred)
```
User: --redact CRITICAL,API_KEYS
      ↓
Small input (<100MB):
  - Buffers in memory
  - Uses ConfigurableEngine
  - Selective un-redaction ✓
  
Large input (>100MB):
  - Streams with StreamingRedactor
  - Redacts ALL patterns
  - Selector ignored (conservative)
```

#### MITM (scred-mitm)
```
HTTP Request/Response:
  - Always streams (bodies can be >100MB)
  - Uses StreamingRedactor
  - Redacts ALL patterns
  - Selector not supported
  
Headers:
  - Buffered (usually <64KB)
  - Uses StreamingRedactor::redact_buffer()
  - Redacts ALL patterns
```

#### Proxy (scred-proxy)
```
HTTP Request/Response:
  - Always streams
  - Uses StreamingRedactor
  - Redacts ALL patterns
  - Selector not supported
```

### Implementation Path (Future)

To support selective filtering in streaming:

#### Step 1: Track Pattern Positions
```rust
pub struct PatternMatch {
    pattern_name: String,
    start_byte: usize,
    end_byte: usize,
    is_selected: bool,
}
```

#### Step 2: Selective Redaction Algorithm
```rust
fn apply_selective_redaction(
    &self,
    original: &str,
    fully_redacted: &str,
    patterns: &[PatternMatch],
) -> String {
    let mut output = fully_redacted.to_string();
    
    // Un-redact patterns NOT in selector
    for pattern in patterns {
        if !pattern.is_selected {
            // Copy original bytes for this pattern
            output[pattern.start_byte..pattern.end_byte] =
                original[pattern.start_byte..pattern.end_byte];
        }
    }
    
    output
}
```

#### Step 3: Handle Lookahead Positions
- Map lookahead positions correctly
- Adjust pattern offsets for lookahead buffer
- Validate position consistency

#### Step 4: Test Thoroughly
- Spanning patterns across chunks
- Lookahead boundary conditions
- Character preservation verification

**Estimated Effort**: 3-4 hours

### Documentation for Users

Add to README:

```markdown
## Selector Support

SCRED supports pattern selectors via `--redact` and `--detect` flags.

### Supported Modes

| Tool | Input Type | Selector Support |
|------|-----------|-----------------|
| scred (CLI) | Buffered | ✓ Full support |
| scred (CLI) | Streaming | Limited (redacts all) |
| scred-mitm | Always streaming | Limited (redacts all) |
| scred-proxy | Always streaming | Limited (redacts all) |

### Usage

```bash
# Selective redaction (CLI, buffered input)
scred --redact CRITICAL,API_KEYS < small_input.txt

# All patterns redacted (streaming)
cat large_file.json | scred

# MITM always redacts all patterns
scred-mitm  # Listens on port 8888

# Proxy always redacts all patterns
scred-proxy # Listens on port 9999
```

### When to Use Selectors

- Use selectors for CLI with small/medium files
- For large files or streaming, all patterns redacted (conservative)
- MITM and Proxy always redact conservatively
```

### Conclusion

Current approach is:
- **✅ Safe**: Redacts conservatively
- **✅ Simple**: No position tracking complexity
- **✅ Performant**: Single regex pass per chunk
- **⚠️ Limited**: Streaming doesn't support selective redaction
- **✅ Documented**: Clear about limitation

Future improvement (not urgent):
- Implement selective filtering in streaming (3-4 hour task)
- Would require careful position tracking
- Worth doing when performance/features justify effort

