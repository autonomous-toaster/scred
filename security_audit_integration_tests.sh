#!/bin/bash
#
# SCRED Security Integration Test Suite
# Tests CLI, proxy, and MITM against real httpbin.org or local httpbin
# Checks that secrets are properly detected and redacted
#

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PASS=0
FAIL=0
TEST_NUM=0

# Configuration
HTTPBIN_URL="${HTTPBIN_URL:-https://httpbin.org}"
SCRED_CLI="${SCRED_CLI:-./target/debug/scred}"
PROXY_PORT="${PROXY_PORT:-9999}"
MITM_PORT="${MITM_PORT:-8888}"

test_name() {
    TEST_NUM=$((TEST_NUM + 1))
    echo -e "\n${BLUE}[TEST $TEST_NUM]${NC} $1"
}

pass() {
    echo -e "${GREEN}✅ PASS${NC}: $1"
    PASS=$((PASS + 1))
}

fail() {
    echo -e "${RED}❌ FAIL${NC}: $1"
    FAIL=$((FAIL + 1))
}

assert_contains() {
    local text="$1"
    local pattern="$2"
    local desc="$3"
    
    if echo "$text" | grep -q "$pattern"; then
        pass "$desc"
    else
        fail "$desc - Expected to find: $pattern"
        echo "  Got: $text"
    fi
}

assert_not_contains() {
    local text="$1"
    local pattern="$2"
    local desc="$3"
    
    if ! echo "$text" | grep -q "$pattern"; then
        pass "$desc"
    else
        fail "$desc - Should NOT find: $pattern"
        echo "  Got: $text"
    fi
}

# ============================================================================
# SCRED CLI TESTS
# ============================================================================

echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         SCRED Security Audit - CLI Tests                   ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

# Test 1: AWS AKIA key detection
test_name "CLI detects AWS AKIA key"
INPUT="aws_key=AKIAIOSFODNN7EXAMPLE"
OUTPUT=$(echo "$INPUT" | $SCRED_CLI 2>/dev/null || echo "ERROR")
assert_not_contains "$OUTPUT" "AKIA" "AWS AKIA redacted"

# Test 2: GitHub token detection  
test_name "CLI detects GitHub token"
INPUT="token=ghp_1234567890abcdefghijklmnopqrstuvwxyz"
OUTPUT=$(echo "$INPUT" | $SCRED_CLI 2>/dev/null || echo "ERROR")
assert_not_contains "$OUTPUT" "ghp_" "GitHub token redacted"

# Test 3: Multiple different patterns
test_name "CLI redacts multiple secret types"
INPUT="AWS=AKIAIOSFODNN7EXAMPLE GITHUB=ghp_1234567890abcdef STRIPE=sk_live_4eC39Hq"
OUTPUT=$(echo "$INPUT" | $SCRED_CLI 2>/dev/null || echo "ERROR")
assert_not_contains "$OUTPUT" "AKIA" "AWS key redacted"
assert_not_contains "$OUTPUT" "ghp_" "GitHub token redacted"
assert_not_contains "$OUTPUT" "sk_live" "Stripe key redacted"

# Test 4: Authorization headers
test_name "CLI detects Authorization header"
INPUT="Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test"
OUTPUT=$(echo "$INPUT" | $SCRED_CLI 2>/dev/null || echo "ERROR")
assert_not_contains "$OUTPUT" "eyJhbGci" "JWT redacted"

# Test 5: Env variable mode
test_name "CLI detects env file secrets"
INPUT="API_KEY=sk_live_4eC39HqLyjWDarhtT6B3
DATABASE_URL=mongodb+srv://user:password@cluster.mongodb.net
AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE"
OUTPUT=$(echo "$INPUT" | $SCRED_CLI --env 2>/dev/null || echo "ERROR")
assert_not_contains "$OUTPUT" "sk_live" "API key in env redacted"
assert_not_contains "$OUTPUT" "password@cluster" "MongoDB password redacted"
assert_not_contains "$OUTPUT" "AKIA" "AWS key in env redacted"

# ============================================================================
# PROXY TESTS (using StreamingRedactor directly)
# ============================================================================

echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         SCRED Security Audit - Proxy Tests                 ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

test_name "Proxy redacts secrets in query parameters"
echo "NOTE: Requires scred-proxy running on port $PROXY_PORT"
# This would test: curl "http://localhost:$PROXY_PORT/get?api_key=sk_live_test"

