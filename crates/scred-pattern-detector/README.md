# SCRED Pattern Detector - Zig 0.16.0

Ultra-high-performance pattern detector for 244+ secret patterns using Trie-based matching.

## Architecture

### Phase 1: Foundation (this phase)
- [x] Zig 0.16.0 project setup with C FFI
- [x] Streaming buffer management (64KB + 512B lookahead)
- [x] Character-preserving output
- [x] Rust FFI wrapper (build system integration)
- [x] Basic benchmark harness
- [ ] Build system verification (zig build-lib)

### Phase 2: Trie-Based Pattern Matching (Next)
- [ ] Implement Aho-Corasick algorithm for multi-pattern matching
- [ ] Build prefix trie for fast lookups (AKIA, ghp_, sk-, etc)
- [ ] SIMD acceleration for binary scanning
- [ ] Support all 244+ patterns from scred-redactor

### Phase 3: Performance Optimization (Week 2)
- [ ] Regex fallback for complex patterns
- [ ] Pattern caching and lazy compilation
- [ ] Memory profiling and optimization
- [ ] Benchmark: Target 50+ MB/s

### Phase 4: Integration & Testing (Week 3)
- [ ] Integrate into scred-redactor
- [ ] Streaming correctness tests
- [ ] End-to-end performance benchmarks
- [ ] Production release

## Building

```bash
# Build Zig library
zig build -Doptimize=ReleaseFast

# Build with Rust
cargo build -p scred-pattern-detector-zig

# Run tests
cargo test -p scred-pattern-detector-zig

# Benchmark (Zig)
cd crates/scred-pattern-detector-zig && zig build bench
```

## Performance Targets

- **Baseline** (current regex): 6.2 MB/s
- **Phase 1 Goal**: 20+ MB/s (3x improvement, with basic trie)
- **Phase 2 Goal**: 50+ MB/s (8x improvement, full Aho-Corasick)
- **Phase 3 Goal**: 80+ MB/s (13x improvement, with SIMD)
- **Stretch**: 100+ MB/s (state machine optimization)

## Key Design Decisions

1. **Trie-Based Matching**: O(n + z) complexity (n=text, z=matches)
2. **Streaming API**: process_chunk(input, lookahead, is_eof) → (output, matches)
3. **C FFI**: Direct Zig-to-Rust interop (no intermediate layer)
4. **Character Preservation**: Output length always = input length
5. **Bounded Memory**: 64KB + 512B lookahead, independent of input size

## Files

```
crates/scred-pattern-detector-zig/
├── build.zig                    # Zig build config
├── build.rs                     # Rust build script (zig compile)
├── Cargo.toml                   # Rust workspace member
├── src/
│   ├── lib.zig                  # Main Zig implementation
│   ├── lib.rs                   # Rust FFI wrapper
│   └── benchmark.zig            # Performance harness
```

## Status

**Current**: Phase 1 - Project initialized with FFI bridge
**Next**: Implement Aho-Corasick pattern matching in Zig

## References

- Zig Documentation: https://ziglang.org/documentation/0.16.0/
- Aho-Corasick Algorithm: https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm
- Performance: https://github.com/autonomous-toaster/scred/wiki/Pattern-Detector-Zig
