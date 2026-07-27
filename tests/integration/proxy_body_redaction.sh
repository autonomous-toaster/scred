#!/usr/bin/env bash
# Integration test: verify scred-proxy redacts secrets in HTTP bodies
# This script is called by `just test-integration-proxy` after the
# Justfile has set up httpbin and scred-proxy.
#
# Usage: PROXY_PORT=9997 tests/integration/proxy_body_redaction.sh
set -euo pipefail

PROXY_PORT="${PROXY_PORT:-9997}"
BASE="http://localhost:${PROXY_PORT}"
PASS=0
FAIL=0

echo "=== Proxy Body Redaction Tests ==="
echo ""

# Test 1: POST with AWS key
echo "--- Test 1: POST with AWS access key ---"
SECRET="AKIAIOSFODNN7EXAMPLE"
RESPONSE=$(curl -sf -X POST "${BASE}/anything" \
    -H "Content-Type: application/json" \
    -d "{\"key\": \"${SECRET}\"}" 2>&1) || {
    echo "FAIL: curl request failed"
    FAIL=$((FAIL+1))
    echo "$RESPONSE"
}
if echo "$RESPONSE" | grep -q "AKIAIOSFODNN7EXAMPLE"; then
    echo "FAIL: AWS key was NOT redacted"
    FAIL=$((FAIL+1))
else
    echo "PASS: AWS key redacted from request body"
    PASS=$((PASS+1))
fi

# Test 2: POST with JWT
echo ""
echo "--- Test 2: POST with JWT token ---"
JWT="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U"
RESPONSE2=$(curl -sf -X POST "${BASE}/anything" \
    -H "Content-Type: application/json" \
    -d "{\"token\": \"${JWT}\"}" 2>&1) || {
    echo "FAIL: curl request failed"
    FAIL=$((FAIL+1))
}
if echo "$RESPONSE2" | grep -q "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"; then
    echo "FAIL: JWT was NOT redacted"
    FAIL=$((FAIL+1))
else
    echo "PASS: JWT redacted from request body"
    PASS=$((PASS+1))
fi

# Test 3: GET without secrets (passthrough)
echo ""
echo "--- Test 3: GET without secrets (passthrough) ---"
RESPONSE3=$(curl -sf "${BASE}/get" 2>&1) || {
    echo "FAIL: GET request failed"
    FAIL=$((FAIL+1))
}
if echo "$RESPONSE3" | grep -q "httpbin"; then
    echo "PASS: Normal request passes through correctly"
    PASS=$((PASS+1))
else
    echo "FAIL: Normal request was modified"
    FAIL=$((FAIL+1))
fi

# Test 4: POST with GitHub token
echo ""
echo "--- Test 4: POST with GitHub token ---"
GH_TOKEN="ghp_abcdefghijklmnopqrstuvwxyz0123456789ab"
RESPONSE4=$(curl -sf -X POST "${BASE}/anything" \
    -H "Content-Type: application/json" \
    -d "{\"token\": \"${GH_TOKEN}\"}" 2>&1) || {
    echo "FAIL: curl request failed"
    FAIL=$((FAIL+1))
}
if echo "$RESPONSE4" | grep -q "ghp_abcdefghijklmnopqrstuvwxyz0123456789ab"; then
    echo "FAIL: GitHub token was NOT redacted"
    FAIL=$((FAIL+1))
else
    echo "PASS: GitHub token redacted from request body"
    PASS=$((PASS+1))
fi

# Summary
echo ""
echo "=== Results ==="
echo "Passed: ${PASS}"
echo "Failed: ${FAIL}"
if [ "$FAIL" -gt 0 ]; then
    echo "OVERALL: FAIL"
    exit 1
else
    echo "OVERALL: PASS"
fi
