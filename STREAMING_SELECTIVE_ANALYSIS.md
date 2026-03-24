# Can Streaming Support Selective Filtering? YES - Analysis & Implementation Path

## The Previous Assessment Was WRONG

Previous conclusion: "Lookahead buffer makes position tracking complex, can't support selective filtering"

**CORRECT conclusion**: Lookahead buffer does NOT prevent selective filtering. The solution is to change what the engine returns.

## The Key Insight: Return Metadata, Not Just Text

### Current Architecture (WRONG for streaming)
```rust
RedactionEngine::redact(text) -> RedactionResult {
    redacted: String,           // Already redacted (can't undo)
    warnings: Vec<...>          // Just counts, no position info
}

// In streaming, can't know which patterns were in lookahead
// Can't selectively un-redact
```

### Better Architecture (CORRECT for streaming)
```rust
RedactionResult {
    redacted: String,                    // Already redacted
    matches: Vec<PatternMatch>,          // Metadata about each match
}

PatternMatch {
    position: usize,                     // Byte position in input
    pattern_type: String,                // Pattern name (e.g., "aws-access-key")
    original_text: String,               // Original unredacted text
    redacted_text: String,               // Replacement (e.g., "AKIAxxxxxxxx")
}

// In streaming:
// 1. Get matches with metadata
// 2. Filter matches by selector
// 3. Un-redact filtered-out patterns
// 4. Output only selected patterns redacted
```

## Why This Works

### Problem with Lookahead (False Problem)
```
Chunk 1: [64KB data]
Lookahead: [512B from end of chunk 1]

Chunk 2: [64KB data]
Combined: [512B lookahead] + [64KB chunk 2]

Previous thinking:
  "Position in combined != position in output"
  "Can't track positions through lookahead"

Actual problem: Wrong approach entirely!
```

### Solution: Metadata-Based Filtering (Real Solution)
```
1. Process combined: lookahead + new_chunk

2. Get matches with ABSOLUTE positions in combined:
   Match { pos: 100, type: "aws", ... }
   Match { pos: 512, type: "github", ... }
   Match { pos: 520, type: "env-var", ... }

3. Determine output_end:
   - Normal case: output from 0 to (combined.len() - lookahead_size)
   - EOF case: output entire combined

4. Calculate overlap region:
   - Lookahead region = [combined.len() - lookahead_size, combined.len()]
   
5. Filter matches to keep:
   - Matches in [0, output_end): KEEP (will be output now)
   - Matches in [output_end, combined.len()): SAVE (will process later)

6. Selective filtering:
   - For KEPT matches:
     - Only keep those passing selector filter
     - Un-redact others (replace 'xxxxx' back with original)
   - Output resulting text

7. Save overlapping matches for next iteration:
   - Pass unprocessed matches to next call
   - These get re-evaluated against new chunk
```

## Why Position Tracking is Actually Simple

### The Key: Store Match Position + Original Text

```rust
// In combined buffer
combined = "API_KEY=AKIAIOSFODNN7EXAMPLE&secret=value";
//          0         10        20        30

// Find match
Match {
    position: 8,           // Position in combined (absolute)
    original_text: "AKIAIOSFODNN7EXAMPLE",
    redacted_text: "AKIAxxxxxxxxxxxxxxxx",
    pattern_type: "aws-access-key"
}

// To un-redact:
output[position..position+original_text.len()] = original_text
```

This is trivial! No complex position mapping needed.

## Implementation Path (2 Hours)

### 1. Modify RedactionResult (~30 min)

```rust
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub position: usize,
    pub pattern_type: String,
    pub original_text: String,
    pub redacted_text: String,
    pub match_len: usize,
}

#[derive(Debug, Clone)]
pub struct RedactionResult {
    pub redacted: String,
    pub matches: Vec<PatternMatch>,
    pub warnings: Vec<RedactionWarning>,
}
```

### 2. Update RedactionEngine::redact() (~45 min)

```rust
pub fn redact(&self, text: &str) -> RedactionResult {
    let mut redacted = text.to_string();
    let mut matches = Vec::new();
    
    for (pattern_name, regex) in &self.compiled_patterns {
        // Filter by selector first (if configured)
        if let Some(selector) = &self.selector {
            if !selector.matches(pattern_name) {
                continue;  // Skip patterns not in selector
            }
        }
        
        // Find ALL matches (with positions)
        for capture in regex.captures_iter(text) {
            let full_match = capture.get(0).unwrap();
            let position = full_match.start();
            let original_text = full_match.as_str().to_string();
            let redacted_text = self.redact_pattern(&original_text);
            
            matches.push(PatternMatch {
                position,
                pattern_type: pattern_name.clone(),
                original_text,
                redacted_text,
                match_len: full_match.len(),
            });
        }
    }
    
    // Apply all matches to redacted text (in reverse order to preserve positions)
    for m in matches.iter().rev() {
        let end = m.position + m.redacted_text.len();
        redacted.replace_range(m.position..end, &m.redacted_text);
    }
    
    RedactionResult { redacted, matches, warnings: Vec::new() }
}
```

