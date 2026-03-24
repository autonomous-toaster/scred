# SCRED Integration Test Plan - HTTP/1.1 and HTTP/2

## Scope

Test that SCRED properly redacts secrets across:
1. **HTTP/1.1** - Both CLI and MITM/Proxy
2. **HTTP/2** - MITM with H2 upstream

## HTTP/1.1 Integration Tests

### Test 1: CLI with HTTP Headers
```bash
curl -H "Authorization: Bearer sk_test_abc123xyz" http://httpbin.org/get | \
  scred --redact CRITICAL
```
Expected: Bearer token passed through (CRITICAL selector)

### Test 2: MITM HTTP Proxy
```bash
export http_proxy=http://localhost:8888
curl http://httpbin.org/get \
  -H "X-API-Key: AKIAIOSFODNN7EXAMPLE" \
  -H "Authorization: Bearer token_xyz"
```
Expected: Both headers redacted before reaching upstream

### Test 3: MITM HTTPS Proxy (HTTP/1.1)
```bash
export https_proxy=http://localhost:8888
curl https://httpbin.org/get \
  -H "X-API-Key: AKIAIOSFODNN7EXAMPLE"
```
Expected: Secret redacted in CONNECT tunnel

### Test 4: Reverse Proxy (HTTP/1.1)
```bash
# Start proxy
./target/release/scred-proxy &

# Send request with secret in body
curl http://localhost:9999/post \
  -d '{"password":"AKIAIOSFODNN7EXAMPLE"}'
```
Expected: Password redacted before reaching upstream

### Test 5: Chunked Transfer-Encoding
```bash
# Send chunked request/response with secrets
curl http://localhost:9999/get \
  -H "Transfer-Encoding: chunked" \
  -d "chunk1 AKIAIOSFODNN7EXAMPLE chunk2"
```
Expected: Secret redacted across chunk boundaries

## HTTP/2 Integration Tests

### Test 6: MITM HTTPS with HTTP/2
```bash
export https_proxy=http://localhost:8888

# h2 command-line client (if available)
h2c https://httpbin.org/get \
  -H "X-API-Key: AKIAIOSFODNN7EXAMPLE"
```
Expected: Secret redacted in HTTP/2 headers

### Test 7: curl with HTTP/2
```bash
export https_proxy=http://localhost:8888

curl --http2 https://httpbin.org/get \
  -H "Authorization: Bearer sk_test_abc123"
```
Expected: Secret redacted in HTTP/2 stream

### Test 8: HTTP/2 POST with Body
```bash
export https_proxy=http://localhost:8888

curl --http2 https://httpbin.org/post \
  -d '{"secret":"AKIAIOSFODNN7EXAMPLE","user":"alice"}'
```
Expected: Secret in JSON body redacted

## Test Infrastructure Needed

### 1. Live Testing Against httpbin.org
```bash
#!/bin/bash

# Start MITM
./target/release/scred-mitm &
MITM_PID=$!

# Start Proxy
./target/release/scred-proxy &
PROXY_PID=$!

sleep 2

# Run tests
export http_proxy=http://localhost:8888
export https_proxy=http://localhost:8888

# Test 1: HTTP through MITM
echo "TEST 1: HTTP through MITM"
curl http://httpbin.org/get \
  -H "X-API-Key: AKIAIOSFODNN7EXAMPLE" 2>/dev/null | \
  tee /tmp/test1_response.txt

if grep -q "AKIAIOSFODNN7EXAMPLE" /tmp/test1_response.txt; then
  echo "❌ FAIL: Secret leaked in response"
else
  echo "✅ PASS: Secret redacted"
fi

# Test 2: HTTPS through MITM
echo "TEST 2: HTTPS through MITM"
curl -k https://httpbin.org/get \
  -H "X-API-Key: AKIAIOSFODNN7EXAMPLE" 2>/dev/null | \
  tee /tmp/test2_response.txt

if grep -q "AKIAIOSFODNN7EXAMPLE" /tmp/test2_response.txt; then
  echo "❌ FAIL: Secret leaked in HTTPS response"
else
  echo "✅ PASS: HTTPS secret redacted"
fi

# Test 3: HTTP/1.1 POST through Proxy
echo "TEST 3: HTTP/1.1 POST through Proxy"
curl http://localhost:9999/post \
  -d '{"api_key":"AKIAIOSFODNN7EXAMPLE"}' 2>/dev/null | \
  tee /tmp/test3_response.txt

if grep -q "AKIAIOSFODNN7EXAMPLE" /tmp/test3_response.txt; then
  echo "❌ FAIL: Secret leaked in POST response"
else
  echo "✅ PASS: POST secret redacted"
fi

# Cleanup
kill $MITM_PID $PROXY_PID 2>/dev/null

echo "Integration tests complete"
```

