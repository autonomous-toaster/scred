#!/bin/bash
# E2E MITM Proxy Regression Test Suite
# 
# Usage: ./run-e2e-tests.sh [test_name]
# Examples:
#   ./run-e2e-tests.sh                    # Run all tests
#   ./run-e2e-tests.sh e2e_http1_basic   # Run one test
#   ./run-e2e-tests.sh -v                # Verbose output

set -e

REPO_ROOT="/Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred-http2"
TEST_FILTER=${1:-""}
VERBOSE=${VERBOSE:-0}

cd "$REPO_ROOT"

echo "════════════════════════════════════════════════════════════════"
echo "  E2E MITM PROXY REGRESSION TEST SUITE"
echo "════════════════════════════════════════════════════════════════"
echo ""

if [ -n "$TEST_FILTER" ]; then
    if [ "$TEST_FILTER" = "-v" ]; then
        VERBOSE=1
        echo "[INFO] Verbose mode enabled"
        echo ""
        TEST_FILTER=""
    else
        echo "[INFO] Running: $TEST_FILTER"
        echo ""
    fi
fi

# Kill any lingering proxy processes
pkill -f "scred-mitm" || true
sleep 1

# Build first
echo "[BUILD] Compiling tests..."
cargo test --test e2e_httpbin --no-run --release 2>&1 | grep -E "Finished|error" || true

echo ""
echo "[TESTS] Running regression tests..."
echo ""

# Run tests
if [ -n "$TEST_FILTER" ]; then
    cargo test --test e2e_httpbin "$TEST_FILTER" --ignored --release -- --nocapture 2>&1
else
    # Run all tests
    cargo test --test e2e_httpbin --ignored --release -- --nocapture 2>&1 | tee /tmp/e2e-test-results.log
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  TEST COMPLETE"
echo "════════════════════════════════════════════════════════════════"

# Cleanup
pkill -f "scred-mitm" || true
