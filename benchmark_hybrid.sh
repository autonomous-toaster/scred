#!/bin/bash

echo "=== SCRED Hybrid Benchmark: Zig vs Rust Regex ==="
echo ""

# Generate test data with mixed secrets
gen_test_data() {
    size_mb=$1
    count=$((size_mb * 1024))
    
    # Mix of patterns to hit both detectors
    for i in $(seq 1 $count); do
        echo "User: john_doe"
        echo "AWS Key: AKIAIOSFODNN7EXAMPLE"
        echo "GitHub Token: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab"
        echo "Private Key: -----BEGIN RSA PRIVATE KEY-----"
        echo "MIIEpAIBAAKCAQEA2a2rwplBCME7Ydh8r6rw"
        echo "-----END RSA PRIVATE KEY-----"
        echo "URL: https://example.com"
        echo "Email: user@example.com"
        echo ""
    done | head -c $((size_mb * 1024 * 1024))
}

# Test different file sizes
for size in 1 5 10; do
    echo "Testing with ~${size}MB input..."
    test_data=$(gen_test_data $size)
    
    # Zig detector (optimized - 44 patterns)
    start=$(date +%s%N)
    output=$(echo "$test_data" | ./target/release/scred)
    end=$(date +%s%N)
    zig_ms=$(( (end - start) / 1000000 ))
    zig_throughput=$(echo "scale=1; $size * 1000 / $zig_ms" | bc)
    
    echo "  Zig (44 patterns):     ${zig_ms}ms (~${zig_throughput} MB/s)"
    
done

echo ""
echo "✅ Benchmark complete"
