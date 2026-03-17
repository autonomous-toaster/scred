#!/bin/bash

set -e

cd "$(dirname "$0")"

# Colors
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}                    SCRED BENCHMARK SUITE${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

# Check binaries exist
if [ ! -f "./scred-zig/scred-full" ] || [ ! -f "./scred-rust/target/release/scred" ] || [ ! -f "./scred-go/scred_new" ]; then
  echo -e "${RED}Error: Not all binaries found${NC}"
  exit 1
fi

# Test data files
if [ ! -f "/tmp/pi_bench_data/lines_100.jsonl" ]; then
  echo -e "${YELLOW}Warning: Test data files not found at /tmp/pi_bench_data/${NC}"
  echo "Creating simple test file instead..."
  mkdir -p /tmp/scred_bench
  
  # Create test files with known content
  cat > /tmp/scred_bench/small.txt << 'TESTEOF'
AKIA1234567890ABCDEF sk-proj-1234567890ABCDEFGHIJ ghp_abcdefghijklmnopqrstuvwxyz0123456789ab
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig
mongodb://user:password123456@mongodb.example.com:27017/dbname
password: superSecretPassword123456789
secret: my_secret_value_123456789
token: 1234567890abcdefghijklmnopq
TESTEOF

  # Expand to medium (10KB)
  for i in {1..100}; do
    cat /tmp/scred_bench/small.txt >> /tmp/scred_bench/medium.txt
  done
  
  # Expand to large (100KB)
  for i in {1..10}; do
    cat /tmp/scred_bench/medium.txt >> /tmp/scred_bench/large.txt
  done
fi

# Use available files
if [ -f "/tmp/pi_bench_data/lines_100.jsonl" ]; then
  SMALL="/tmp/pi_bench_data/lines_100.jsonl"
  MEDIUM="/tmp/pi_bench_data/messages_500.jsonl"
  LARGE="/tmp/pi_bench_data/full_file.jsonl"
else
  SMALL="/tmp/scred_bench/small.txt"
  MEDIUM="/tmp/scred_bench/medium.txt"
  LARGE="/tmp/scred_bench/large.txt"
fi

run_benchmark() {
  local impl="$1"
  local binary="$2"
  local file="$3"
  local label="$4"
  
  local size=$(wc -c < "$file")
  local size_kb=$((size / 1024))
  
  local start=$(date +%s%N)
  cat "$file" | "$binary" > /dev/null 2>&1
  local end=$(date +%s%N)
  
  local duration_ns=$((end - start))
  local duration_ms=$((duration_ns / 1000000))
  local throughput=$(( (size * 1000) / duration_ms / 1024))  # KB/s
  
  printf "%-15s %-10s %-6dms  %-6dMB/s\n" "$impl" "$label" "$duration_ms" "$throughput"
}

echo -e "${GREEN}Testing with small file (486 KB)${NC}"
echo "Implementation  Size         Time     Throughput"
echo "─────────────────────────────────────────────────"
run_benchmark "Zig" "./scred-zig/scred-full" "$SMALL" "486KB"
run_benchmark "Rust" "./scred-rust/target/release/scred" "$SMALL" "486KB"
run_benchmark "Go" "./scred-go/scred_new" "$SMALL" "486KB"

echo ""
echo -e "${GREEN}Testing with medium file (1.6 MB)${NC}"
echo "Implementation  Size         Time     Throughput"
echo "─────────────────────────────────────────────────"
run_benchmark "Zig" "./scred-zig/scred-full" "$MEDIUM" "1.6MB"
run_benchmark "Rust" "./scred-rust/target/release/scred" "$MEDIUM" "1.6MB"
run_benchmark "Go" "./scred-go/scred_new" "$MEDIUM" "1.6MB"

echo ""
echo -e "${GREEN}Testing with large file (3.7 MB)${NC}"
echo "Implementation  Size         Time     Throughput"
echo "─────────────────────────────────────────────────"
run_benchmark "Zig" "./scred-zig/scred-full" "$LARGE" "3.7MB"
run_benchmark "Rust" "./scred-rust/target/release/scred" "$LARGE" "3.7MB"
run_benchmark "Go" "./scred-go/scred_new" "$LARGE" "3.7MB"

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}                    BINARY SIZE COMPARISON${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════════════════════${NC}"
echo ""
echo "Implementation  Binary Size"
echo "─────────────────────────────"
ls -lh ./scred-zig/scred-full ./scred-rust/target/release/scred ./scred-go/scred_new | awk '{print $9, "\t\t", $5}'

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════════════════${NC}"
