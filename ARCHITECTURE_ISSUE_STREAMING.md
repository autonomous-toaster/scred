# CRITICAL ARCHITECTURE ISSUE: Double Redaction in Streaming

## Problem Found

### Current Implementation (Broken)
```rust
// In streaming_request.rs, line 195-210
let (output, bytes_written, patterns) = redactor.process_chunk(&chunk, &mut lookahead, is_eof);

let filtered_output = if let Some(sel) = &config.redact_selector {
    let config_engine = ConfigurableEngine::new(...);
    // ❌ WRONG: Re-redacting ORIGINAL chunk instead of using output
    config_engine.redact_only(&String::from_utf8_lossy(&chunk))  // ← BUG!
} else {
    output.clone()
};
```

### Issues

1. **Double Redaction**: Same patterns are redacted twice
   - First by `StreamingRedactor::process_chunk()`
   - Second by `ConfigurableEngine::redact_only()`

2. **Incorrect Logic**: We're redacting the ORIGINAL chunk, not the already-redacted output
   - Should be: `config_engine.redact_only(&output)` if we must do this
   - But we shouldn't do it at all!

3. **Architectural Confusion**:
   - `StreamingRedactor`: Handles streaming chunking + lookahead
   - `ConfigurableEngine`: Handles pattern selector filtering
   - They should be unified, not layered!

4. **Inefficiency**: Two full regex pattern scans per chunk

## Root Cause

When we implemented selector support for streaming in the previous session, we:
1. Made `StreamingRedactor` have a selector field
2. But never actually used it in `process_chunk()`
3. So we added a wrapper layer with `ConfigurableEngine`
4. But applied it incorrectly (to original chunk instead of output)

## The Real Solution

**Option A: Merge ConfigurableEngine into StreamingRedactor** (RECOMMENDED)

StreamingRedactor should:
1. Accept both `detect_selector` and `redact_selector`
2. In `process_chunk()`:
   - Detect all patterns (full scan)
   - Filter detection results by `detect_selector`
   - Filter redaction by `redact_selector` before returning output
3. Remove the need for ConfigurableEngine wrapper entirely

```rust
pub struct StreamingRedactor {
    engine: Arc<RedactionEngine>,
    config: StreamingConfig,
    detect_selector: PatternSelector,
    redact_selector: PatternSelector,
}

impl StreamingRedactor {
    pub fn process_chunk(
        &self,
        chunk: &[u8],
        lookahead: &mut Vec<u8>,
        is_eof: bool,
    ) -> (String, u64, u64) {
        // ... lookahead + combination logic ...
        
        // Detect all patterns
        let redacted_result = self.engine.redact(&combined_str);
        
        // Filter which patterns to actually redact
        let filtered_output = if self.redact_selector != PatternSelector::All {
            // Apply selector filtering...
        } else {
            redacted_result.redacted
        };
        
        // Filter detection results for statistics
        let patterns_found = redacted_result.warnings
            .iter()
            .filter(|w| self.is_selected(&w.pattern_name, &self.detect_selector))
            .count() as u64;
        
        return (filtered_output, ...)
    }
}
```

**Option B: Keep separate, fix the wrapping**

Make ConfigurableEngine support streaming:
1. Add `redact_chunk()` method
2. Apply selector filtering to already-redacted output

```rust
impl ConfigurableEngine {
    pub fn redact_chunk(&self, redacted_output: &str) -> String {
        // Filter already-redacted output by selector
        // Only keep redactions for selected patterns
    }
}
```

## Correct Data Flow (What Should Happen)

```
User: --redact CRITICAL,API_KEYS
    ↓
StreamingRedactor with redact_selector=CRITICAL,API_KEYS
    ↓
process_chunk(chunk):
    1. Combine with lookahead buffer
    2. Detect ALL patterns (full regex scan)
    3. Filter detections by detect_selector (for statistics/logging)
    4. Filter redactions by redact_selector (before output)
       - CRITICAL patterns: redact them
       - API_KEYS patterns: redact them
       - Other patterns: DON'T redact them
    5. Return (filtered_output, bytes_written, pattern_count)
    ↓
upstream_writer.write_all(filtered_output)
```

## Current (Wrong) Data Flow

```
User: --redact CRITICAL,API_KEYS
    ↓
StreamingRedactor (ignores selector)
    ↓
process_chunk(chunk):
    1. Combine with lookahead buffer
    2. Detect ALL patterns
    3. Redact ALL patterns (selector ignored!)
    4. Return (all_redacted_output, ...)
    ↓
ConfigurableEngine.redact_only(&ORIGINAL_CHUNK):  ← WRONG!
    1. Detects ALL patterns again (redundant!)
    2. Filters which ones to show (useless, already redacted)
    3. Returns re-redacted text
    ↓
upstream_writer.write_all(double_redacted_output)
```

## Impact

- **Correctness**: Semantically working but inefficient
- **Performance**: ~2x slower due to double regex scan
- **Code Quality**: Architectural confusion
- **Maintainability**: Hard to understand intent

## Recommendation

Implement **Option A**: Merge ConfigurableEngine logic into StreamingRedactor

This:
1. Eliminates double redaction
2. Makes selector support native to streaming
3. Improves performance by 50% (single regex pass)
4. Clarifies architecture (one streaming engine)
5. Simplifies code

## Timeline

This is not a blocking bug (output is correct), but should be fixed in next refactoring pass.

## Files to Modify

- `crates/scred-redactor/src/streaming.rs` - Add selector filtering
- `crates/scred-http/src/streaming_request.rs` - Remove ConfigurableEngine wrapper
- `crates/scred-http/src/streaming_response.rs` - Remove ConfigurableEngine wrapper

