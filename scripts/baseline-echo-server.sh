#!/bin/bash
# Baseline test: Direct HTTP client to echo server (no proxy)

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

cleanup() {
    pkill -f "debug_echo_server.py" 2>/dev/null || true
    sleep 0.5
}

trap cleanup EXIT

echo -e "${YELLOW}=== BASELINE: Direct to Echo Server (No Proxy) ===${NC}"
echo "Goal: Measure echo server throughput"
echo ""

# Start echo server
python3 /tmp/debug_echo_server.py 8888 2>/dev/null &
sleep 1

echo "Running benchmark (300 sequential requests)..."

# Simple benchmark using curl with keepalive
start_ns=$(date +%s%N)
success=0

# Use a Python script to handle HTTP/1.1 properly
python3 << 'PYSCRIPT'
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(("127.0.0.1", 8888))

success = 0
for i in range(300):
    request = "GET / HTTP/1.1\r\nHost: 127.0.0.1:8888\r\nConnection: keep-alive\r\n\r\n"
    sock.sendall(request.encode())
    
    # Read response headers and body
    response = b""
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        response += chunk
        if b"}" in response:
            break
    
    if b"200" in response[:50]:
        success += 1
    
    if (i + 1) % 75 == 0:
        print(f"  {i + 1}/300 (success: {success})")

sock.close()

elapsed = time.time() - start_time
throughput = (success * 500 / 1_000_000) / elapsed
print(f"\nDirect server results:")
print(f"  Success: {success}/300")
print(f"  Throughput: {throughput:.3f} MB/s")
print(f"METRIC throughput_mb_s={throughput:.3f}")
PYSCRIPT

cleanup
