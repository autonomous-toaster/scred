#!/usr/bin/env python3
"""
SCRED Performance Regression Testing Framework

Measures detection and redaction performance across:
- Multiple payload sizes
- Different pattern densities
- Scalar vs SIMD implementations
- Platform variations

Usage:
    python3 perf_regression_test.py [--baseline] [--save-results] [--compare-baseline]
"""

import subprocess
import json
import time
import os
import sys
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import List, Dict, Tuple
import statistics

@dataclass
class BenchResult:
    """Single benchmark result"""
    test_name: str
    payload_size: int  # bytes
    pattern_count: int  # patterns in payload
    density: float     # patterns per KB
    impl: str          # 'scalar' or 'simd'
    latency_ms: float
    throughput_mbps: float
    timestamp: str

class PayloadGenerator:
    """Generate test payloads with varying secret densities"""
    
    SECRETS = [
        "AKIAIOSFODNN7EXAMPLE",
        "ghp_1234567890abcdefghijklmnopqrstuvwxyz",
        "sk_live_1234567890abcdefghijklmnopqrstuvwxyz",
        "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz",
        "opsgenie_key_1234567890abcdefghijklmnopqrst",
    ]
    
    FILLER = "This is normal text without secrets. Just regular content here. "
    
    @staticmethod
    def sparse(size_kb: int) -> str:
        """1 secret per 100KB"""
        pattern = (PayloadGenerator.FILLER * 100).encode()[:1024]
        secrets = [s.encode() for s in PayloadGenerator.SECRETS]
        payload = b''
        secret_idx = 0
        
        while len(payload) < size_kb * 1024:
            payload += pattern
            if len(payload) % (100 * 1024) == 0:
                payload += secrets[secret_idx % len(secrets)]
                secret_idx += 1
        
        return payload[:size_kb * 1024].decode('utf-8', errors='ignore')
    
    @staticmethod
    def medium(size_kb: int) -> str:
        """1 secret per 10KB"""
        pattern = (PayloadGenerator.FILLER * 20).encode()[:1024]
        secrets = [s.encode() for s in PayloadGenerator.SECRETS]
        payload = b''
        secret_idx = 0
        
        while len(payload) < size_kb * 1024:
            payload += pattern
            payload += secrets[secret_idx % len(secrets)]
            secret_idx += 1
        
        return payload[:size_kb * 1024].decode('utf-8', errors='ignore')
    
    @staticmethod
    def dense(size_kb: int) -> str:
        """Multiple secrets per KB"""
        secrets = PayloadGenerator.SECRETS
        payload = ''
        
        while len(payload) < size_kb * 1024:
            for secret in secrets:
                payload += secret + ','
        
        return payload[:size_kb * 1024]