### 2. Character Preservation Validation
```bash
# Test that character count is preserved

# Input with secret
INPUT="Header: AKIAIOSFODNN7EXAMPLE | Body"
INPUT_LEN=${#INPUT}

# Pipe through scred
OUTPUT=$(echo -n "$INPUT" | ./target/release/scred)
OUTPUT_LEN=${#OUTPUT}

if [ $INPUT_LEN -eq $OUTPUT_LEN ]; then
  echo "✅ Character preservation verified"
else
  echo "❌ Character count mismatch: $INPUT_LEN vs $OUTPUT_LEN"
fi
```

### 3. Pattern Verification
```bash
# Test that common patterns are detected and redacted

PATTERNS=(
  "AKIAIOSFODNN7EXAMPLE"           # AWS
  "ghp_1234567890abcdefghijklmnop" # GitHub PAT
  "sk_test_4eC39HqLyjWDarhtT1ZdV7dn" # Stripe
)

for pattern in "${PATTERNS[@]}"; do
  echo "Testing pattern: $pattern"
  
  result=$(echo "Secret: $pattern" | ./target/release/scred)
  
  if echo "$result" | grep -q "$pattern"; then
    echo "❌ Pattern NOT redacted: $pattern"
  else
    echo "✅ Pattern redacted: $pattern"
  fi
done
```

### 4. Selector Support Validation
```bash
# Test that selectors work correctly

# Test 1: CRITICAL only
echo "Testing CRITICAL selector"
result=$(echo "AWS: AKIAIOSFODNN7EXAMPLE | Other: some_other_key" | \
  ./target/release/scred --redact CRITICAL)

# Check if redacted (CRITICAL tier includes AWS)
if ! echo "$result" | grep -q "AKIAIOSFODNN7EXAMPLE"; then
  echo "✅ CRITICAL selector working"
fi

# Test 2: API_KEYS only
echo "Testing API_KEYS selector"
result=$(echo "Bearer: ghp_1234567890abcdefghijklmnop" | \
  ./target/release/scred --redact API_KEYS)

# GitHub PAT is in API_KEYS tier
if ! echo "$result" | grep -q "ghp_"; then
  echo "✅ API_KEYS selector working"
fi
```

## Expected Results

### HTTP/1.1 Tests
- ✅ Secrets in headers redacted
- ✅ Secrets in body redacted
- ✅ Chunked encoding handled
- ✅ Character count preserved
- ✅ All patterns detected

### HTTP/2 Tests
- ✅ Secrets in H2 headers redacted
- ✅ Secrets in H2 body redacted
- ✅ HPACK encoding handled
- ✅ Stream multiplexing works
- ✅ Frame boundaries correct

### Selector Tests
- ✅ CRITICAL tier selects AWS, GitHub, Stripe
- ✅ API_KEYS tier includes more patterns
- ✅ Combinable selectors work
- ✅ DEFAULT selectors applied correctly

## Files to Create/Modify

1. `tests/integration_http11.sh` - HTTP/1.1 tests
2. `tests/integration_http2.sh` - HTTP/2 tests
3. `tests/integration_selectors.sh` - Selector validation
4. `tests/integration_patterns.sh` - Pattern coverage
5. Update `.github/workflows/ci.yml` to run tests

## Timeline

- Create tests: 2-3 hours
- Run against httpbin.org: 1 hour
- Fix any issues: 1-2 hours
- Total: 4-6 hours

## Success Criteria

- ✅ All HTTP/1.1 tests pass
- ✅ All HTTP/2 tests pass
- ✅ Character preservation verified
- ✅ All patterns detected correctly
- ✅ Selectors working as designed
- ✅ No secrets leak in any scenario
- ✅ CI integration complete

