#!/bin/bash
# Test Phase 1 throughput improvement

set -e

echo "=== Phase 1 Throughput Benchmark ==="
echo ""

# Create test data with secrets (Python heredoc approach)
echo "Generating test data (10MB with secrets)..."
python3 > /tmp/test_secrets_10mb.txt << 'PYTHON'
import sys
data = "aws_key=AKIAIOSFODNN7EXAMPLE\ngithub_token=ghp_1234567890abcdefghijklmnopqrstuvwxyz\nregular_data=value\n" * 100000
sys.stdout.buffer.write(data.encode())
PYTHON

echo "Test file created: $(wc -c < /tmp/test_secrets_10mb.txt) bytes"
echo ""

# Test 1: Single run with verbose output
echo "Test 1: Single 10MB run with statistics"
echo "---"
./target/release/scred --text-mode -v < /tmp/test_secrets_10mb.txt > /tmp/output.txt 2>&1
grep -E "Throughput|Bytes" /tmp/output.txt || echo "No stats found"
echo ""

# Test 2: Multiple runs to get average
echo "Test 2: Average throughput over 3 runs"
echo "---"
for i in {1..3}; do
    ./target/release/scred --text-mode -v < /tmp/test_secrets_10mb.txt > /dev/null 2>&1 | grep "Throughput" || true
done
echo ""

# Cleanup
rm -f /tmp/test_secrets_10mb.txt /tmp/output.txt

echo "=== Summary ==="
echo "Expected Phase 1 improvement: +15% (120 → 138 MB/s)"
echo "Target minimum: 125 MB/s"
