#!/bin/bash
# Comprehensive E2E Test Suite for SCRED HTTP/2
# 
# Runs all test tiers to validate protocol compliance, redaction integrity,
# and system stability.
#
# Usage:
#   ./run-comprehensive-tests.sh [tier]
#   ./run-comprehensive-tests.sh all        # All tiers
#   ./run-comprehensive-tests.sh compliance # Just compliance tests
#   ./run-comprehensive-tests.sh redaction  # Just redaction tests
#   ./run-comprehensive-tests.sh e2e        # Just E2E tests

set -e

REPO_ROOT="/Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred-http2"
TIER=${1:-"all"}

cd "$REPO_ROOT"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo ""
}

run_tests() {
    test_name=$1
    test_bin=$2
    
    echo -e "${YELLOW}[TEST]${NC} Running $test_name..."
    
    if timeout 120 cargo test --test "$test_bin" 2>&1 | tee /tmp/test_output.log; then
        passed=$(grep "test result:" /tmp/test_output.log | grep -oE "[0-9]+ passed" | grep -oE "[0-9]+")
        echo -e "${GREEN}✓ PASS${NC}: $test_name ($passed tests)"
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}: $test_name"
        return 1
    fi
}

print_summary() {
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  TEST SUMMARY${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    
    # Count all tests
    total_tests=$(cargo test --test h2_compliance --test redaction_isolation --test e2e_httpbin -- --list 2>&1 | grep "test " | wc -l)
    
    echo ""
    echo -e "Tiers Tested:"
    echo "  • ${GREEN}Tier 1: Protocol Compliance${NC} (RFC 7540, RFC 7541, RFC 9113)"
    echo "  • ${GREEN}Tier 2: Redaction Integrity${NC} (Per-stream isolation, no cross-leakage)"
    echo "  • ${GREEN}Tier 3: E2E Integration${NC} (MITM proxy, real HTTP/2 traffic)"
    echo ""
    echo -e "Total Tests: ${total_tests}"
    echo ""
    echo -e "${GREEN}All tiers completed successfully!${NC}"
}

case "$TIER" in
    compliance|tier1)
        print_header "TIER 1: Protocol Compliance Tests (RFC Coverage)"
        run_tests "H2 Frame Validation" "h2_compliance"
        ;;
    
    redaction|tier2)
        print_header "TIER 2: Redaction Isolation Tests"
        run_tests "Stream Isolation & Redaction" "redaction_isolation"
        ;;
    
    e2e|tier3)
        print_header "TIER 3: E2E Integration Tests"
        run_tests "MITM Proxy Regression Tests" "e2e_httpbin"
        ;;
    
    all|*)
        echo ""
        echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
        echo -e "${BLUE}  COMPREHENSIVE E2E TEST SUITE FOR SCRED HTTP/2${NC}"
        echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
        
        failed=0
        
        print_header "TIER 1: Protocol Compliance Tests (RFC Coverage)"
        run_tests "H2 Frame Validation" "h2_compliance" || ((failed++))
        
        print_header "TIER 2: Redaction Isolation Tests"
        run_tests "Stream Isolation & Redaction" "redaction_isolation" || ((failed++))
        
        print_header "TIER 3: E2E Integration Tests"
        run_tests "MITM Proxy Regression Tests" "e2e_httpbin" || ((failed++))
        
        print_summary
        
        if [ $failed -gt 0 ]; then
            echo -e "${RED}FAILED: $failed tier(s) had failures${NC}"
            exit 1
        fi
        ;;
esac

echo ""
echo -e "${GREEN}✓ All tests passed!${NC}"
echo ""