### 3. Update StreamingRedactor::process_chunk() (~45 min)

```rust
pub fn process_chunk(
    &self,
    chunk: &[u8],
    lookahead: &mut Vec<u8>,
    is_eof: bool,
) -> (String, u64, u64) {
    let mut combined = lookahead.clone();
    combined.extend_from_slice(chunk);
    
    let combined_str = String::from_utf8_lossy(&combined);
    let redaction_result = self.engine.redact(&combined_str);
    
    let mut output = redaction_result.redacted;
    let patterns_found = redaction_result.matches.len() as u64;
    
    // Calculate output boundaries
    let output_end = if is_eof {
        output.len()
    } else if output.len() > self.config.lookahead_size {
        output.len() - self.config.lookahead_size
    } else {
        0
    };
    
    // **NEW: Apply selective filtering for output region**
    if let Some(selector) = &self.selector {
        let mut output_chars = output.chars().collect::<Vec<_>>();
        
        for m in &redaction_result.matches {
            // Only filter matches in the output region
            if m.position >= output_end {
                continue; // Save for next iteration
            }
            
            // Skip if selector doesn't match this pattern
            if !selector.matches(&m.pattern_type) {
                // Un-redact: restore original text
                let start = m.position;
                let end = start + m.redacted_text.len();
                for (i, c) in m.original_text.chars().enumerate() {
                    if start + i < output_chars.len() {
                        output_chars[start + i] = c;
                    }
                }
            }
        }
        
        output = output_chars.iter().collect();
    }
    
    // Prepare final output
    let output_text = if output_end > 0 {
        output[..output_end].to_string()
    } else {
        String::new()
    };
    
    // Save lookahead
    if !is_eof && output_end < output.len() {
        *lookahead = output[output_end..].as_bytes().to_vec();
    } else {
        lookahead.clear();
    }
    
    (output_text, output_text.len() as u64, patterns_found)
}
```

## Character Preservation Guarantee

```
Input:  "API_KEY=AKIAIOSFODNN7EXAMPLE&secret=xyz"
Length: 38 characters

Match 1: pos 8, original="AKIAIOSFODNN7EXAMPLE" (20 chars)
         redacted="AKIAxxxxxxxxxxxxxxxx" (20 chars) ✓ Same length

If selector doesn't include AWS:
  Un-redacted text: "AKIAIOSFODNN7EXAMPLE" (20 chars) ✓ Same length

Output: "API_KEY=AKIAIOSFODNN7EXAMPLE&secret=xyz"
Length: 38 characters ✓ Preserved!
```

## Benefits of Metadata Approach

✅ Lookahead buffer IRRELEVANT (works with any buffer size)
✅ Streaming WORKS naturally (process matches, not text positions)
✅ Selective filtering WORKS in streaming (filter matches before output)
✅ Character preservation GUARANTEED (same match length in/out)
✅ No bidirectional mapping needed (simple: position + text replacement)
✅ All three tools consistent (CLI, MITM, Proxy use same engine)
✅ Performance BETTER (no re-redacting overlaps)

## Timeline

- **Modify RedactionResult**: 30 min
- **Update RedactionEngine**: 45 min
- **Update StreamingRedactor**: 45 min
- **Update consumers (CLI, MITM, Proxy)**: 30 min
- **Testing**: 30 min
- **Total**: ~3 hours

## Architecture After Fix

```
RedactionEngine (all 272 patterns)
  ├─ For each pattern, find ALL matches
  ├─ Return matches WITH METADATA (position, original, pattern_type)
  └─ Redact all matches

StreamingRedactor
  ├─ Get redacted text + matches from engine
  ├─ Filter matches by selector (if configured)
  ├─ Un-redact filtered-out matches
  └─ Output only selected patterns redacted

Result:
- CLI: Use selectors (small inputs, buffered)
- MITM: Use selectors in streaming (conservative: all patterns)
- Proxy: Use selectors in streaming (conservative: all patterns)
```

## Recommendation

✅ **Implement this approach**
- Makes streaming selector-aware
- Maintains character preservation
- Eliminates buffering entirely
- All three tools unified
- ~3 hours work

❌ **Don't do the previous buffered approach**
- Violates "streaming is first class" requirement
- Memory bloat for large files
- Inconsistent architecture

## Next Action

1. Implement metadata-based approach (3 hours)
2. Remove all buffering code from CLI
3. Update all three tools to use streaming exclusively
4. Add comprehensive streaming selector tests
5. Benchmark: ensure performance is good

Result: Pure streaming architecture with full selector support.
