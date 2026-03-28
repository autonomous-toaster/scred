#!/usr/bin/env python3
"""Test HTTP/1.1 Keep-Alive performance with persistent connection."""

import socket
import time
import sys

def test_keepalive(host, port, num_requests):
    """Send multiple requests on a single TCP connection."""
    
    # Create single TCP connection
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect((host, port))
    
    success = 0
    start_time = time.time()
    
    for i in range(num_requests):
        # Send HTTP/1.1 request with Keep-Alive
        request = f"""GET / HTTP/1.1\r
Host: {host}:{port}\r
User-Agent: Python-Test\r
Connection: keep-alive\r
\r
"""
        
        sock.sendall(request.encode())
        
        # Read response
        response = b""
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            response += chunk
            # Look for end of HTTP response (empty line after headers + content)
            if b"\r\n\r\n" in response:
                # Simple heuristic: check if we got the full body
                if b"}" in response:
                    break
        
        if response and b"200" in response[:50]:
            success += 1
        
        if (i + 1) % 75 == 0:
            print(f"  {i + 1}/{num_requests} (success: {success})")
    
    sock.close()
    
    elapsed = time.time() - start_time
    response_size = 500
    throughput = (success * response_size / 1_000_000) / elapsed
    rps = success / elapsed
    
    print(f"\nResults (Keep-Alive on persistent connection):")
    print(f"  Success: {success}/{num_requests}")
    print(f"  Duration: {elapsed:.3f}s ({int(elapsed*1000)}ms)")
    print(f"  RPS: {rps:.1f}")
    print(f"  Throughput: {throughput:.3f} MB/s")
    print(f"  Baseline (old): 0.029 MB/s")
    
    return throughput

if __name__ == "__main__":
    print("=== PHASE 5: HTTP/1.1 Keep-Alive Test ===")
    print("Goal: Test with persistent connection")
    print()
    
    result = test_keepalive("127.0.0.1", 9999, 300)
    print(f"\nMETRIC throughput_mb_s={result:.3f}")
    
    if result > 0.035:
        print("✅ Keep-Alive working! Significant improvement detected.")
    else:
        print("⚠️  Keep-Alive may not be enabled or benefiting.")
