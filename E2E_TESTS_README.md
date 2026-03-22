# E2E MITM Proxy Regression Tests

Comprehensive end-to-end test suite for detecting regressions in the SCRED MITM proxy.

## What's Tested

These tests verify real HTTP/2 traffic through the MITM proxy to httpbin.org:

| Test | Purpose | Expected Result |
|------|---------|-----------------|
| `e2e_http1_basic` | Basic HTTP/1.1 request | ✅ 200 OK response |
| `e2e_http2_alpn` | HTTP/2 ALPN negotiation | ✅ Either H2 or downgrade to H1 |
| `e2e_secret_in_query` | Secret redaction | ✅ Secret not in response |
| `e2e_sequential_requests` | Connection stability | ✅ All 3 requests succeed |
| `e2e_keep_alive` | Connection reuse | ✅ Keep-Alive working |
| `e2e_post_request` | POST with JSON body | ✅ Request succeeds |
| `e2e_error_handling` | Graceful failure | ✅ Proxy doesn't crash |
| `e2e_proxy_startup` | Proxy readiness | ✅ Listening on port |
| `e2e_large_response` | Large payload (10KB) | ✅ Handled correctly |
| `e2e_compressed_response` | Gzip compression | ✅ Decompressed correctly |

## Running Tests

### All Tests
```bash
cd /Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred-http2

# Quick run
./run-e2e-tests.sh

# Or using cargo directly
cargo test --test e2e_httpbin --ignored --release -- --nocapture
```

### Single Test
```bash
./run-e2e-tests.sh e2e_http1_basic
# or
cargo test --test e2e_httpbin e2e_http1_basic --ignored --release -- --nocapture
```

### Available Test Names
```
e2e_http1_basic
e2e_http2_alpn
e2e_secret_in_query
e2e_sequential_requests
e2e_keep_alive
e2e_post_request
e2e_error_handling
e2e_proxy_startup
e2e_large_response
e2e_compressed_response
```

## How They Work

1. **Start MITM Proxy**: Each test starts a fresh instance on a random port
2. **Make curl Request**: Uses curl through the proxy to httpbin.org
3. **Verify Result**: Checks status code, response format, secrets, etc.
4. **Cleanup**: Kills proxy process

## Output

Tests print detailed output with markers:

- `[PROXY]` - Proxy startup/shutdown
- `[CURL]` - curl command execution
- `[CURL-STDERR]` - Protocol details (H1 vs H2)
- `[CURL-STDOUT]` - Response body
- `[REQUEST]` - Individual request tracking
- `[RESULT]` - Pass/Fail verdict

Example output:
```
=== TEST: HTTP/1.1 Basic Request ===
[PROXY] Starting MITM proxy on port 59920...
[PROXY] ✓ Proxy ready on port 59920
[CURL] Running: curl --proxy http://127.0.0.1:59920 -v -k https://httpbin.org/get
[CURL-STDERR]
* Uses proxy env variable no_proxy == '...'
* Trying 127.0.0.1:59920...
< HTTP/1.1 200 OK
[CURL-STDOUT]
{"method": "GET", "url": "https://httpbin.org/get", ...}
[RESULT] ✅ PASS: HTTP/1.1 request successful
```

## Debugging Failed Tests

When a test fails:

1. **Check MITM startup**: Does proxy start on a port?
2. **Check connectivity**: Can curl reach httpbin?
3. **Check protocol**: Is H1 vs H2 being used?
4. **Review output**: Look at STDERR for protocol errors

Enable verbose output:
```bash
RUST_LOG=debug ./run-e2e-tests.sh e2e_http1_basic
```

## Integration with CI/CD

Example GitHub Actions workflow:
```yaml
- name: E2E Regression Tests
  run: |
    cd scred-http2
    cargo test --test e2e_httpbin --ignored --release -- --nocapture
  timeout-minutes: 10
```

## Regression Detection

These tests catch:

- ✅ MITM proxy failures to start
- ✅ Protocol negotiation issues (H1 vs H2)
- ✅ Connection resets/closes
- ✅ Response redaction not working
- ✅ Keep-alive/connection pooling issues
- ✅ Proxy crashes on edge cases
- ✅ Large payload handling
- ✅ Compression handling

## Requirements

- `curl` installed and in PATH
- Network access to httpbin.org
- Rust toolchain (for cargo)
- ~30 seconds per full test run

## Known Limitations

1. **HTTP/2 Status**: Currently downgrading to HTTP/1.1 (Phase 1)
   - Test `e2e_http2_alpn` accepts either H1 or H2
   - Native H2 support planned for Phase 2

2. **Network Dependent**: Tests require internet access to httpbin.org
   - Can be modified to use local test server if needed

3. **Timing**: Tests have 10s per-request timeout
   - May fail on slow networks
   - Adjustable with `curl -m` parameter

## Files

- `tests/e2e_httpbin.rs` - Main test suite
- `run-e2e-tests.sh` - Convenience runner script
- `E2E_TESTS_README.md` - This file

## Future Enhancements

- [ ] Local httpbin server (don't depend on external service)
- [ ] Performance benchmarking
- [ ] Concurrent request stress testing
- [ ] Custom secret redaction verification
- [ ] Full HTTP/2 multiplexing tests (when native H2 ready)
