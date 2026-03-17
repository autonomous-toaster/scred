#!/bin/bash

cd "$(dirname "$0")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TOTAL=0
PASSED=0
FAILED=0

echo -e "${BLUE}════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}                    SCRED COMPREHENSIVE TEST SUITE${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

# Test Zig
if [ ! -f "./scred-zig/scred-full" ]; then
  echo -e "${RED}✗ Zig binary not found${NC}"
  exit 1
fi

# Test Rust
if [ ! -f "./scred-rust/target/release/scred" ]; then
  echo -e "${RED}✗ Rust binary not found${NC}"
  exit 1
fi

# Test Go
if [ ! -f "./scred-go/scred_new" ]; then
  echo -e "${RED}✗ Go binary not found${NC}"
  exit 1
fi

skip_header=true
while IFS=',' read -r input expected pattern_type description; do
  if [ "$skip_header" = true ]; then
    skip_header=false
    continue
  fi

  TOTAL=$((TOTAL + 1))
  
  # Test Zig
  zig_result=$(echo -n "$input" | ./scred-zig/scred-full 2>/dev/null || echo "ERROR")
  # Test Rust
  rust_result=$(echo -n "$input" | ./scred-rust/target/release/scred 2>/dev/null || echo "ERROR")
  # Test Go
  go_result=$(echo -n "$input" | ./scred-go/scred_new 2>/dev/null || echo "ERROR")

  all_match=true
  
  if [ "$zig_result" != "$expected" ] || [ "$rust_result" != "$expected" ] || [ "$go_result" != "$expected" ]; then
    all_match=false
    FAILED=$((FAILED + 1))
    
    # Show detailed failure
    echo -e "${RED}FAIL [Test $TOTAL]: $description${NC}"
    echo "  Input:    '$input'"
    echo "  Expected: '$expected'"
    echo "  Zig:      '$zig_result' $([ "$zig_result" = "$expected" ] && echo '✓' || echo '✗')"
    echo "  Rust:     '$rust_result' $([ "$rust_result" = "$expected" ] && echo '✓' || echo '✗')"
    echo "  Go:       '$go_result' $([ "$go_result" = "$expected" ] && echo '✓' || echo '✗')"
    echo ""
  else
    PASSED=$((PASSED + 1))
  fi

  if [ $((TOTAL % 10)) -eq 0 ]; then
    echo -ne "${BLUE}Progress: $TOTAL tests processed${NC}\r"
  fi

done < test_cases.csv

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}                          TEST RESULTS SUMMARY${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "Total Tests:  ${BLUE}$TOTAL${NC}"
echo -e "Passed:       ${GREEN}$PASSED${NC}"
echo -e "Failed:       ${RED}$FAILED${NC}"

if [ $FAILED -eq 0 ]; then
  echo -e "Status:       ${GREEN}✓ ALL TESTS PASSED${NC}"
  exit 0
else
  percentage=$((PASSED * 100 / TOTAL))
  echo -e "Status:       ${RED}✗ $percentage% PASS RATE ($FAILED failures)${NC}"
  exit 1
fi
