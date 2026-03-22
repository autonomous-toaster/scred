#!/usr/bin/env python3
"""
Benchmark: Zig analyzer vs current Rust regex approach
Measures content analysis and pattern selection performance
"""

import subprocess
import time
import tempfile
import os

def generate_test_cases():
    """Generate diverse test cases"""
    return {
        "jwt": ("Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U\n" * 1000),
        "aws_keys": ("AWS Key: AKIAIOSFODNN7EXAMPLE\n" * 1000),
        "postgres": ("postgres://user:SecurePassword123@localhost:5432/db\n" * 1000),
        "http": ("GET /api HTTP/1.1\r\nAuthorization: Bearer token123\r\nX-API-Key: sk_test_abc123\r\n" * 100),
        "json": ('{"api_key": "sk-proj-abc", "secret": "ghp_1234567890", "tokens": [1,2,3]}\n' * 1000),
        "mixed": (
            "AWS: AKIAIOSFODNN7EXAMPLE\n"
            "GitHub: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab\n"
            "OpenAI: sk-proj-abc123def456ghi789jkl012mno\n"
            "Postgres: postgres://user:pass@host/db\n"
        ) * 250,
    }

def measure_throughput(data, name="test"):
    """Measure CLI throughput on data"""
    start = time.time()
    result = subprocess.run(
        ["./target/release/scred"],
        input=data.encode() if isinstance(data, str) else data,
        capture_output=True,
        timeout=30,
    )
    elapsed = time.time() - start
    
    if result.returncode != 0:
        return 0, "Error"
    
    size_mb = len(data) / 1024 / 1024 if isinstance(data, str) else len(data) / 1024 / 1024
    throughput = size_mb / elapsed if elapsed > 0 else 0
    
    # Extract stats from stderr
    stats = result.stderr.decode() if result.stderr else ""
    return throughput, stats.split('\n')[0] if stats else "OK"

def main():
    print("=" * 70)
    print("SCRED: Zig Analyzer Benchmark")
    print("=" * 70)
    
    test_cases = generate_test_cases()
    
    print("\n📊 THROUGHPUT MEASUREMENTS (Current Rust Regex)")
    print("-" * 70)
    
    results = {}
    for name, data in test_cases.items():
        size_mb = len(data) / 1024 / 1024
        print(f"  {name:15} ({size_mb:.1f}MB)...", end=" ", flush=True)
        
        throughput, info = measure_throughput(data)
        results[name] = throughput
        
        print(f"✓ {throughput:.1f} MB/s")
    
    print("\n" + "=" * 70)
    print("ANALYSIS")
    print("=" * 70)
    
    avg_throughput = sum(results.values()) / len(results)
    print(f"Average throughput: {avg_throughput:.1f} MB/s")
    print(f"Slowest: {min(results.items(), key=lambda x: x[1])[0]} ({min(results.values()):.1f} MB/s)")
    print(f"Fastest: {max(results.items(), key=lambda x: x[1])[0]} ({max(results.values()):.1f} MB/s)")
    
    print("\n💡 EXPECTED IMPROVEMENTS WITH ZIG")
    print("-" * 70)
    print("Optimization                  | Factor | Expected")
    print("-" * 70)
    
    # Content analysis improvement
    print("1. Smart pattern selection    | 10x    | Reduce 198→15 patterns")
    print("2. PCRE2 vs Rust regex        | 2.5x   | Faster compilation/matching")
    print("3. Combined (realistic)       | 6-10x  | Account for cache hits/overhead")
    
    print("\n" + "=" * 70)
    print("EXPECTED RESULTS WITH ZIG")
    print("-" * 70)
    
    for name, current in results.items():
        expected_min = current * 6
        expected_max = current * 10
        print(f"  {name:15}: {current:.1f} MB/s → {expected_min:.1f}-{expected_max:.1f} MB/s")
    
    print(f"\nAverage expected: {avg_throughput * 6:.1f} - {avg_throughput * 10:.1f} MB/s")
    
    print("\n" + "=" * 70)
    print("NEXT STEPS")
    print("=" * 70)
    print("1. Implement Zig FFI integration in main.rs")
    print("2. Add --zig flag to toggle between Rust and Zig implementations")
    print("3. Re-run benchmark with Zig analyzer enabled")
    print("4. Measure actual improvement vs theoretical")
    print("=" * 70)

if __name__ == "__main__":
    main()
