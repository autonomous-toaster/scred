# P0#1 Implementation: Selector-Aware Streaming Redactor

## Problem
- StreamingRedactor doesn't support selectors
- stream_request_to_upstream() uses Arc<StreamingRedactor>
- Can't pass ConfigurableEngine directly

## Solution: Create SelectiveStreamingRedactor Wrapper

```rust
struct SelectiveStreamingRedactor {
    config_engine: Arc<ConfigurableEngine>,
}

impl SelectiveStreamingRedactor {
    fn redact_buffer(&self, data: &[u8]) -> (String, StreamingStats) {
        // Use ConfigurableEngine which respects selectors
        let text = String::from_utf8_lossy(data).to_string();
        let redacted = self.config_engine.redact_only(&text);
        (redacted, StreamingStats::default()) // Simplified stats
    }
    
    fn process_chunk(&self, chunk: &[u8], lookahead: &mut Vec<u8>, is_eof: bool) 
        -> (Vec<u8>, usize, Vec<PatternMatch>) 
    {
        // More complex - need to implement streaming logic for ConfigurableEngine
        // OR defer to StreamingRedactor but filter results
    }
}
```

## Issue: process_chunk is Complex
The process_chunk() method has lookahead buffer for patterns that span chunk boundaries.
This is the hard part to implement.

## Simpler Approach: Composition + Delegation

```rust
struct SelectiveStreamingRedactor {
    inner: Arc<StreamingRedactor>,
    config_engine: Arc<ConfigurableEngine>,
}

impl SelectiveStreamingRedactor {
    fn redact_buffer(&self, data: &[u8]) -> (String, StreamingStats) {
        // First redact with inner (gets all patterns)
        let (redacted, stats) = self.inner.redact_buffer(data);
        
        // Then apply selector filter
        // But wait - this only removes redactions, doesn't prevent detection
        // This won't work!
    }
}
```

## Best Approach: Replace streaming functions with selector-aware versions

Create new functions in scred-http that work like stream_request_to_upstream but use ConfigurableEngine:

```rust
pub async fn stream_request_with_selector<R, W>(
    client_reader: &mut BufReader<R>,
    mut upstream_writer: W,
    request_line: &str,
    config_engine: Arc<ConfigurableEngine>,
    config: StreamingRequestConfig,
) -> Result<StreamingStats>
{
    // Similar logic to stream_request_to_upstream but uses config_engine
    // For simplicity, can read entire request first, apply selector, then send
}
```

## Practical Implementation Path

### Step 1: Create NonStreamingRedaction functions
```rust
// In handle_connection()
// Instead of using stream_request_to_upstream with Arc<StreamingRedactor>
// Use inline redaction with ConfigurableEngine

let request_body = read_all_request_body(&mut client_reader)?;
let redacted_body = config_engine.redact_only(&request_body);
upstream.write_all(redacted_body.as_bytes()).await?;
```

### Step 2: Accept temporary performance tradeoff
- Buffer entire request/response
- Apply selector-aware redaction all at once
- Trade: Memory vs Correctness

### Step 3: Future optimization
- Implement proper streaming with selector support
- Create streaming version of ConfigurableEngine if needed

## Decision
Use **Practical Implementation Path** - inline buffered redaction with ConfigurableEngine.
This:
- ✅ Fixes P0#1 (selectors now work)
- ✅ Simple to implement (1-2 hours)
- ✅ Unblocks P0#4, P0#5
- ⏳ Trades streaming efficiency for correctness (acceptable for now)

