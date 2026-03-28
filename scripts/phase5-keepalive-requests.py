#!/usr/bin/env python3
"""Test HTTP/1.1 Keep-Alive performance using requests library."""

import requests
import time
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

def create_session():
    """Create a requests session with connection pooling."""
    session = requests.Session()
    adapter = HTTPAdapter(
        pool_connections=1,
        pool_maxsize=1,
        max_retries=Retry(total=0, backoff_factor=0)
    )
    session.mount('http://', adapter)
    return session

def test_keepalive(host, port, num_requests):
    """Send multiple requests using same session (Keep-Alive)."""
    
    url = f"http://{host}:{port}/"
    session = create_session()
    
    success = 0
    start_time = time.time()
    
    for i in range(num_requests):
        try:
            response = session.get(url, timeout=5)
            if response.status_code == 200:
                success += 1
        except:
            pass
        
        if (i + 1) % 75 == 0:
            print(f"  {i + 1}/{num_requests} (success: {success})")
    
    session.close()
    
    elapsed = time.time() - start_time
    response_size = 500
    throughput = (success * response_size / 1_000_000) / elapsed
    rps = success / elapsed
    
    print(f"\nResults (HTTP/1.1 Keep-Alive via requests.Session):")
    print(f"  Success: {success}/{num_requests}")
    print(f"  Duration: {elapsed:.3f}s ({int(elapsed*1000)}ms)")
    print(f"  RPS: {rps:.1f}")
    print(f"  Throughput: {throughput:.3f} MB/s")
    print(f"  Baseline (sequential): 0.029 MB/s")
    if throughput > 0:
        print(f"  Improvement: {throughput/0.029:.1f}×")
    
    return throughput

if __name__ == "__main__":
    print("=== PHASE 5: HTTP/1.1 Keep-Alive Test (using requests.Session) ===")
    print("Goal: Test persistent connection with requests library")
    print()
    
    result = test_keepalive("127.0.0.1", 9999, 300)
    print(f"\nMETRIC throughput_mb_s={result:.3f}")
