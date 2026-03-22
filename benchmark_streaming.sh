#!/usr/bin/env bash
set -e

cd /Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred

# Build release binary first
cargo build --release --bin scred 2>&1 | grep Finished

SCRED_BIN="./target/release/scred"

# Generate test data more efficiently
generate_test_data() {
    local size_mb=$1
    local pattern=$2
    
    python3 << PYTHON
size_bytes = $size_mb * 1024 * 1024
pattern = "$pattern"

# Fast: write in 1MB chunks
chunk = ("Lorem ipsum dolor sit amet, consectetur adipiscing elit. " * 1000) + pattern + "\n"
chunk = (chunk * ((size_bytes // len(chunk)) + 2))[:size_bytes]

import sys
sys.stdout.buffer.write(chunk.encode())
PYTHON
}

benchmark_file() {
    local size_mb=$1
    local pattern=$2
    local name=$3
    
    test_file="/tmp/test_${size_mb}mb_${name}.txt"
    
    # Generate and time in one pass
    echo "Generating $size_mb MB..." >&2
    generate_test_data $size_mb "$pattern" > "$test_file"
    
    echo "Processing..." >&2
    start=$(date +%s%N)
    $SCRED_BIN < "$test_file" > /dev/null 2>&1
    end=$(date +%s%N)
    
    elapsed_ms=$(( (end - start) / 1000000 ))
    if [ $elapsed_ms -lt 1 ]; then elapsed_ms=1; fi
    
    # MB/s = (size_MB * 1000) / elapsed_ms
    throughput=$(echo "scale=1; ($size_mb * 1000) / $elapsed_ms" | bc 2>/dev/null || echo "0")
    
    echo "  $size_mb MB in ${elapsed_ms}ms = $throughput MB/s" >&2
    
    rm -f "$test_file"
    echo "$throughput"
}

echo "====== SCRED Streaming Throughput Benchmark ======"
echo ""

# Quick tests with smaller sizes for fast results
t1=$(benchmark_file 20 "AKIAIOSFODNN7EXAMPLE" "aws")
t2=$(benchmark_file 30 "ghp_abcdefghijklmnopqrstuvwxyz0123456789ab" "github")
t3=$(benchmark_file 40 "sk-proj-abc123def456" "openai")

avg=$(echo "scale=1; ($t1 + $t2 + $t3) / 3" | bc 2>/dev/null || echo "0")

echo ""
echo "Test 1 (20 MB AWS):  $t1 MB/s"
echo "Test 2 (30 MB GitHub): $t2 MB/s"
echo "Test 3 (40 MB OpenAI): $t3 MB/s"
echo "Average: $avg MB/s"
echo ""

# Check result
if (( $(echo "$avg >= 50" | bc -l) )); then
    echo "✅ PASSED: $avg MB/s >= 50 MB/s"
    echo "$avg"
else
    echo "⚠️  Throughput: $avg MB/s (target: 50 MB/s)"
    echo "$avg"
fi
