#!/usr/bin/env python3
"""
Profile the redaction performance to identify bottlenecks
"""

import subprocess
import time
import sys

def generate_test_data(size_mb, pattern_type="mixed"):
    """Generate test data with secrets"""
    patterns = {
        "aws": "AWS key: AKIAIOSFODNN7EXAMPLE\n",
        "github": "GitHub: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab\n",
        "openai": "OpenAI: sk-proj-abc123def456ghi789jkl012mno\n",
        "postgres": "postgres://user:SecurePassword123@localhost:5432/db\n",
        "normal": "Normal log line at 2024-03-19 10:30:45 with timestamp\n",
        "mixed": "AWS:AKIAIOSFODNN7EXAMPLE GitHub:ghp_abc Postgres:postgres://u:p@h/db\n",
    }
    
    pattern = patterns.get(pattern_type, patterns["mixed"])
    lines_needed = (size_mb * 1024 * 1024) // len(pattern)
    data = pattern * lines_needed
    return data[:size_mb * 1024 * 1024]

def benchmark_profile(size_mb, pattern_type, name):
    """Time redaction and report throughput"""
    print(f"\nTest: {name} ({size_mb}MB, {pattern_type})")
    
    test_data = generate_test_data(size_mb, pattern_type)
    
    start = time.time()
    result = subprocess.run(
        ["./target/release/scred"],
        input=test_data.encode(),
        capture_output=True,
        timeout=30,
        cwd="/Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred"
    )
    elapsed = time.time() - start
    
    if result.returncode != 0:
        print(f"  ❌ Error: {result.stderr.decode()}")
        return 0
    
    output = result.stdout
    throughput = (size_mb) / elapsed if elapsed > 0 else 0
    
    print(f"  Time: {elapsed:.2f}s")
    print(f"  Throughput: {throughput:.1f} MB/s")
    print(f"  Input: {len(test_data) / 1024 / 1024:.1f} MB")
    print(f"  Output: {len(output) / 1024 / 1024:.1f} MB")
    
    return throughput

print("="*60)
print("PHASE 4: Performance Profiling")
print("="*60)

# Test different scenarios
print("\nScenario 1: AWS Keys Only")
t1 = benchmark_profile(25, "aws", "AWS keys")

print("\nScenario 2: GitHub Tokens Only")
t2 = benchmark_profile(25, "github", "GitHub tokens")

print("\nScenario 3: Normal Logs (no secrets)")
t3 = benchmark_profile(25, "normal", "Normal logs")

print("\nScenario 4: Mixed Secrets")
t4 = benchmark_profile(25, "mixed", "Mixed secrets")

print("\nScenario 5: Large File (100MB)")
t5 = benchmark_profile(100, "mixed", "Large file")

print("\n" + "="*60)
print("PROFILE RESULTS")
print("="*60)
print(f"AWS keys:      {t1:.1f} MB/s")
print(f"GitHub tokens: {t2:.1f} MB/s")
print(f"Normal logs:   {t3:.1f} MB/s")
print(f"Mixed secrets: {t4:.1f} MB/s")
print(f"Large file:    {t5:.1f} MB/s")
print(f"Average:       {(t1+t2+t3+t4+t5)/5:.1f} MB/s")
print("="*60)

if (t1+t2+t3+t4+t5)/5 >= 50:
    print("✅ TARGET REACHED: 50+ MB/s")
else:
    deficit = 50 - (t1+t2+t3+t4+t5)/5
    print(f"⚠️  Target deficit: {deficit:.1f} MB/s")
    print(f"   Speedup needed: {50 / ((t1+t2+t3+t4+t5)/5):.1f}x")
