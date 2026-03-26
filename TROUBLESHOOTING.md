# SCRED Troubleshooting & FAQ

## Common Issues

### Installation Issues

#### Problem: `cargo install scred` fails with "unknown feature"

**Cause**: Trying to use SIMD feature with stable Rust

**Solution**:
```bash
# Use nightly for SIMD feature
cargo +nightly install scred --features simd-accel

# Or use stable without SIMD
cargo install scred  # default, works everywhere
```

#### Problem: Binary not found after installation

**Cause**: Cargo bin directory not in PATH

**Solution**:
```bash
# Check installation location
~/.cargo/bin/scred --help

# Add to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.cargo/bin:$PATH"
```

#### Problem: `rustc` version too old

**Cause**: Rust 1.70 or newer required

**Solution**:
```bash
# Update Rust
rustup update

# Check version
rustc --version  # should be 1.70+
```

---

### Runtime Issues

#### Problem: Hangs on stdin

**Cause**: Waiting for EOF

**Solution**:
```bash
# Use input redirection
scred < input.txt

# Use pipe
echo "text" | scred

# Or provide filename
scred input.txt
```

#### Problem: Output is binary/corrupted

**Cause**: Binary mode activated

**Solution**:
```bash
# Ensure text mode
scred --mode streaming < input.txt

# Check file encoding
file input.txt  # should be ASCII/UTF-8
```

#### Problem: High memory usage

**Cause**: Not using streaming mode

**Solution**:
```bash
# Use streaming for large files
scred --mode streaming < large_file.txt > output.txt

# Streaming uses <64KB typical
```

#### Problem: Slow performance

**Cause**: Large input without optimization

**Solution**:
```bash
# Use SIMD if available
cargo +nightly build --release --features simd-accel
./target/release/scred input.txt

# Or process in chunks
split -l 1000 large_file.txt chunk_
for f in chunk_*; do scred "$f" >> output.txt; done
```

---

### Detection Issues

#### Problem: Secret not detected

**Cause**: Pattern not implemented or false negative

**Solution**:
```bash
# Check available patterns
scred --list-patterns | grep -i "pattern_name"

# Check pattern details
scred --pattern-info "pattern_name"

# Verify with known secret
echo "AKIAIOSFODNN7EXAMPLE" | scred  # should redact
```

#### Problem: Too many false positives

**Cause**: Pattern too broad

**Solution**:
```bash
# Check what's being matched
scred --mode detect input.txt | head -20

# Report false positives
# Issue: https://github.com/your-org/scred/issues
```

#### Problem: Environment variable not redacted

**Cause**: Pattern might be missing `=` character

**Solution**:
```bash
# Check pattern format
scred --pattern-info "env-pattern-name"

# Test with full format
echo "MY_SECRET=secret_value" | scred
```

---

### Performance Issues

#### Problem: SIMD slower than scalar

**Cause**: Normal noise at small sizes, or compiler issue

**Solution**:
```bash
# Test with larger payload (>10KB)
python3 perf_regression_test.py

# Check build flags
cargo +nightly build --release --features simd-accel -v

# Ensure optimizations enabled
# Check: -C opt-level=3
```

#### Problem: Memory leaks in streaming

**Cause**: Buffer not releasing properly

**Solution**:
```bash
# Use memory profiler
valgrind --leak-check=full scred input.txt

# Or use Rust analyzer
cargo build --release --features simd-accel
```

---

## FAQ

### General Questions

**Q: Is SCRED safe for production use?**
A: Yes, v0.2.0 is production-ready. 346 tests passing, zero known security issues.

**Q: What's the difference between scalar and SIMD?**
A: SIMD uses CPU vector instructions for 0.4-2.1% faster detection. Scalar uses standard code. Both are safe; SIMD requires nightly Rust.

**Q: Why is macro improvement (0.4-2.1%) less than micro (29-48%)?**
A: Only 22% of patterns use charset scanning (which SIMD helps). 78% use regex (no SIMD benefit). This is expected and well-analyzed.

**Q: Can I use SCRED in a library?**
A: Yes, import `scred_detector::detect_all()`:
```rust
use scred_detector::detect_all;

let matches = detect_all(text.as_bytes());
```

**Q: Does SCRED support regex patterns?**
A: Yes, 24 patterns use regex. But SIMD optimization only helps charset scanning.

**Q: Is SCRED open source?**
A: Yes, see [LICENSE](LICENSE).

---

### Performance Questions

**Q: How fast is SCRED?**
A: Typically <100µs for detection, <100ms for 1MB redaction. See [SIMD_IMPLEMENTATION.md](SIMD_IMPLEMENTATION.md) for benchmarks.

**Q: Should I use SIMD in production?**
A: If processing 1MB+ payloads where 0.4-2.1% matters, yes. Otherwise, stable is fine.

**Q: How much memory does streaming use?**
A: Typically <64KB for most patterns. Max is ~20KB per pattern lookahead.

**Q: Can I make detection faster?**
A: Use SIMD (0.4-2.1% faster), or process in parallel (batch multiple files).

**Q: Why is the first run slower?**
A: Binary loading and compilation overhead. Subsequent runs use same binary.

---

### Pattern Questions

