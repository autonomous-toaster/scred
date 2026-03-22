#!/usr/bin/env bash
set -e

cd /Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred

# Build
cargo build --release --bin scred 2>&1 | grep Finished

# Detailed timing breakdown
echo "Testing detailed performance..."

# Create a 100MB test file
echo "Generating 100MB test file..." >&2
python3 << 'PYTHON' > /tmp/test_100mb.txt
import sys
# Fast generation: repeat pattern
pattern = ("Lorem ipsum dolor sit amet, consectetur adipiscing elit. " * 10)
pattern = (pattern + "AKIAIOSFODNN7EXAMPLE\n") * 100000
sys.stdout.buffer.write(pattern.encode())
PYTHON

# Time with different approaches
echo ""
echo "=== Test 1: Direct piping (no shell overhead) ===" >&2
time cat /tmp/test_100mb.txt | ./target/release/scred > /dev/null

echo ""
echo "=== File size ===" >&2
ls -lh /tmp/test_100mb.txt | awk '{print $5}'

# Calculate MB/s
size_bytes=$(stat -f%z /tmp/test_100mb.txt)
size_mb=$(echo "scale=1; $size_bytes / 1048576" | bc)
echo "Size: $size_mb MB" >&2

# Full test with timing
echo ""
echo "=== Throughput Calculation ===" >&2
start=$(date +%s%N)
./target/release/scred < /tmp/test_100mb.txt > /dev/null
end=$(date +%s%N)

elapsed_ms=$(( (end - start) / 1000000 ))
throughput=$(echo "scale=1; ($size_mb * 1000) / $elapsed_ms" | bc)

echo "Size: $size_mb MB"
echo "Time: ${elapsed_ms}ms"
echo "Throughput: $throughput MB/s"

rm -f /tmp/test_100mb.txt
