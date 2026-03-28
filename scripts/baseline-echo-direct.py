#!/usr/bin/env python3
"""Direct baseline: HTTP client to echo server with Keep-Alive."""

import socket
import time
import sys

def test_direct(host, port, num_requests):
    """Test direct HTTP/1.1 with Keep-Alive."""
    print(f"Connecting to {host}:{port}...")
    
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect((host, port))
    
    success = 0
    start = time.time()
    
    for i in range(num_requests):
        # Send HTTP/1.1 request with Keep-Alive
        request = f"GET / HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: keep-alive\r\n\r\n"
        sock.sendall(request.encode())
        
        # Read response
        response = b""
        while True:
            try:
                chunk = sock.recv(4096)
                if not chunk:
                    break
                response += chunk
                # Look for end of response (we got the body)
                if b"}" in response and len(response) > 400:
                    break
            except:
                break
        
        if b"200" in response[:50]:
            success += 1
        
        if (i + 1) % 75 == 0:
            print(f"  {i + 1}/{num_requests} (success: {success})")
    
    sock.close()
    
    elapsed = time.time() - start
    throughput = (success * 500 / 1_000_000) / elapsed
    rps = success / elapsed
    
    print(f"\nResults:")
    print(f"  Success: {success}/{num_requests}")
    print(f"  Duration: {elapsed:.3f}s")
    print(f"  RPS: {rps:.1f}")
    print(f"  Throughput: {throughput:.3f} MB/s")
    
    return throughput

if __name__ == "__main__":
    print("=== BASELINE: Direct HTTP/1.1 with Keep-Alive ===")
    print()
    
    result = test_direct("127.0.0.1", 8888, 300)
    print(f"\nMETRIC throughput_mb_s={result:.3f}")
