#!/usr/bin/env python3
"""Test HTTP/1.1 Keep-Alive performance with persistent connection."""

import socket
import time

def read_http_response(sock):
    """Read a complete HTTP response from socket."""
    response = b""
    headers_end = False
    content_length = 0
    
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        response += chunk
        
        # Parse headers if not done yet
        if not headers_end:
            if b"\r\n\r\n" in response:
                headers_end = True
                header_part = response.split(b"\r\n\r\n")[0]
                
                # Extract Content-Length
                for line in header_part.split(b"\r\n"):
                    if line.lower().startswith(b"content-length:"):
                        content_length = int(line.split(b":")[1].strip())
                        break
        
        # Check if we have complete body
        if headers_end and content_length > 0:
            headers_size = response.find(b"\r\n\r\n") + 4
            body_size = len(response) - headers_size
            if body_size >= content_length:
                break
    
    return response

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
User-Agent: Python-Test-v2\r
Connection: keep-alive\r
\r
"""
        
        try:
            sock.sendall(request.encode())
            response = read_http_response(sock)
            
            if response and b"200" in response[:50]:
                success += 1
        except:
            pass
        
        if (i + 1) % 75 == 0:
            print(f"  {i + 1}/{num_requests} (success: {success})")
    
    sock.close()
    
    elapsed = time.time() - start_time
    response_size = 500
    throughput = (success * response_size / 1_000_000) / elapsed
    rps = success / elapsed
    
    print(f"\nResults (HTTP/1.1 Keep-Alive on persistent connection):")
    print(f"  Success: {success}/{num_requests}")
    print(f"  Duration: {elapsed:.3f}s ({int(elapsed*1000)}ms)")
    print(f"  RPS: {rps:.1f}")
    print(f"  Throughput: {throughput:.3f} MB/s")
    print(f"  Baseline (sequential separate connections): 0.029 MB/s")
    print(f"  Improvement: {throughput/0.029:.1f}×")
    
    return throughput

if __name__ == "__main__":
    print("=== PHASE 5: HTTP/1.1 Keep-Alive Test (v2) ===")
    print("Goal: Test with persistent connection")
    print()
    
    result = test_keepalive("127.0.0.1", 9999, 300)
    print(f"\nMETRIC throughput_mb_s={result:.3f}")