class PerformanceTester:
    """Benchmark detection and redaction performance"""
    
    SCALAR_BUILD = "target/release/scred"
    SIMD_BUILD = "target_nightly/release/scred"
    
    def __init__(self, repo_root: str = "."):
        self.repo_root = Path(repo_root)
        self.results: List[BenchResult] = []
    
    def build(self, impl: str = 'scalar') -> bool:
        """Build specified implementation"""
        if impl == 'scalar':
            cmd = ["cargo", "build", "--release"]
        else:
            cmd = ["cargo", "+nightly", "build", "--release", "--features", "simd-accel"]
        
        result = subprocess.run(cmd, cwd=self.repo_root, capture_output=True)
        return result.returncode == 0
    
    def measure_detection(self, payload: str, impl: str = 'scalar', runs: int = 5) -> Tuple[float, float]:
        """
        Measure detection performance
        Returns: (latency_ms, throughput_mbps)
        """
        # Write test file
        test_file = Path(f"/tmp/test_payload_{impl}.txt")
        test_file.write_text(payload)
        
        binary = self.SCALAR_BUILD if impl == 'scalar' else self.SIMD_BUILD
        
        latencies = []
        for _ in range(runs):
            start = time.perf_counter()
            result = subprocess.run(
                [str(self.repo_root / binary), "--mode", "streaming"],
                stdin=open(test_file),
                capture_output=True,
                timeout=5
            )
            elapsed = (time.perf_counter() - start) * 1000  # ms
            
            if result.returncode == 0:
                latencies.append(elapsed)
        
        if not latencies:
            return 0, 0
        
        avg_latency = statistics.mean(latencies)
        payload_mb = len(payload) / (1024 * 1024)
        throughput = payload_mb / (avg_latency / 1000)  # MB/s
        
        return avg_latency, throughput
    
    def run_benchmark(self, name: str, size_kb: int, density_fn, impl: str = 'scalar') -> BenchResult:
        """Run single benchmark"""
        payload = density_fn(size_kb)
        secret_count = payload.count('AKIA') + payload.count('ghp_') + \
                      payload.count('sk_') + payload.count('sk-') + \
                      payload.count('opsgenie')
        density = secret_count / (size_kb / 1024) if size_kb > 0 else 0
        
        latency_ms, throughput = self.measure_detection(payload, impl)
        
        result = BenchResult(
            test_name=name,
            payload_size=size_kb * 1024,
            pattern_count=secret_count,
            density=density,
            impl=impl,
            latency_ms=latency_ms,
            throughput_mbps=throughput,
            timestamp=time.strftime('%Y-%m-%d %H:%M:%S')
        )
        
        self.results.append(result)
        return result
    
    def run_suite(self) -> None:
        """Run full benchmark suite"""
        test_cases = [
            ("sparse_10kb", 10, PayloadGenerator.sparse),
            ("sparse_100kb", 100, PayloadGenerator.sparse),
            ("sparse_1mb", 1024, PayloadGenerator.sparse),
            ("medium_10kb", 10, PayloadGenerator.medium),
            ("medium_100kb", 100, PayloadGenerator.medium),
            ("medium_1mb", 1024, PayloadGenerator.medium),
            ("dense_10kb", 10, PayloadGenerator.dense),
            ("dense_100kb", 100, PayloadGenerator.dense),
        ]
        
        for name, size, density_fn in test_cases:
            for impl in ['scalar', 'simd']:
                if impl == 'simd' and not self.build('simd'):
                    print(f"⚠️  Skipping {name} (SIMD) - nightly build failed")
                    continue
                
                print(f"Running {name} ({impl})...", end=' ', flush=True)
                result = self.run_benchmark(name, size, density_fn, impl)
                print(f"✓ {result.latency_ms:.2f}ms")
    
    def print_results(self) -> None:
        """Print formatted results"""
        print("\n" + "="*80)
        print("SCRED Performance Regression Test Results")
        print("="*80 + "\n")
        
        # Group by test name
        by_test = {}
        for result in self.results:
            if result.test_name not in by_test:
                by_test[result.test_name] = []
            by_test[result.test_name].append(result)
        
        for test_name in sorted(by_test.keys()):
            print(f"\n{test_name}:")
            print("-" * 80)
            print(f"{'Impl':<10} {'Size':>10} {'Patterns':>10} {'Latency':>12} {'Throughput':>12}")
            print("-" * 80)
            
            for result in sorted(by_test[test_name], key=lambda x: x.impl):
                print(f"{result.impl:<10} {result.payload_size/1024:>9.0f}KB "
                      f"{result.pattern_count:>10d} {result.latency_ms:>11.2f}ms "
                      f"{result.throughput_mbps:>11.2f}MB/s")
        
        # Summary
        print("\n" + "="*80)
        print("Summary Statistics")
        print("="*80)
        
        scalar_results = [r for r in self.results if r.impl == 'scalar']
        simd_results = [r for r in self.results if r.impl == 'simd']
        
        if scalar_results:
            avg_latency = statistics.mean([r.latency_ms for r in scalar_results])
            print(f"Scalar: avg latency {avg_latency:.2f}ms")
        
        if simd_results:
            avg_latency = statistics.mean([r.latency_ms for r in simd_results])
            print(f"SIMD:   avg latency {avg_latency:.2f}ms")
        
        if scalar_results and simd_results:
            improvement = (
                (statistics.mean([r.latency_ms for r in scalar_results]) -
                 statistics.mean([r.latency_ms for r in simd_results])) /
                statistics.mean([r.latency_ms for r in scalar_results]) * 100
            )
            print(f"SIMD Improvement: {improvement:+.1f}%")
    
    def save_results(self, filename: str = "perf_baseline.json") -> None:
        """Save results to JSON for comparison"""
        data = {
            'timestamp': time.strftime('%Y-%m-%d %H:%M:%S'),
            'results': [asdict(r) for r in self.results]
        }
        with open(filename, 'w') as f:
            json.dump(data, f, indent=2)
        print(f"✓ Results saved to {filename}")

def main():
    """Run performance regression tests"""
    import argparse
    
    parser = argparse.ArgumentParser(description='SCRED Performance Regression Testing')
    parser.add_argument('--baseline', action='store_true', help='Establish baseline')
    parser.add_argument('--save-results', action='store_true', help='Save results to file')
    parser.add_argument('--compare-baseline', action='store_true', help='Compare against baseline')
    parser.add_argument('--repo', default='.', help='Repository root')
    
    args = parser.parse_args()
    
    tester = PerformanceTester(args.repo)
    
    print("Building scalar version...")
    if not tester.build('scalar'):
        print("❌ Failed to build scalar version")
        sys.exit(1)
    
    print("Running performance tests...\n")
    tester.run_suite()
    
    tester.print_results()
    
    if args.save_results:
        tester.save_results()

if __name__ == '__main__':
    main()