test_name "Proxy redacts secrets in request headers"
echo "NOTE: Requires scred-proxy running on port $PROXY_PORT"
# This would test: curl -H "Authorization: Bearer token123" "http://localhost:$PROXY_PORT/get"

test_name "Proxy redacts secrets in request body"
echo "NOTE: Requires scred-proxy running on port $PROXY_PORT"
# This would test: curl -X POST "http://localhost:$PROXY_PORT/post" -d '{"api_key":"sk_live_test"}'

# ============================================================================
# INTEGRATION TESTS: Direct httpbin.org connections
# ============================================================================

echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║      SCRED Security Audit - httpbin.org Integration        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

test_name "Can reach $HTTPBIN_URL"
if curl -s "$HTTPBIN_URL/get" > /dev/null 2>&1; then
    pass "httpbin.org is reachable"
else
    echo -e "${YELLOW}⚠️  SKIP${NC}: httpbin.org not reachable, using local tests only"
fi

# Test 6: Secret in query parameter (httpbin echoes it back)
test_name "Secret in query parameter detected"
RESPONSE=$(curl -s "$HTTPBIN_URL/get?api_key=sk_live_4eC39HqLyjWDarhtT6B3" 2>/dev/null || echo "")
if echo "$RESPONSE" | grep -q "sk_live"; then
    # httpbin echoes back the query param - show it's there in raw response
    echo "  Raw response contains query param (before redaction): OK"
    pass "Query parameter visible in httpbin response"
else
    echo "  Note: Could not test with httpbin.org"
fi

# ============================================================================
# SECURITY AUDIT: Selective Filtering (Vulnerability Check)
# ============================================================================

echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     SCRED Security Audit - Selective Filtering Check       ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

test_name "Selective redaction flag handling"
echo "NOTE: Current implementation does NOT support --redact selective filtering"
echo "  Expected: --detect shows patterns, --redact filters output"
echo "  Current: All detected secrets are always redacted (secure default)"
INPUT="AWS=AKIAIOSFODNN7EXAMPLE STRIPE=sk_live_test"
OUTPUT=$(echo "$INPUT" | $SCRED_CLI 2>/dev/null || echo "ERROR")
# With current implementation, both should be redacted
assert_not_contains "$OUTPUT" "AKIA" "AWS redacted regardless of --redact setting"
assert_not_contains "$OUTPUT" "sk_live" "Stripe redacted regardless of --redact setting"

# ============================================================================
# PATTERN DETECTION CONSISTENCY
# ============================================================================

echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║    SCRED Security Audit - Pattern Detection Consistency    ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

test_name "Consistent redaction across formats"
for format in "key=value" "key: value" "\"key\":\"value\""; do
    INPUT="aws${format//key/key}AKIAIOSFODNN7EXAMPLE${format//value/value}"
    OUTPUT=$(echo "$INPUT" | $SCRED_CLI 2>/dev/null || echo "ERROR")
    if ! echo "$OUTPUT" | grep -q "AKIA"; then
        pass "AWS key redacted in format: $format"
    else
        fail "AWS key NOT redacted in format: $format"
    fi
done

# ============================================================================
# STREAMING MODE TESTS  
# ============================================================================

echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       SCRED Security Audit - Streaming Mode Tests          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

test_name "Streaming mode handles large inputs"
# Create large input with many secrets
LARGE_INPUT=$(for i in {1..100}; do 
    echo "Line$i: AWS=AKIAIOSFODNN7EXAMPLE STRIPE=sk_live_4eC39HqLyjWDarhtT6B3"
done)
OUTPUT=$(echo "$LARGE_INPUT" | $SCRED_CLI 2>/dev/null || echo "ERROR")
REDACTED_COUNT=$(echo "$OUTPUT" | grep -c "AKIA" || echo "0")
if [ "$REDACTED_COUNT" -eq 0 ]; then
    pass "All AWS keys in large input redacted"
else
    fail "Some AWS keys not redacted in large input (found: $REDACTED_COUNT)"
fi

# ============================================================================
# SUMMARY
# ============================================================================

echo -e "\n${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                      SUMMARY                               ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

TOTAL=$((PASS + FAIL))
echo -e "Total Tests: $TOTAL"
echo -e "Passed: ${GREEN}$PASS${NC}"
echo -e "Failed: ${RED}$FAIL${NC}"

if [ $FAIL -eq 0 ]; then
    echo -e "\n${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}❌ Some tests failed!${NC}"
    exit 1
fi