**Q: How many patterns does SCRED support?**
A: 273+ patterns covering 252+ providers (AWS, GitHub, Stripe, etc.)

**Q: Can I add custom patterns?**
A: Currently no. File an issue for pattern requests.

**Q: Why is pattern X not detected?**
A: Might not be implemented. Check: `scred --list-patterns | grep -i pattern`

**Q: Are false positives an issue?**
A: All patterns have bounded `min_len` and `max_len` to reduce false positives.

**Q: Can I whitelist/blacklist patterns?**
A: Not currently. Feature request welcome.

---

### Character Preservation Questions

**Q: Why are the first 4 characters visible after redaction?**
A: For context. Shows you which provider it was without exposing the actual secret.

**Q: Can I change the redaction character (x)?**
A: Currently hardcoded to 'x'. Feature request welcome.

**Q: Does length change after redaction?**
A: No, output is same length as input. This preserves structure.

**Q: What about multi-line secrets?**
A: SSH keys and PGP blocks are fully redacted (not prefix-preserved).

---

### Integration Questions

**Q: Can I use SCRED as a proxy?**
A: Yes, see [SIMD_DEPLOYMENT.md](SIMD_DEPLOYMENT.md) - TLS MITM proxy support.

**Q: Can SCRED integrate with CI/CD?**
A: Yes, it's designed for this. Run in pipeline to scan commits.

**Q: Does SCRED support environment variables?**
A: Yes, detects patterns like `API_KEY=secret`.

**Q: Can SCRED redact HTTP requests?**
A: Yes, see HTTP proxy mode in [SIMD_DEPLOYMENT.md](SIMD_DEPLOYMENT.md).

---

### Troubleshooting Deep Dive

#### Debug Logging

```bash
# Build with logging
RUST_LOG=scred=debug cargo run -- --mode streaming < input.txt

# Enable detailed output
RUST_BACKTRACE=1 scred input.txt
```

#### Test with Known Secrets

```bash
# Test all major secret types
cat << 'EOF' | scred
AWS: AKIAIOSFODNN7EXAMPLE
GitHub: ghp_1234567890abcdefghijklmnopqrstuvwxyz
Stripe: sk_live_1234567890abcdefghijklmnopqrstuvwxyz
OpenAI: sk-proj-1234567890abcdefghijklmnopqrstuvwxyz
EOF
```

#### Build Verification

```bash
# Check build details
cargo build --release -v 2>&1 | grep -i "simd\|opt"

# Verify binary
file target/release/scred
ldd target/release/scred | grep -i simd
```

---

## Getting Help

### Resources

1. **Documentation**:
   - [SIMD_IMPLEMENTATION.md](SIMD_IMPLEMENTATION.md) - Technical details
   - [SIMD_DEPLOYMENT.md](SIMD_DEPLOYMENT.md) - Deployment guide
   - [README.md](README.md) - Quick start

2. **Testing**:
   - `cargo test --lib` - Run all tests
   - `python3 perf_regression_test.py` - Check performance

3. **Community**:
   - [GitHub Issues](https://github.com/your-org/scred/issues)
   - Check existing issues first
   - Provide minimal reproducible example

### Reporting Bugs

Include:
1. **Version**: `scred --version`
2. **Platform**: `uname -a`
3. **Rust version**: `rustc --version`
4. **Minimal example** that reproduces issue
5. **Expected vs actual output**

Example:
```
Version: 0.2.0
Platform: macOS 13.1 arm64
Rust: 1.75.0 (stable)

Issue: Secret X not detected
Code:
  echo "AKIAIOSFODNN7EXAMPLE" | scred

Expected: Shows redacted output
Actual: No output
```

---

## Advanced Troubleshooting

### Building from Source

```bash
# Clone with full history
git clone --depth 1 https://github.com/your-org/scred.git
cd scred

# Build with debug info
cargo build --release --debug-info

# Run with backtrace
RUST_BACKTRACE=full cargo run -- input.txt
```

### Performance Profiling

```bash
# Install profiling tools
cargo install cargo-flamegraph

# Generate flame graph
cargo flamegraph --release -o my_graph.svg -- input.txt

# View graph
open my_graph.svg
```

### Memory Profiling

```bash
# Use Valgrind (Linux)
valgrind --tool=massif --massif-out-file=massif.out scred large_file.txt
ms_print massif.out | head -40

# Use Instruments (macOS)
instruments -t 'Allocations' -p $$ ./target/release/scred input.txt
```

---

## Prevention Tips

### Best Practices

1. **Always test with known secrets**:
   ```bash
   echo "AKIAIOSFODNN7EXAMPLE" | scred
   ```

2. **Use streaming for large files**:
   ```bash
   scred --mode streaming < large_file.txt
   ```

3. **Verify output**:
   ```bash
   scred input.txt | head -10  # Check sample
   ```

4. **Keep SCRED updated**:
   ```bash
   cargo install --force scred  # Get latest
   ```

5. **Monitor performance**:
   ```bash
   python3 perf_regression_test.py
   ```

---

## Still Having Issues?

1. Check this FAQ and [documentation](README.md)
2. Search [GitHub Issues](https://github.com/your-org/scred/issues)
3. Create new issue with minimal reproducible example
4. Include all relevant diagnostic info

We're here to help! 🙌
