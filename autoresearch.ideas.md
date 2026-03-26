# SCRED Performance Optimization Ideas

## Current Baseline
- Speed: 27ms for 662KB (24.5 MB/s throughput)
- Correctness: 100% secrets detected, character preservation verified
- Status: Highly optimized for production use

## Future Optimization Opportunities

### 1. SIMD Charset Scanning (High Impact)
- Current: Loop byte-by-byte with LUT lookup
- Opportunity: Use SIMD to process 16-32 bytes at once
- Potential: 15-25% faster pattern scanning
- Complexity: Requires platform-specific SIMD (portable_simd crate)

### 2. Pattern Deduplication
- Current: Search all patterns sequentially
- Opportunity: Group patterns by first byte, use trie for prefix matching
- Potential: 20-30% faster multi-pattern detection
- Tradeoff: More memory for pattern tree structure

### 3. Parallel Chunk Processing
- Current: Single-threaded streaming
- Opportunity: Process 64KB chunks in parallel threads
- Potential: 2-4x speedup on multi-core systems
- Tradeoff: Requires synchronization overhead, may not help for small files

### 4. Regex Caching
- Current: Patterns are regex-free (good!)
- Note: Keep this way - avoid regex overhead completely

### 5. Allocate Reduction in redact_text()
- Current: Clone entire input, modify bytes
- Opportunity: Use Copy-on-Write or streaming redaction
- Potential: 10-15% improvement
- Note: Character preservation requires same output length

### 6. ConfigurableEngine Fast Path
- Current: All patterns go through filtering
- Opportunity: Detect common cases (all patterns redacted) and skip filtering
- Potential: 5-10% improvement
- Status: Tried this but caused slowdown - needs careful implementation

### 7. Bounded Lookahead for SSH Keys
- Current: Up to 10KB lookahead for multiline patterns
- Opportunity: Reduce to 3-5KB for typical cases
- Potential: 5-10% improvement for mixed workloads
- Note: Must preserve correctness (full key detection)

## Measurement Notes
- High variance on first run (cache warming)
- Use runs 3+ for reliable measurements
- macOS time command has precision issues - use Rust's nanosecond timer
- Current throughput is 24.5 MB/s which is excellent for pattern matching

## Correctness Requirements (MUST MAINTAIN)
- 100% true positive rate: All secrets must be detected
- 0% false positive rate: No innocent text redacted
- Character preservation: Output length == input length
- 100% test pass rate on 245 pattern tests

## Performance Target
- Current: 27ms for 662KB → ACHIEVED
- Ideal: <15ms for 662KB → Would need SIMD improvements
- Acceptable: <30ms → Currently met
